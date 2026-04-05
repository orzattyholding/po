//! Transport trait definitions for PO.
//!
//! The `AsyncFrameTransport` trait is the abstraction boundary between
//! the protocol logic and the physical transport. Any medium that can
//! send and receive ordered bytes can implement this trait.

use thiserror::Error;

/// Errors from transport operations.
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("connection closed by peer")]
    ConnectionClosed,

    #[error("connection timed out")]
    Timeout,

    #[error("I/O error: {0}")]
    Io(String),

    #[error("TLS/QUIC error: {0}")]
    Quic(String),

    #[error("transport not connected")]
    NotConnected,
}

/// An async bidirectional byte transport for PO frames.
///
/// Implementations:
/// - `QuicTransport` — QUIC streams via Quinn
/// - `MemoryTransport` — In-memory pipe (for tests)
///
/// Future implementations: BLE, Wi-Fi Direct, LoRa, Serial
#[async_trait::async_trait]
pub trait AsyncFrameTransport: Send + Sync {
    /// Read bytes into the buffer.
    ///
    /// Returns `Ok(n)` where `n` is the number of bytes read (0 < n <= buf.len()),
    /// or `Err(TransportError::ConnectionClosed)` if the stream has ended.
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;

    /// Write all bytes to the transport. This must not return until all
    /// bytes have been accepted by the transport layer.
    async fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError>;

    /// Flush any buffered data to the network.
    async fn flush(&mut self) -> Result<(), TransportError> {
        Ok(()) // Default no-op; transports can override
    }

    /// Gracefully close the transport.
    async fn close(&mut self) -> Result<(), TransportError> {
        Ok(()) // Default no-op
    }
}
