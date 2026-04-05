//! UDP broadcast-based peer discovery for LAN environments.
//!
//! Sends periodic beacons on the broadcast address and listens for
//! beacons from other PO nodes on the same network.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use dashmap::DashMap;
use tracing::{debug, warn};

use po_crypto::identity::NodeId;

/// Default discovery port.
pub const DISCOVERY_PORT: u16 = 5433;


/// A discovered peer on the local network.
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub node_id: String,
    pub addr: SocketAddr,
    pub quic_port: u16,
    pub last_seen: std::time::Instant,
}

/// LAN discovery service using UDP broadcast.
pub struct Discovery {
    socket: Arc<UdpSocket>,
    peers: Arc<DashMap<String, DiscoveredPeer>>,
    our_node_id: String,
    our_quic_port: u16,
}

impl Discovery {
    /// Start the discovery service.
    pub async fn start(
        node_id: &NodeId,
        quic_port: u16,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{DISCOVERY_PORT}")).await?;
        socket.set_broadcast(true)?;

        Ok(Self {
            socket: Arc::new(socket),
            peers: Arc::new(DashMap::new()),
            our_node_id: node_id.to_hex(),
            our_quic_port: quic_port,
        })
    }

    /// Send a single discovery beacon.
    pub async fn send_beacon(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let msg = format!("PO|{}|{}", self.our_node_id, self.our_quic_port);
        let broadcast_addr: SocketAddr = format!("255.255.255.255:{DISCOVERY_PORT}").parse()?;
        self.socket.send_to(msg.as_bytes(), broadcast_addr).await?;
        debug!("Sent discovery beacon");
        Ok(())
    }

    /// Listen for a single beacon and register the peer.
    pub async fn listen_once(&self) -> Result<Option<DiscoveredPeer>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buf = [0u8; 256];
        let (n, addr) = self.socket.recv_from(&mut buf).await?;
        let msg = std::str::from_utf8(&buf[..n])?;

        if let Some(peer) = self.parse_beacon(msg, addr) {
            // Don't register ourselves
            if peer.node_id != self.our_node_id {
                self.peers.insert(peer.node_id.clone(), peer.clone());
                return Ok(Some(peer));
            }
        }

        Ok(None)
    }

    /// Get all currently known peers.
    pub fn known_peers(&self) -> Vec<DiscoveredPeer> {
        self.peers.iter().map(|r| r.value().clone()).collect()
    }

    /// Parse a beacon message.
    fn parse_beacon(&self, msg: &str, source: SocketAddr) -> Option<DiscoveredPeer> {
        let parts: Vec<&str> = msg.split('|').collect();
        if parts.len() != 3 || parts[0] != "PO" {
            return None;
        }

        let node_id = parts[1].to_string();
        let quic_port: u16 = parts[2].parse().ok()?;

        Some(DiscoveredPeer {
            node_id,
            addr: source,
            quic_port,
            last_seen: std::time::Instant::now(),
        })
    }

    /// Spawn the background beacon + listener tasks.
    /// Returns a channel that emits newly discovered peers.
    pub fn spawn_background(
        self: Arc<Self>,
        beacon_interval: std::time::Duration,
    ) -> mpsc::Receiver<DiscoveredPeer> {
        let (tx, rx) = mpsc::channel(32);

        // Beacon sender task
        let disc_clone = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                if let Err(e) = disc_clone.send_beacon().await {
                    warn!("Beacon send error: {e}");
                }
                tokio::time::sleep(beacon_interval).await;
            }
        });

        // Beacon listener task
        let disc_clone = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                match disc_clone.listen_once().await {
                    Ok(Some(peer)) => {
                        let _ = tx.send(peer).await;
                    }
                    Ok(None) => {} // Our own beacon or invalid
                    Err(e) => {
                        warn!("Beacon listen error: {e}");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });

        rx
    }
}
