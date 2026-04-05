//! High-level PO node — the "ridiculously easy" API.
//!
//! # Examples
//!
//! ```rust,no_run
//! use po_node::Po;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Server: 2 lines
//!     let mut server = Po::bind(4433).await?;
//!     let (peer, data) = server.recv().await?.unwrap();
//!     println!("Got: {}", String::from_utf8_lossy(&data));
//!
//!     // Client: 3 lines
//!     let mut client = Po::connect("127.0.0.1:4433").await?;
//!     client.send(b"Hello!").await?;
//!     Ok(())
//! }
//! ```

use std::net::SocketAddr;
use po_crypto::identity::Identity;
use po_transport::quic::{QuicTransport, QuicListener, QuicConfig};
use po_session::state::Session;
use po_session::channel::channels;

use crate::peer::PeerInfo;

/// A PO node — the main entry point for the protocol.
///
/// Wraps all the complexity of QUIC, crypto handshake, and encryption
/// behind a dead-simple API.
pub struct Po {
    session: Session,
    transport: QuicTransport,
    identity: Identity,
    peer_info: Option<PeerInfo>,
}

impl Po {
    /// Connect to a remote PO node.
    ///
    /// This will:
    /// 1. Establish a QUIC connection
    /// 2. Perform the E2EE handshake (Ed25519 + X25519 + ChaCha20)
    /// 3. Return a ready-to-use encrypted connection
    ///
    /// ```ignore
    /// let mut po = Po::connect("192.168.1.5:4433").await?;
    /// po.send(b"encrypted hello!").await?;
    /// ```
    pub async fn connect(addr: &str) -> Result<Self, PoError> {
        let socket_addr: SocketAddr = addr.parse()
            .map_err(|e| PoError::Config(format!("invalid address '{addr}': {e}")))?;

        let identity = Identity::generate();
        let mut transport = QuicTransport::connect(socket_addr).await
            .map_err(|e| PoError::Transport(e.to_string()))?;

        let mut session = Session::new(Identity::from_bytes(&identity.secret_key_bytes()));
        session.handshake_initiator(&mut transport).await
            .map_err(|e| PoError::Handshake(e.to_string()))?;

        let peer_info = session.peer_node_id().map(|id| PeerInfo {
            node_id: *id,
            addr: socket_addr,
            pubkey: [0u8; 32], // TODO: extract from session
        });

        Ok(Self {
            session,
            transport,
            identity,
            peer_info,
        })
    }

    /// Listen for an incoming connection on the given port.
    ///
    /// This will:
    /// 1. Start a QUIC listener
    /// 2. Accept the first incoming connection
    /// 3. Perform the E2EE handshake
    /// 4. Return a ready-to-use encrypted connection
    ///
    /// ```ignore
    /// let mut po = Po::bind(4433).await?;
    /// let data = po.recv().await?;
    /// ```
    pub async fn bind(port: u16) -> Result<Self, PoError> {
        let identity = Identity::generate();

        let config = QuicConfig {
            bind_addr: format!("0.0.0.0:{port}").parse().unwrap(),
        };
        let listener = QuicListener::bind(config).await
            .map_err(|e| PoError::Transport(e.to_string()))?;

        let mut transport = listener.accept().await
            .map_err(|e| PoError::Transport(e.to_string()))?;

        let mut session = Session::new(Identity::from_bytes(&identity.secret_key_bytes()));
        session.handshake_responder(&mut transport).await
            .map_err(|e| PoError::Handshake(e.to_string()))?;

        let peer_info = session.peer_node_id().map(|id| PeerInfo {
            node_id: *id,
            addr: format!("0.0.0.0:{port}").parse().unwrap(),
            pubkey: [0u8; 32],
        });

        Ok(Self {
            session,
            transport,
            identity,
            peer_info,
        })
    }

    /// Send encrypted data to the connected peer.
    pub async fn send(&mut self, data: &[u8]) -> Result<(), PoError> {
        self.session
            .send(&mut self.transport, channels::DEFAULT, data)
            .await
            .map_err(|e| PoError::Session(e.to_string()))
    }

    /// Receive the next message from the connected peer.
    ///
    /// Returns `Some((channel_id, data))` or `None` if the connection closed.
    pub async fn recv(&mut self) -> Result<Option<(u32, Vec<u8>)>, PoError> {
        self.session
            .recv(&mut self.transport)
            .await
            .map_err(|e| PoError::Session(e.to_string()))
    }

    /// Get our node ID.
    pub fn node_id(&self) -> String {
        self.session.node_id().to_hex()
    }

    /// Get our Ed25519 public key bytes (32 bytes).
    pub fn public_key(&self) -> [u8; 32] {
        self.identity.public_key_bytes()
    }

    /// Get the peer's node ID (if connected).
    pub fn peer_node_id(&self) -> Option<String> {
        self.session.peer_node_id().map(|id| id.to_hex())
    }

    /// Get information about the connected peer.
    pub fn peer_info(&self) -> Option<&PeerInfo> {
        self.peer_info.as_ref()
    }

    /// Gracefully close the connection.
    pub async fn close(&mut self) -> Result<(), PoError> {
        self.session
            .close(&mut self.transport)
            .await
            .map_err(|e| PoError::Session(e.to_string()))
    }
}

/// Errors from the PO node.
#[derive(Debug)]
pub enum PoError {
    Config(String),
    Transport(String),
    Handshake(String),
    Session(String),
}

impl std::fmt::Display for PoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(e) => write!(f, "config: {e}"),
            Self::Transport(e) => write!(f, "transport: {e}"),
            Self::Handshake(e) => write!(f, "handshake: {e}"),
            Self::Session(e) => write!(f, "session: {e}"),
        }
    }
}

impl std::error::Error for PoError {}
