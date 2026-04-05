//! Connection state machine for PO sessions.

use crate::framer::Framer;
use crate::handshake::{self, HandshakeError};
use po_crypto::aead::SessionCipher;
use po_crypto::identity::{Identity, NodeId};
use po_transport::traits::AsyncFrameTransport;
use po_wire::{FrameHeader, FrameType};

/// The lifecycle state of a PO connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Connection established at transport level, no handshake yet.
    New,
    /// Handshake in progress.
    Handshaking,
    /// Handshake complete, encrypted session active.
    Established,
    /// Graceful close initiated.
    Closing,
    /// Connection fully closed.
    Closed,
}

/// A fully managed PO session over any transport.
///
/// Handles the handshake, encryption, and frame IO.
pub struct Session {
    state: SessionState,
    framer: Framer,
    cipher: Option<SessionCipher>,
    identity: Identity,
    peer_node_id: Option<NodeId>,
    peer_pubkey: Option<[u8; 32]>,
}

impl Session {
    /// Create a new session with the given identity.
    pub fn new(identity: Identity) -> Self {
        Self {
            state: SessionState::New,
            framer: Framer::new(),
            cipher: None,
            identity,
            peer_node_id: None,
            peer_pubkey: None,
        }
    }

    /// Get the current session state.
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Get our own node ID.
    pub fn node_id(&self) -> &NodeId {
        self.identity.node_id()
    }

    /// Get the peer's node ID (available after handshake).
    pub fn peer_node_id(&self) -> Option<&NodeId> {
        self.peer_node_id.as_ref()
    }

    /// Perform the handshake as the initiator (client).
    pub async fn handshake_initiator(
        &mut self,
        transport: &mut dyn AsyncFrameTransport,
    ) -> Result<(), HandshakeError> {
        self.state = SessionState::Handshaking;

        let result =
            handshake::perform_handshake_initiator(&self.identity, transport, &mut self.framer)
                .await?;

        self.cipher = Some(result.cipher);
        self.peer_pubkey = Some(result.peer_pubkey);
        self.peer_node_id = Some(result.peer_node_id);
        self.state = SessionState::Established;

        Ok(())
    }

    /// Perform the handshake as the responder (server).
    pub async fn handshake_responder(
        &mut self,
        transport: &mut dyn AsyncFrameTransport,
    ) -> Result<(), HandshakeError> {
        self.state = SessionState::Handshaking;

        let result =
            handshake::perform_handshake_responder(&self.identity, transport, &mut self.framer)
                .await?;

        self.cipher = Some(result.cipher);
        self.peer_pubkey = Some(result.peer_pubkey);
        self.peer_node_id = Some(result.peer_node_id);
        self.state = SessionState::Established;

        Ok(())
    }

    /// Send encrypted application data.
    pub async fn send(
        &mut self,
        transport: &mut dyn AsyncFrameTransport,
        channel: u32,
        data: &[u8],
    ) -> Result<(), SessionError> {
        if self.state != SessionState::Established {
            return Err(SessionError::NotEstablished);
        }

        let cipher = self.cipher.as_mut().ok_or(SessionError::NoCipher)?;

        // Encode header bytes for AAD
        let header = FrameHeader::data(channel, 0).with_encrypted();
        let mut header_buf = [0u8; 32];
        let header_len = header
            .encode(&mut header_buf)
            .map_err(|e| SessionError::Wire(e.to_string()))?;
        let aad = &header_buf[..header_len];

        // Encrypt payload
        let encrypted = cipher
            .encrypt(data, aad)
            .map_err(|e| SessionError::Crypto(e.to_string()))?;

        // Update header with actual encrypted payload length
        let final_header = FrameHeader {
            payload_len: encrypted.len() as u64,
            ..header
        };

        self.framer
            .write_frame(transport, &final_header, &encrypted)
            .await
            .map_err(|e| SessionError::Framer(e.to_string()))?;

        Ok(())
    }

    /// Receive the next message. Returns `(channel_id, decrypted_data)`.
    ///
    /// Automatically handles control frames (Ping/Pong/Close).
    pub async fn recv(
        &mut self,
        transport: &mut dyn AsyncFrameTransport,
    ) -> Result<Option<(u32, Vec<u8>)>, SessionError> {
        loop {
            if self.state == SessionState::Closed {
                return Ok(None);
            }

            let (header, payload) = match self.framer.read_frame(transport).await {
                Ok(Some(frame)) => frame,
                Ok(None) => {
                    self.state = SessionState::Closed;
                    return Ok(None);
                }
                Err(e) => return Err(SessionError::Framer(e.to_string())),
            };

            // Handle control frames
            match header.frame_type {
                FrameType::Ping => {
                    let pong = FrameHeader::control(FrameType::Pong);
                    self.framer
                        .write_frame(transport, &pong, &[])
                        .await
                        .map_err(|e| SessionError::Framer(e.to_string()))?;
                    continue; // Don't return pings to the caller
                }
                FrameType::Pong => continue, // Absorb pongs
                FrameType::Close => {
                    self.state = SessionState::Closed;
                    return Ok(None);
                }
                FrameType::Data => {
                    // Decrypt if the frame is marked as encrypted
                    if header.flags.encrypted {
                        let cipher = self.cipher.as_ref().ok_or(SessionError::NoCipher)?;

                        // Reconstruct AAD from the header (same process as sender)
                        let aad_header = FrameHeader::data(header.channel_id, 0).with_encrypted();
                        let mut aad_buf = [0u8; 32];
                        let aad_len = aad_header
                            .encode(&mut aad_buf)
                            .map_err(|e| SessionError::Wire(e.to_string()))?;

                        let decrypted = cipher
                            .decrypt(&payload, &aad_buf[..aad_len])
                            .map_err(|e| SessionError::Crypto(e.to_string()))?;

                        return Ok(Some((header.channel_id, decrypted)));
                    } else {
                        return Ok(Some((header.channel_id, payload.to_vec())));
                    }
                }
                _ => continue, // Skip other frame types for now
            }
        }
    }

    /// Send a graceful close frame.
    pub async fn close(
        &mut self,
        transport: &mut dyn AsyncFrameTransport,
    ) -> Result<(), SessionError> {
        if self.state == SessionState::Closed {
            return Ok(());
        }

        self.state = SessionState::Closing;
        let header = FrameHeader::control(FrameType::Close);
        self.framer
            .write_frame(transport, &header, &[])
            .await
            .map_err(|e| SessionError::Framer(e.to_string()))?;
        self.state = SessionState::Closed;

        Ok(())
    }
}

/// Session-level errors.
#[derive(Debug)]
pub enum SessionError {
    NotEstablished,
    NoCipher,
    Wire(String),
    Crypto(String),
    Framer(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotEstablished => write!(f, "session not established (handshake not complete)"),
            Self::NoCipher => write!(f, "no session cipher available"),
            Self::Wire(e) => write!(f, "wire error: {e}"),
            Self::Crypto(e) => write!(f, "crypto error: {e}"),
            Self::Framer(e) => write!(f, "framer error: {e}"),
        }
    }
}

impl std::error::Error for SessionError {}
