//! Frame type definitions for Protocol Orzatty.
//!
//! Each frame type uses 4 bits (lower nibble of byte 0), supporting up to 16 types.

use crate::error::WireError;
use core::fmt;

/// The type of a PO frame, encoded in the lower 4 bits of the first header byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    /// Application data (chat messages, RPC payloads, etc.)
    Data = 0x00,

    /// Initiate cryptographic handshake (sends Ed25519 pubkey + X25519 ephemeral).
    HandshakeInit = 0x01,

    /// Reply to handshake with responder's keys.
    HandshakeReply = 0x02,

    /// Confirm handshake — session key is now active.
    HandshakeComplete = 0x03,

    /// Keep-alive ping. Should be sent with CONTROL flag.
    Ping = 0x04,

    /// Ping response. Should be sent with CONTROL flag.
    Pong = 0x05,

    /// Graceful connection close. Should be sent with CONTROL flag.
    Close = 0x06,

    /// File transfer metadata (name, size, hash).
    FileHeader = 0x07,

    /// File data chunk (sequential binary payload).
    FileChunk = 0x08,

    /// Acknowledgement of a received frame or operation.
    Ack = 0x09,
}

impl FrameType {
    /// Try to convert a raw `u8` nibble (0x00–0x0F) into a `FrameType`.
    #[inline]
    pub const fn from_u8(value: u8) -> Result<Self, WireError> {
        match value {
            0x00 => Ok(Self::Data),
            0x01 => Ok(Self::HandshakeInit),
            0x02 => Ok(Self::HandshakeReply),
            0x03 => Ok(Self::HandshakeComplete),
            0x04 => Ok(Self::Ping),
            0x05 => Ok(Self::Pong),
            0x06 => Ok(Self::Close),
            0x07 => Ok(Self::FileHeader),
            0x08 => Ok(Self::FileChunk),
            0x09 => Ok(Self::Ack),
            other => Err(WireError::UnknownFrameType(other)),
        }
    }

    /// Returns `true` if this is a handshake-related frame.
    #[inline]
    pub const fn is_handshake(&self) -> bool {
        matches!(
            self,
            Self::HandshakeInit | Self::HandshakeReply | Self::HandshakeComplete
        )
    }

    /// Returns `true` if this is a control frame (non-application data).
    #[inline]
    pub const fn is_control(&self) -> bool {
        matches!(self, Self::Ping | Self::Pong | Self::Close)
    }

    /// Returns `true` if this carries file transfer data.
    #[inline]
    pub const fn is_file_transfer(&self) -> bool {
        matches!(self, Self::FileHeader | Self::FileChunk)
    }
}

impl fmt::Display for FrameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Data => write!(f, "DATA"),
            Self::HandshakeInit => write!(f, "HS_INIT"),
            Self::HandshakeReply => write!(f, "HS_REPLY"),
            Self::HandshakeComplete => write!(f, "HS_COMPLETE"),
            Self::Ping => write!(f, "PING"),
            Self::Pong => write!(f, "PONG"),
            Self::Close => write!(f, "CLOSE"),
            Self::FileHeader => write!(f, "FILE_HDR"),
            Self::FileChunk => write!(f, "FILE_CHUNK"),
            Self::Ack => write!(f, "ACK"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_types_roundtrip() {
        let types = [
            (0x00, FrameType::Data),
            (0x01, FrameType::HandshakeInit),
            (0x02, FrameType::HandshakeReply),
            (0x03, FrameType::HandshakeComplete),
            (0x04, FrameType::Ping),
            (0x05, FrameType::Pong),
            (0x06, FrameType::Close),
            (0x07, FrameType::FileHeader),
            (0x08, FrameType::FileChunk),
            (0x09, FrameType::Ack),
        ];
        for (byte, expected) in types {
            assert_eq!(FrameType::from_u8(byte), Ok(expected));
            assert_eq!(expected as u8, byte);
        }
    }

    #[test]
    fn unknown_type_rejected() {
        for byte in 0x0A..=0x0F {
            assert!(matches!(
                FrameType::from_u8(byte),
                Err(WireError::UnknownFrameType(_))
            ));
        }
    }

    #[test]
    fn classification() {
        assert!(FrameType::HandshakeInit.is_handshake());
        assert!(FrameType::HandshakeReply.is_handshake());
        assert!(FrameType::HandshakeComplete.is_handshake());
        assert!(!FrameType::Data.is_handshake());

        assert!(FrameType::Ping.is_control());
        assert!(FrameType::Pong.is_control());
        assert!(FrameType::Close.is_control());
        assert!(!FrameType::Data.is_control());

        assert!(FrameType::FileHeader.is_file_transfer());
        assert!(FrameType::FileChunk.is_file_transfer());
        assert!(!FrameType::Data.is_file_transfer());
    }
}
