//! Peer information and registry.

use po_crypto::identity::NodeId;
use std::net::SocketAddr;

/// Information about a connected peer.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// The peer's cryptographic node ID.
    pub node_id: NodeId,
    /// The peer's network address.
    pub addr: SocketAddr,
    /// The peer's Ed25519 public key.
    pub pubkey: [u8; 32],
}
