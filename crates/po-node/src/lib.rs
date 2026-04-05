//! Complete PO node with discovery, connection management, and high-level API.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use po_node::Po;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Start a node (listen + discover)
//!     let mut node = Po::bind(4433).await.unwrap();
//!
//!     // Or connect to a specific peer
//!     let mut node = Po::connect("192.168.1.5:4433").await.unwrap();
//!
//!     // Send data
//!     node.send(b"Hello!").await.unwrap();
//!
//!     // Receive data
//!     let data = node.recv().await.unwrap();
//! }
//! ```

pub mod node;
pub mod discovery;
pub mod peer;

pub use node::Po;
pub use peer::PeerInfo;
pub use discovery::Discovery;
