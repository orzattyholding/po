//! In-memory transport for testing PO without a network.
//!
//! Creates a pair of connected transports using tokio mpsc channels.
//! Perfect for unit and integration tests.

use tokio::sync::mpsc;
use crate::traits::{AsyncFrameTransport, TransportError};

/// An in-memory transport backed by tokio mpsc channels.
pub struct MemoryTransport {
    tx: mpsc::Sender<Vec<u8>>,
    rx: mpsc::Receiver<Vec<u8>>,
    /// Leftover bytes from the last received chunk that haven't been
    /// consumed yet by a `read()` call.
    read_buf: Vec<u8>,
    read_pos: usize,
}

impl MemoryTransport {
    /// Create a pair of connected in-memory transports.
    ///
    /// Data written to `a` can be read from `b`, and vice versa.
    pub fn pair(buffer_size: usize) -> (Self, Self) {
        let (tx_a, rx_b) = mpsc::channel(buffer_size);
        let (tx_b, rx_a) = mpsc::channel(buffer_size);

        let a = Self {
            tx: tx_a,
            rx: rx_a,
            read_buf: Vec::new(),
            read_pos: 0,
        };
        let b = Self {
            tx: tx_b,
            rx: rx_b,
            read_buf: Vec::new(),
            read_pos: 0,
        };

        (a, b)
    }
}

#[async_trait::async_trait]
impl AsyncFrameTransport for MemoryTransport {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        // If we have leftover bytes from a previous recv, use those first
        if self.read_pos < self.read_buf.len() {
            let remaining = &self.read_buf[self.read_pos..];
            let n = remaining.len().min(buf.len());
            buf[..n].copy_from_slice(&remaining[..n]);
            self.read_pos += n;
            if self.read_pos >= self.read_buf.len() {
                self.read_buf.clear();
                self.read_pos = 0;
            }
            return Ok(n);
        }

        // Wait for new data from the channel
        match self.rx.recv().await {
            Some(data) => {
                let n = data.len().min(buf.len());
                buf[..n].copy_from_slice(&data[..n]);
                if n < data.len() {
                    // Store the rest for the next read
                    self.read_buf = data;
                    self.read_pos = n;
                }
                Ok(n)
            }
            None => Err(TransportError::ConnectionClosed),
        }
    }

    async fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.tx
            .send(data.to_vec())
            .await
            .map_err(|_| TransportError::ConnectionClosed)
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        // Dropping the sender closes the channel
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bidirectional_communication() {
        let (mut a, mut b) = MemoryTransport::pair(32);

        a.write_all(b"hello from A").await.unwrap();
        b.write_all(b"hello from B").await.unwrap();

        let mut buf = [0u8; 64];
        let n = b.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello from A");

        let n = a.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"hello from B");
    }

    #[tokio::test]
    async fn partial_reads() {
        let (mut a, mut b) = MemoryTransport::pair(32);

        a.write_all(b"hello world!").await.unwrap();

        // Read in small chunks
        let mut small_buf = [0u8; 5];
        let n = b.read(&mut small_buf).await.unwrap();
        assert_eq!(&small_buf[..n], b"hello");

        let n = b.read(&mut small_buf).await.unwrap();
        assert_eq!(&small_buf[..n], b" worl");

        let n = b.read(&mut small_buf).await.unwrap();
        assert_eq!(&small_buf[..n], b"d!");
    }

    #[tokio::test]
    async fn connection_closed_on_drop() {
        let (a, mut b) = MemoryTransport::pair(32);
        drop(a); // Close sender side

        let mut buf = [0u8; 64];
        let result = b.read(&mut buf).await;
        assert!(matches!(result, Err(TransportError::ConnectionClosed)));
    }
}
