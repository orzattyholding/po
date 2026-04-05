//! Cryptographic handshake protocol for PO connections.
//!
//! Implements the 3-way handshake:
//! 1. **Initiator → Responder**: `HandshakeInit` (Ed25519 pubkey + X25519 ephemeral + signature)
//! 2. **Responder → Initiator**: `HandshakeReply` (Ed25519 pubkey + X25519 ephemeral + signature)
//! 3. **Initiator → Responder**: `HandshakeComplete` (encrypted confirmation with session key)
//!
//! After step 3, both sides have the same ChaCha20-Poly1305 session key.

use po_crypto::aead::SessionCipher;
use po_crypto::exchange::EphemeralKeypair;
use po_crypto::identity::Identity;
use po_transport::traits::AsyncFrameTransport;
use po_wire::{FrameHeader, FrameType};

use crate::framer::{Framer, FramerError};
use crate::message::{HandshakeComplete, HandshakeInit, HandshakeReply};

use ed25519_dalek::{Signature, VerifyingKey};
use std::time::{SystemTime, UNIX_EPOCH};

/// Result of a successful handshake.
pub struct HandshakeResult {
    /// The session cipher for encrypting/decrypting frames.
    pub cipher: SessionCipher,
    /// The peer's verified Ed25519 public key.
    pub peer_pubkey: [u8; 32],
    /// The peer's NodeId (SHA-256 of their pubkey).
    pub peer_node_id: po_crypto::identity::NodeId,
}

/// Perform the handshake as the initiator (client side).
pub async fn perform_handshake_initiator(
    identity: &Identity,
    transport: &mut dyn AsyncFrameTransport,
    framer: &mut Framer,
) -> Result<HandshakeResult, HandshakeError> {
    // Generate ephemeral X25519 keypair
    let ephemeral = EphemeralKeypair::generate();
    let our_eph_pub = ephemeral.public_bytes();

    // Build signed data: [version(1) || x25519_ephemeral(32) || timestamp(8)]
    let timestamp = now_millis();
    let mut sign_data = Vec::with_capacity(41);
    sign_data.push(1u8); // version
    sign_data.extend_from_slice(&our_eph_pub);
    sign_data.extend_from_slice(&timestamp.to_le_bytes());

    let signature = identity.sign(&sign_data);

    // Send HandshakeInit
    let init = HandshakeInit {
        version: 1,
        ed25519_pubkey: identity.public_key_bytes(),
        x25519_ephemeral: our_eph_pub,
        timestamp,
        signature: signature.to_bytes().to_vec(),
    };
    let payload =
        bincode::serialize(&init).map_err(|e| HandshakeError::Serialization(e.to_string()))?;
    let header = FrameHeader {
        frame_type: FrameType::HandshakeInit,
        flags: po_wire::FrameFlags::default(),
        channel_id: 0,
        stream_id: 0,
        payload_len: payload.len() as u64,
    };
    framer
        .write_frame(transport, &header, &payload)
        .await
        .map_err(HandshakeError::Framer)?;

    // Wait for HandshakeReply
    let (reply_header, reply_payload) = framer
        .read_frame(transport)
        .await
        .map_err(HandshakeError::Framer)?
        .ok_or(HandshakeError::ConnectionClosed)?;

    if reply_header.frame_type != FrameType::HandshakeReply {
        return Err(HandshakeError::UnexpectedFrame(reply_header.frame_type));
    }

    let reply: HandshakeReply = bincode::deserialize(&reply_payload)
        .map_err(|e| HandshakeError::Serialization(e.to_string()))?;

    // Verify responder's signature over [initiator_x25519_pub || responder_x25519_pub]
    let peer_verifying =
        VerifyingKey::from_bytes(&reply.ed25519_pubkey).map_err(|_| HandshakeError::InvalidKey)?;
    let mut verify_data = Vec::with_capacity(64);
    verify_data.extend_from_slice(&our_eph_pub);
    verify_data.extend_from_slice(&reply.x25519_ephemeral);

    let peer_sig = Signature::from_bytes(
        reply
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| HandshakeError::InvalidSignature)?,
    );
    if !Identity::verify(&peer_verifying, &verify_data, &peer_sig) {
        return Err(HandshakeError::InvalidSignature);
    }

    // Derive session key
    let context = build_session_context(&identity.public_key_bytes(), &reply.ed25519_pubkey);
    let session_key = ephemeral
        .derive_session_key(&reply.x25519_ephemeral, &context)
        .map_err(|e| HandshakeError::KeyDerivation(e.to_string()))?;

    // Send HandshakeComplete with encrypted confirmation
    let mut cipher = SessionCipher::new(session_key.as_bytes());
    let confirmation = cipher
        .encrypt(b"PO_READY", b"handshake-complete")
        .map_err(|e| HandshakeError::Encryption(e.to_string()))?;

    let complete = HandshakeComplete { confirmation };
    let complete_payload =
        bincode::serialize(&complete).map_err(|e| HandshakeError::Serialization(e.to_string()))?;
    let complete_header = FrameHeader {
        frame_type: FrameType::HandshakeComplete,
        flags: po_wire::FrameFlags::default(),
        channel_id: 0,
        stream_id: 0,
        payload_len: complete_payload.len() as u64,
    };
    framer
        .write_frame(transport, &complete_header, &complete_payload)
        .await
        .map_err(HandshakeError::Framer)?;

    let peer_node_id = po_crypto::identity::NodeId::from_public_key(&peer_verifying);

    Ok(HandshakeResult {
        cipher,
        peer_pubkey: reply.ed25519_pubkey,
        peer_node_id,
    })
}

/// Perform the handshake as the responder (server side).
pub async fn perform_handshake_responder(
    identity: &Identity,
    transport: &mut dyn AsyncFrameTransport,
    framer: &mut Framer,
) -> Result<HandshakeResult, HandshakeError> {
    // Wait for HandshakeInit
    let (init_header, init_payload) = framer
        .read_frame(transport)
        .await
        .map_err(HandshakeError::Framer)?
        .ok_or(HandshakeError::ConnectionClosed)?;

    if init_header.frame_type != FrameType::HandshakeInit {
        return Err(HandshakeError::UnexpectedFrame(init_header.frame_type));
    }

    let init: HandshakeInit = bincode::deserialize(&init_payload)
        .map_err(|e| HandshakeError::Serialization(e.to_string()))?;

    if init.version != 1 {
        return Err(HandshakeError::UnsupportedVersion(init.version));
    }

    // Verify initiator's signature over [version || x25519_ephemeral || timestamp]
    let peer_verifying =
        VerifyingKey::from_bytes(&init.ed25519_pubkey).map_err(|_| HandshakeError::InvalidKey)?;
    let mut verify_data = Vec::with_capacity(41);
    verify_data.push(init.version);
    verify_data.extend_from_slice(&init.x25519_ephemeral);
    verify_data.extend_from_slice(&init.timestamp.to_le_bytes());

    let peer_sig = Signature::from_bytes(
        init.signature
            .as_slice()
            .try_into()
            .map_err(|_| HandshakeError::InvalidSignature)?,
    );
    if !Identity::verify(&peer_verifying, &verify_data, &peer_sig) {
        return Err(HandshakeError::InvalidSignature);
    }

    // Timestamp freshness check (allow 30 seconds drift)
    let now = now_millis();
    let drift = now.abs_diff(init.timestamp);
    if drift > 30_000 {
        return Err(HandshakeError::TimestampExpired);
    }

    // Generate our ephemeral X25519 keypair
    let ephemeral = EphemeralKeypair::generate();
    let our_eph_pub = ephemeral.public_bytes();

    // Sign [initiator_x25519_pub || our_x25519_pub]
    let mut sign_data = Vec::with_capacity(64);
    sign_data.extend_from_slice(&init.x25519_ephemeral);
    sign_data.extend_from_slice(&our_eph_pub);
    let signature = identity.sign(&sign_data);

    // Send HandshakeReply
    let reply = HandshakeReply {
        ed25519_pubkey: identity.public_key_bytes(),
        x25519_ephemeral: our_eph_pub,
        signature: signature.to_bytes().to_vec(),
    };
    let payload =
        bincode::serialize(&reply).map_err(|e| HandshakeError::Serialization(e.to_string()))?;
    let header = FrameHeader {
        frame_type: FrameType::HandshakeReply,
        flags: po_wire::FrameFlags::default(),
        channel_id: 0,
        stream_id: 0,
        payload_len: payload.len() as u64,
    };
    framer
        .write_frame(transport, &header, &payload)
        .await
        .map_err(HandshakeError::Framer)?;

    // Derive session key
    let context = build_session_context(&init.ed25519_pubkey, &identity.public_key_bytes());
    let session_key = ephemeral
        .derive_session_key(&init.x25519_ephemeral, &context)
        .map_err(|e| HandshakeError::KeyDerivation(e.to_string()))?;
    let cipher = SessionCipher::new(session_key.as_bytes());

    // Wait for HandshakeComplete
    let (complete_header, complete_payload) = framer
        .read_frame(transport)
        .await
        .map_err(HandshakeError::Framer)?
        .ok_or(HandshakeError::ConnectionClosed)?;

    if complete_header.frame_type != FrameType::HandshakeComplete {
        return Err(HandshakeError::UnexpectedFrame(complete_header.frame_type));
    }

    let complete: HandshakeComplete = bincode::deserialize(&complete_payload)
        .map_err(|e| HandshakeError::Serialization(e.to_string()))?;

    // Decrypt and verify confirmation
    let decrypted = cipher
        .decrypt(&complete.confirmation, b"handshake-complete")
        .map_err(|_| HandshakeError::ConfirmationFailed)?;

    if decrypted != b"PO_READY" {
        return Err(HandshakeError::ConfirmationFailed);
    }

    let peer_node_id = po_crypto::identity::NodeId::from_public_key(&peer_verifying);

    Ok(HandshakeResult {
        cipher,
        peer_pubkey: init.ed25519_pubkey,
        peer_node_id,
    })
}

/// Build the HKDF context: sorted concatenation of both pubkeys.
/// Sorting ensures both sides derive the same key regardless of who initiated.
fn build_session_context(initiator_pubkey: &[u8; 32], responder_pubkey: &[u8; 32]) -> Vec<u8> {
    let mut ctx = Vec::with_capacity(64 + 10);
    ctx.extend_from_slice(b"po-v1-");
    // Always put initiator first for deterministic derivation
    ctx.extend_from_slice(initiator_pubkey);
    ctx.extend_from_slice(responder_pubkey);
    ctx
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Handshake errors.
#[derive(Debug)]
pub enum HandshakeError {
    Framer(FramerError),
    Serialization(String),
    InvalidSignature,
    InvalidKey,
    UnsupportedVersion(u8),
    TimestampExpired,
    KeyDerivation(String),
    Encryption(String),
    ConfirmationFailed,
    ConnectionClosed,
    UnexpectedFrame(FrameType),
}

impl std::fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Framer(e) => write!(f, "framer: {e}"),
            Self::Serialization(e) => write!(f, "serialization: {e}"),
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::InvalidKey => write!(f, "invalid public key"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported protocol version: {v}"),
            Self::TimestampExpired => write!(f, "handshake timestamp expired"),
            Self::KeyDerivation(e) => write!(f, "key derivation: {e}"),
            Self::Encryption(e) => write!(f, "encryption: {e}"),
            Self::ConfirmationFailed => write!(f, "handshake confirmation failed"),
            Self::ConnectionClosed => write!(f, "connection closed during handshake"),
            Self::UnexpectedFrame(t) => write!(f, "unexpected frame type: {t}"),
        }
    }
}

impl std::error::Error for HandshakeError {}
