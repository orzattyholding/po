//! Frame reader/writer for PO connections.
//!
//! The `Framer` handles buffering, fragmentation, and reassembly of PO frames
//! over any `AsyncFrameTransport`. It manages both reading and writing —
//! unlike the original implementation which only handled reads.

use bytes::{Bytes, BytesMut};
use po_wire::{FrameHeader, WireError};
use po_transport::traits::{AsyncFrameTransport, TransportError};

/// Default maximum frame payload size (10 MB).
const DEFAULT_MAX_FRAME_SIZE: u64 = 10 * 1024 * 1024;

/// Frame reader/writer that handles buffering and reassembly.
pub struct Framer {
    /// Read buffer for accumulating incoming bytes.
    read_buf: BytesMut,
    /// Maximum allowed payload size.
    max_frame_size: u64,
}

impl Default for Framer {
    fn default() -> Self {
        Self::new()
    }
}

impl Framer {
    /// Create a new `Framer` with default settings.
    pub fn new() -> Self {
        Self {
            read_buf: BytesMut::with_capacity(65536),
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
        }
    }

    /// Set the maximum allowed payload size for incoming frames.
    pub fn with_max_frame_size(mut self, max: u64) -> Self {
        self.max_frame_size = max;
        self
    }

    // ─── Writing ────────────────────────────────────────────────────────

    /// Write a complete frame (header + payload) to the transport.
    ///
    /// Coalesces header + payload into a single `write_all` call to minimize
    /// syscalls and QUIC packet fragmentation.
    pub async fn write_frame(
        &self,
        transport: &mut dyn AsyncFrameTransport,
        header: &FrameHeader,
        payload: &[u8],
    ) -> Result<(), FramerError> {
        let header_len = header.encoded_len();
        let total_len = header_len + payload.len();

        // Coalesce header + payload into a single buffer for one write_all call.
        // Stack-allocated for small frames (≤ 25-byte header + no payload),
        // heap-allocated otherwise.
        let mut combined = Vec::with_capacity(total_len);
        combined.resize(header_len, 0u8);
        header.encode(&mut combined[..header_len]).map_err(FramerError::Wire)?;
        combined.extend_from_slice(payload);

        transport
            .write_all(&combined)
            .await
            .map_err(FramerError::Transport)?;

        Ok(())
    }

    // ─── Reading ────────────────────────────────────────────────────────

    /// Read the next complete frame from the transport.
    ///
    /// Returns `None` if the connection was cleanly closed.
    pub async fn read_frame(
        &mut self,
        transport: &mut dyn AsyncFrameTransport,
    ) -> Result<Option<(FrameHeader, Bytes)>, FramerError> {
        loop {
            // 1. Try to parse a header from the current buffer
            if let Some((header, header_len)) = self.try_parse_header()? {
                // Validate payload size
                if header.payload_len > self.max_frame_size {
                    return Err(FramerError::Wire(WireError::PayloadTooLarge {
                        declared: header.payload_len,
                        max_allowed: self.max_frame_size,
                    }));
                }

                let total_needed = header_len + header.payload_len as usize;

                // 2. Do we have the full frame?
                if self.read_buf.len() >= total_needed {
                    // Consume header bytes
                    let _ = self.read_buf.split_to(header_len);
                    // Consume payload bytes
                    let payload = self.read_buf.split_to(header.payload_len as usize).freeze();
                    return Ok(Some((header, payload)));
                }

                // 3. Need more bytes — read at least enough for the payload
                let still_needed = total_needed - self.read_buf.len();
                if !self.fill_buffer(transport, still_needed).await? {
                    return Ok(None); // Connection closed
                }
                continue;
            }

            // 4. Not enough data for a header — read more
            if !self.fill_buffer(transport, 1).await? {
                if self.read_buf.is_empty() {
                    return Ok(None); // Clean EOF
                }
                return Err(FramerError::Wire(WireError::Incomplete {
                    needed_min: 4,
                    available: self.read_buf.len(),
                }));
            }
        }
    }

    /// Try to parse a `FrameHeader` from the current read buffer.
    fn try_parse_header(&self) -> Result<Option<(FrameHeader, usize)>, FramerError> {
        if self.read_buf.is_empty() {
            return Ok(None);
        }
        match FrameHeader::decode(&self.read_buf) {
            Ok((header, len)) => Ok(Some((header, len))),
            Err(WireError::Incomplete { .. }) => Ok(None), // Need more data
            Err(e) => Err(FramerError::Wire(e)),
        }
    }

    /// Read at least `min_bytes` additional bytes into the buffer.
    /// Returns `false` if the connection was closed before any bytes were read.
    async fn fill_buffer(
        &mut self,
        transport: &mut dyn AsyncFrameTransport,
        min_bytes: usize,
    ) -> Result<bool, FramerError> {
        let mut total = 0;
        let mut tmp = [0u8; 65536];

        while total < min_bytes {
            match transport.read(&mut tmp).await {
                Ok(n) => {
                    self.read_buf.extend_from_slice(&tmp[..n]);
                    total += n;
                }
                Err(TransportError::ConnectionClosed) => {
                    return Ok(false);
                }
                Err(e) => return Err(FramerError::Transport(e)),
            }
        }

        Ok(true)
    }

    /// Number of buffered bytes not yet consumed.
    pub fn buffered(&self) -> usize {
        self.read_buf.len()
    }
}

/// Errors from the framing layer.
#[derive(Debug)]
pub enum FramerError {
    Wire(WireError),
    Transport(TransportError),
}

impl std::fmt::Display for FramerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wire(e) => write!(f, "wire: {e}"),
            Self::Transport(e) => write!(f, "transport: {e}"),
        }
    }
}

impl std::error::Error for FramerError {}

#[cfg(test)]
mod tests {
    use super::*;
    use po_wire::FrameType;
    use po_transport::MemoryTransport;

    #[tokio::test]
    async fn write_and_read_data_frame() {
        let (mut a, mut b) = MemoryTransport::pair(64);
        let framer_w = Framer::new();
        let mut framer_r = Framer::new();

        let payload = b"Hello Protocol Orzatty!";
        let header = FrameHeader::data(0, payload.len() as u64);

        framer_w.write_frame(&mut a, &header, payload).await.unwrap();

        let (recv_header, recv_payload) = framer_r.read_frame(&mut b).await.unwrap().unwrap();
        assert_eq!(recv_header.frame_type, FrameType::Data);
        assert_eq!(recv_payload.as_ref(), payload);
    }

    #[tokio::test]
    async fn write_and_read_control_frame() {
        let (mut a, mut b) = MemoryTransport::pair(64);
        let framer_w = Framer::new();
        let mut framer_r = Framer::new();

        let header = FrameHeader::control(FrameType::Ping);
        framer_w.write_frame(&mut a, &header, &[]).await.unwrap();

        let (recv_header, recv_payload) = framer_r.read_frame(&mut b).await.unwrap().unwrap();
        assert_eq!(recv_header.frame_type, FrameType::Ping);
        assert!(recv_header.flags.control);
        assert!(recv_payload.is_empty());
    }

    #[tokio::test]
    async fn multiple_frames_sequential() {
        let (mut a, mut b) = MemoryTransport::pair(64);
        let framer_w = Framer::new();
        let mut framer_r = Framer::new();

        for i in 0u8..10 {
            let payload = vec![i; (i as usize + 1) * 10];
            let header = FrameHeader::data(i as u32, payload.len() as u64);
            framer_w.write_frame(&mut a, &header, &payload).await.unwrap();
        }

        for i in 0u8..10 {
            let (h, p) = framer_r.read_frame(&mut b).await.unwrap().unwrap();
            assert_eq!(h.channel_id, i as u32);
            assert_eq!(p.len(), (i as usize + 1) * 10);
            assert!(p.iter().all(|&b| b == i));
        }
    }

    #[tokio::test]
    async fn eof_returns_none() {
        let (a, mut b) = MemoryTransport::pair(64);
        let mut framer_r = Framer::new();

        drop(a); // Close the writer

        let result = framer_r.read_frame(&mut b).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn large_payload() {
        let (mut a, mut b) = MemoryTransport::pair(256);
        let framer_w = Framer::new();
        let mut framer_r = Framer::new();

        let payload = vec![0xAB; 100_000]; // 100KB
        let header = FrameHeader::data(1, payload.len() as u64);
        framer_w.write_frame(&mut a, &header, &payload).await.unwrap();

        let (h, p) = framer_r.read_frame(&mut b).await.unwrap().unwrap();
        assert_eq!(h.payload_len, 100_000);
        assert_eq!(p.as_ref(), payload.as_slice());
    }
}
