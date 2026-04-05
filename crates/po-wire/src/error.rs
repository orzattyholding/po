//! Error types for the PO wire format.
//!
//! All errors are `no_std` compatible and carry precise diagnostic information
//! for debugging malformed frames without allocating.

use core::fmt;

/// Errors that occur during frame encoding or decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    /// The output buffer is too small to hold the encoded data.
    BufferTooSmall {
        /// Minimum bytes needed to complete the operation.
        needed: usize,
        /// Bytes actually available in the buffer.
        available: usize,
    },

    /// The input buffer does not contain enough data for a complete decode.
    /// This is NOT a fatal error — it means "read more bytes and try again".
    Incomplete {
        /// Minimum additional bytes needed (estimate).
        needed_min: usize,
        /// Bytes currently available.
        available: usize,
    },

    /// A VarInt encoding is malformed (overflow or invalid prefix).
    InvalidVarInt,

    /// The frame type nibble does not map to a known `FrameType`.
    UnknownFrameType(u8),

    /// The payload length declared in the header exceeds the configured maximum.
    PayloadTooLarge {
        /// Declared payload length.
        declared: u64,
        /// Maximum allowed by configuration.
        max_allowed: u64,
    },
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall { needed, available } => {
                write!(f, "buffer too small: need {needed} bytes, have {available}")
            }
            Self::Incomplete {
                needed_min,
                available,
            } => {
                write!(
                    f,
                    "incomplete input: need at least {needed_min} bytes, have {available}"
                )
            }
            Self::InvalidVarInt => write!(f, "malformed VarInt encoding"),
            Self::UnknownFrameType(t) => write!(f, "unknown frame type: {t:#04x}"),
            Self::PayloadTooLarge {
                declared,
                max_allowed,
            } => {
                write!(f, "payload too large: {declared} bytes (max {max_allowed})")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WireError {}
