//! Protocol messages serialized with bincode.
//!
//! These are the structured payloads carried inside PO frames. The frame
//! header tells you the type; these structs define the payload format.

use serde::{Deserialize, Serialize};

/// Handshake initiation message (sent by initiator).
///
/// Carried in a `FrameType::HandshakeInit` frame.
#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeInit {
    /// Protocol version (currently 1).
    pub version: u8,
    /// The initiator's Ed25519 public key (32 bytes).
    pub ed25519_pubkey: [u8; 32],
    /// Ephemeral X25519 public key for this session (32 bytes).
    pub x25519_ephemeral: [u8; 32],
    /// Timestamp (Unix millis) — prevents replay attacks.
    pub timestamp: u64,
    /// Ed25519 signature over `[version || x25519_ephemeral || timestamp]`.
    pub signature: Vec<u8>,
}

/// Handshake reply message (sent by responder).
///
/// Carried in a `FrameType::HandshakeReply` frame.
#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeReply {
    /// The responder's Ed25519 public key (32 bytes).
    pub ed25519_pubkey: [u8; 32],
    /// Ephemeral X25519 public key for this session (32 bytes).
    pub x25519_ephemeral: [u8; 32],
    /// Ed25519 signature over `[initiator_x25519_pub || responder_x25519_pub]`.
    pub signature: Vec<u8>,
}

/// Handshake completion confirmation.
///
/// Carried in a `FrameType::HandshakeComplete` frame.
/// The payload is encrypted with the newly derived session key.
#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeComplete {
    /// Confirmation token: "PO_READY" encrypted with the session key.
    pub confirmation: Vec<u8>,
}

/// File transfer metadata.
///
/// Carried in a `FrameType::FileHeader` frame.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileHeader {
    /// Original filename.
    pub name: String,
    /// Total file size in bytes.
    pub size: u64,
    /// SHA-256 hash of the complete file (for integrity verification).
    pub sha256: [u8; 32],
    /// Chunk size used for `FileChunk` frames.
    pub chunk_size: u32,
}

/// Generic protocol message enum for easy matching.
#[derive(Debug, Serialize, Deserialize)]
pub enum ProtocolMessage {
    HandshakeInit(HandshakeInit),
    HandshakeReply(HandshakeReply),
    HandshakeComplete(HandshakeComplete),
    FileHeader(FileHeader),
    Data(Vec<u8>),
}
