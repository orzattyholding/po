//! PO Frame Header — the compact binary header that precedes every frame.
//!
//! ## Wire Layout
//!
//! ```text
//! ┌─────────┬──────────────┬──────────────┬──────────────┐
//! │ Byte 0  │ VarInt       │ VarInt       │ VarInt       │
//! │ Type +  │ Channel ID   │ Stream ID    │ Payload Len  │
//! │ Flags   │ (1–8 bytes)  │ (1–8 bytes)  │ (1–8 bytes)  │
//! └─────────┴──────────────┴──────────────┴──────────────┘
//!
//! Byte 0 bit layout:
//! ┌───┬───┬───┬───┬───┬───┬───┬───┐
//! │ 7 │ 6 │ 5 │ 4 │ 3 │ 2 │ 1 │ 0 │
//! │CTL│PRI│ENC│RSV│    FrameType   │
//! └───┴───┴───┴───┴───┴───┴───┴───┘
//! ```
//!
//! **Minimum header size: 4 bytes** (1 type/flags + 3×1-byte VarInts).
//! **Maximum header size: 25 bytes** (1 type/flags + 3×8-byte VarInts).

use crate::error::WireError;
use crate::frame_type::FrameType;
use crate::varint;

// --- Flag bit positions in byte 0 ---

/// Bit 7: This is a control frame (not application data).
const FLAG_CONTROL: u8 = 0b1000_0000;
/// Bit 6: High-priority — process immediately, bypass queues.
const FLAG_PRIORITY: u8 = 0b0100_0000;
/// Bit 5: Payload is encrypted with the session cipher.
const FLAG_ENCRYPTED: u8 = 0b0010_0000;
/// Bit 4: Reserved for future use.
const _FLAG_RESERVED: u8 = 0b0001_0000;
/// Mask for the frame type (lower 4 bits).
const TYPE_MASK: u8 = 0x0F;

/// Flags that modify how a frame is processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameFlags {
    /// This is a control frame (ping, pong, close).
    pub control: bool,
    /// High priority — should bypass normal processing queues.
    pub priority: bool,
    /// Payload is encrypted with the session's ChaCha20-Poly1305 cipher.
    pub encrypted: bool,
}

impl FrameFlags {
    /// Encode flags into the upper 4 bits of byte 0.
    #[inline]
    const fn to_bits(self) -> u8 {
        let mut bits = 0u8;
        if self.control {
            bits |= FLAG_CONTROL;
        }
        if self.priority {
            bits |= FLAG_PRIORITY;
        }
        if self.encrypted {
            bits |= FLAG_ENCRYPTED;
        }
        bits
    }

    /// Decode flags from byte 0.
    #[inline]
    const fn from_bits(byte: u8) -> Self {
        Self {
            control: byte & FLAG_CONTROL != 0,
            priority: byte & FLAG_PRIORITY != 0,
            encrypted: byte & FLAG_ENCRYPTED != 0,
        }
    }
}

/// A decoded PO frame header.
///
/// This is a pure value type — it does not own or reference the payload.
/// After decoding the header, read `payload_len` bytes from the transport
/// to get the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// The type of this frame (Data, Handshake, Ping, etc.).
    pub frame_type: FrameType,
    /// Processing flags (control, priority, encrypted).
    pub flags: FrameFlags,
    /// Logical channel for application-level multiplexing.
    pub channel_id: u32,
    /// Stream identifier for QUIC-like concurrent streams within a channel.
    pub stream_id: u64,
    /// Length of the payload that follows this header.
    pub payload_len: u64,
}

impl FrameHeader {
    /// Create a new header for a data frame with default flags.
    #[inline]
    pub const fn data(channel_id: u32, payload_len: u64) -> Self {
        Self {
            frame_type: FrameType::Data,
            flags: FrameFlags {
                control: false,
                priority: false,
                encrypted: false,
            },
            channel_id,
            stream_id: 0,
            payload_len,
        }
    }

    /// Create a new control frame header (e.g., Ping, Pong, Close).
    #[inline]
    pub const fn control(frame_type: FrameType) -> Self {
        Self {
            frame_type,
            flags: FrameFlags {
                control: true,
                priority: false,
                encrypted: false,
            },
            channel_id: 0,
            stream_id: 0,
            payload_len: 0,
        }
    }

    /// Set the encrypted flag on this header.
    #[inline]
    pub const fn with_encrypted(mut self) -> Self {
        self.flags.encrypted = true;
        self
    }

    /// Set the priority flag on this header.
    #[inline]
    pub const fn with_priority(mut self) -> Self {
        self.flags.priority = true;
        self
    }

    /// Set the stream ID.
    #[inline]
    pub const fn with_stream(mut self, stream_id: u64) -> Self {
        self.stream_id = stream_id;
        self
    }

    /// Calculate the exact number of bytes this header will occupy when encoded.
    #[inline]
    pub const fn encoded_len(&self) -> usize {
        1 // byte 0 (type + flags)
        + varint::encoded_len(self.channel_id as u64)
        + varint::encoded_len(self.stream_id)
        + varint::encoded_len(self.payload_len)
    }

    /// Encode this header into the provided buffer.
    ///
    /// Returns the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, WireError> {
        let needed = self.encoded_len();
        if buf.len() < needed {
            return Err(WireError::BufferTooSmall {
                needed,
                available: buf.len(),
            });
        }

        let mut offset = 0;

        // Byte 0: flags (upper 4 bits) + frame type (lower 4 bits)
        buf[0] = self.flags.to_bits() | (self.frame_type as u8 & TYPE_MASK);
        offset += 1;

        // VarInt: channel_id
        offset += varint::encode(self.channel_id as u64, &mut buf[offset..])?;

        // VarInt: stream_id
        offset += varint::encode(self.stream_id, &mut buf[offset..])?;

        // VarInt: payload_len
        offset += varint::encode(self.payload_len, &mut buf[offset..])?;

        debug_assert_eq!(offset, needed);
        Ok(offset)
    }

    /// Decode a header from the provided buffer.
    ///
    /// Returns `(header, bytes_consumed)`.
    ///
    /// # Errors
    /// - `WireError::Incomplete` if the buffer doesn't contain a complete header.
    /// - `WireError::UnknownFrameType` if the type nibble is invalid.
    pub fn decode(buf: &[u8]) -> Result<(Self, usize), WireError> {
        if buf.is_empty() {
            return Err(WireError::Incomplete {
                needed_min: 4,
                available: 0,
            });
        }

        let byte0 = buf[0];
        let mut offset = 1;

        // Decode flags
        let flags = FrameFlags::from_bits(byte0);

        // Decode frame type from lower 4 bits
        let frame_type = FrameType::from_u8(byte0 & TYPE_MASK)?;

        // Decode channel_id
        let (channel_raw, n) = varint::decode(&buf[offset..]).map_err(|e| match e {
            WireError::Incomplete { needed_min, .. } => WireError::Incomplete {
                needed_min: offset + needed_min,
                available: buf.len(),
            },
            other => other,
        })?;
        offset += n;

        // Decode stream_id
        let (stream_id, n) = varint::decode(&buf[offset..]).map_err(|e| match e {
            WireError::Incomplete { needed_min, .. } => WireError::Incomplete {
                needed_min: offset + needed_min,
                available: buf.len(),
            },
            other => other,
        })?;
        offset += n;

        // Decode payload_len
        let (payload_len, n) = varint::decode(&buf[offset..]).map_err(|e| match e {
            WireError::Incomplete { needed_min, .. } => WireError::Incomplete {
                needed_min: offset + needed_min,
                available: buf.len(),
            },
            other => other,
        })?;
        offset += n;

        Ok((
            Self {
                frame_type,
                flags,
                channel_id: channel_raw as u32,
                stream_id,
                payload_len,
            },
            offset,
        ))
    }
}

impl core::fmt::Display for FrameHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[{}] ch={} stream={} len={} flags=[{}{}{}]",
            self.frame_type,
            self.channel_id,
            self.stream_id,
            self.payload_len,
            if self.flags.control { "C" } else { "" },
            if self.flags.priority { "P" } else { "" },
            if self.flags.encrypted { "E" } else { "" },
        )
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use alloc::format;

    #[test]
    fn minimal_data_header() {
        // Smallest possible: Data type, channel 0, stream 0, payload 0
        let h = FrameHeader::data(0, 0);
        let mut buf = [0u8; 32];
        let n = h.encode(&mut buf).unwrap();
        assert_eq!(n, 4, "minimum header should be 4 bytes");
        assert_eq!(buf[0] & TYPE_MASK, 0x00); // Data type
        assert_eq!(buf[0] & 0xF0, 0x00); // No flags

        let (decoded, consumed) = FrameHeader::decode(&buf[..n]).unwrap();
        assert_eq!(decoded, h);
        assert_eq!(consumed, n);
    }

    #[test]
    fn data_with_flags() {
        let h = FrameHeader::data(1, 100)
            .with_encrypted()
            .with_priority()
            .with_stream(42);

        let mut buf = [0u8; 32];
        let n = h.encode(&mut buf).unwrap();

        // Byte 0: PRI(0x40) + ENC(0x20) + Data(0x00) = 0x60
        assert_eq!(buf[0], 0x60);

        let (decoded, consumed) = FrameHeader::decode(&buf[..n]).unwrap();
        assert_eq!(decoded, h);
        assert_eq!(consumed, n);
    }

    #[test]
    fn control_ping() {
        let h = FrameHeader::control(FrameType::Ping);
        let mut buf = [0u8; 32];
        let n = h.encode(&mut buf).unwrap();

        // Byte 0: CONTROL(0x80) + Ping(0x04) = 0x84
        assert_eq!(buf[0], 0x84);
        assert_eq!(n, 4); // 1 + three 1-byte varints (all 0)

        let (decoded, _) = FrameHeader::decode(&buf[..n]).unwrap();
        assert!(decoded.flags.control);
        assert_eq!(decoded.frame_type, FrameType::Ping);
    }

    #[test]
    fn handshake_type() {
        for ft in [
            FrameType::HandshakeInit,
            FrameType::HandshakeReply,
            FrameType::HandshakeComplete,
        ] {
            let h = FrameHeader {
                frame_type: ft,
                flags: FrameFlags::default(),
                channel_id: 0,
                stream_id: 0,
                payload_len: 128,
            };
            let mut buf = [0u8; 32];
            let n = h.encode(&mut buf).unwrap();
            let (decoded, _) = FrameHeader::decode(&buf[..n]).unwrap();
            assert_eq!(decoded.frame_type, ft);
            assert!(decoded.frame_type.is_handshake());
        }
    }

    #[test]
    fn large_values() {
        let h = FrameHeader {
            frame_type: FrameType::FileChunk,
            flags: FrameFlags {
                control: false,
                priority: true,
                encrypted: true,
            },
            channel_id: 1_000_000,
            stream_id: 9_999_999_999,
            payload_len: 4_294_967_296, // 4GB
        };
        let mut buf = [0u8; 32];
        let n = h.encode(&mut buf).unwrap();
        let (decoded, consumed) = FrameHeader::decode(&buf[..n]).unwrap();
        assert_eq!(decoded, h);
        assert_eq!(consumed, n);
    }

    #[test]
    fn encoded_len_accurate() {
        let h = FrameHeader::data(42, 100).with_stream(12345);
        assert_eq!(h.encoded_len(), h.encode(&mut [0u8; 32]).unwrap());
    }

    #[test]
    fn incomplete_decode() {
        // Only 2 bytes — not enough for a full header
        let buf = [0x00, 0x00];
        match FrameHeader::decode(&buf) {
            Err(WireError::Incomplete { .. }) => {} // Expected
            other => panic!("expected Incomplete, got {other:?}"),
        }
    }

    #[test]
    fn unknown_type_rejected() {
        // Byte 0 with type nibble = 0x0F (reserved)
        let buf = [0x0F, 0x00, 0x00, 0x00];
        assert!(matches!(
            FrameHeader::decode(&buf),
            Err(WireError::UnknownFrameType(0x0F))
        ));
    }

    #[test]
    fn display_format() {
        let h = FrameHeader::data(5, 42).with_encrypted();
        let s = format!("{h}");
        assert!(s.contains("DATA"));
        assert!(s.contains("ch=5"));
        assert!(s.contains("len=42"));
        assert!(s.contains("E")); // Encrypted flag
    }

    #[test]
    fn all_frame_types_encode_decode() {
        let types = [
            FrameType::Data,
            FrameType::HandshakeInit,
            FrameType::HandshakeReply,
            FrameType::HandshakeComplete,
            FrameType::Ping,
            FrameType::Pong,
            FrameType::Close,
            FrameType::FileHeader,
            FrameType::FileChunk,
            FrameType::Ack,
        ];
        for ft in types {
            let h = FrameHeader {
                frame_type: ft,
                flags: FrameFlags::default(),
                channel_id: 0,
                stream_id: 0,
                payload_len: 0,
            };
            let mut buf = [0u8; 32];
            let n = h.encode(&mut buf).unwrap();
            let (decoded, _) = FrameHeader::decode(&buf[..n]).unwrap();
            assert_eq!(decoded.frame_type, ft, "type {ft} failed roundtrip");
        }
    }
}
