//! # po-wire
//!
//! **Zero-dependency, `no_std` wire format codec for Protocol Orzatty (PO).**
//!
//! This crate provides the binary framing layer — encoding and decoding frame
//! headers with QUIC-style VarInt fields. It is the foundation of the PO stack:
//! every byte that crosses the network goes through `po-wire`.
//!
//! ## Quick Start
//!
//! ```rust
//! use po_wire::{FrameHeader, FrameType};
//!
//! // Encode a data frame header
//! let header = FrameHeader::data(0, 13); // channel 0, 13-byte payload
//! let mut buf = [0u8; 32];
//! let header_len = header.encode(&mut buf).unwrap();
//!
//! // Decode it back
//! let (decoded, consumed) = FrameHeader::decode(&buf[..header_len]).unwrap();
//! assert_eq!(decoded.payload_len, 13);
//! ```
//!
//! ## Features
//!
//! - **Zero dependencies**: No allocator needed. Pure `core` Rust.
//! - **`no_std` compatible**: Runs on WASM, embedded, anywhere.
//! - **Compact**: Minimum 4-byte header for small messages.
//! - **QUIC VarInt**: RFC 9000 §16 variable-length integer encoding.

#![no_std]

pub mod error;
pub mod varint;
pub mod frame_type;
pub mod header;

// --- Public re-exports for ergonomic usage ---
pub use error::WireError;
pub use frame_type::FrameType;
pub use header::{FrameHeader, FrameFlags};
