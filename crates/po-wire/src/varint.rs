//! QUIC-style Variable-Length Integer encoding.
//!
//! Uses the RFC 9000 §16 format: the two most significant bits of the first byte
//! encode the length of the integer (1, 2, 4, or 8 bytes), leaving 6, 14, 30, or
//! 62 bits for the value respectively.
//!
//! | Prefix (2 bits) | Length | Usable Bits | Max Value              |
//! |-----------------|--------|-------------|------------------------|
//! | 00              | 1 byte | 6           | 63                     |
//! | 01              | 2 bytes| 14          | 16,383                 |
//! | 10              | 4 bytes| 30          | 1,073,741,823          |
//! | 11              | 8 bytes| 62          | 4,611,686,018,427,387,903 |

use crate::error::WireError;

/// Maximum value encodable in a QUIC VarInt (2^62 - 1).
pub const VARINT_MAX: u64 = (1 << 62) - 1;

/// Encode a `u64` value as a QUIC-style VarInt into `buf`.
///
/// Returns the number of bytes written.
///
/// # Errors
/// - `WireError::BufferTooSmall` if `buf` is not large enough.
/// - `WireError::InvalidVarInt` if `value` exceeds `VARINT_MAX`.
#[inline]
pub fn encode(value: u64, buf: &mut [u8]) -> Result<usize, WireError> {
    if value > VARINT_MAX {
        return Err(WireError::InvalidVarInt);
    }

    if value <= 63 {
        // 1 byte: prefix 00
        if buf.is_empty() {
            return Err(WireError::BufferTooSmall {
                needed: 1,
                available: 0,
            });
        }
        buf[0] = value as u8;
        Ok(1)
    } else if value <= 16_383 {
        // 2 bytes: prefix 01
        if buf.len() < 2 {
            return Err(WireError::BufferTooSmall {
                needed: 2,
                available: buf.len(),
            });
        }
        let v = (value as u16) | 0x4000;
        buf[..2].copy_from_slice(&v.to_be_bytes());
        Ok(2)
    } else if value <= 1_073_741_823 {
        // 4 bytes: prefix 10
        if buf.len() < 4 {
            return Err(WireError::BufferTooSmall {
                needed: 4,
                available: buf.len(),
            });
        }
        let v = (value as u32) | 0x8000_0000;
        buf[..4].copy_from_slice(&v.to_be_bytes());
        Ok(4)
    } else {
        // 8 bytes: prefix 11
        if buf.len() < 8 {
            return Err(WireError::BufferTooSmall {
                needed: 8,
                available: buf.len(),
            });
        }
        let v = value | 0xC000_0000_0000_0000;
        buf[..8].copy_from_slice(&v.to_be_bytes());
        Ok(8)
    }
}

/// Decode a QUIC-style VarInt from `buf`.
///
/// Returns `(value, bytes_consumed)`.
///
/// # Errors
/// - `WireError::Incomplete` if `buf` is too short.
#[inline]
pub fn decode(buf: &[u8]) -> Result<(u64, usize), WireError> {
    if buf.is_empty() {
        return Err(WireError::Incomplete {
            needed_min: 1,
            available: 0,
        });
    }

    let prefix = buf[0] >> 6;
    let len = 1usize << prefix; // 1, 2, 4, or 8

    if buf.len() < len {
        return Err(WireError::Incomplete {
            needed_min: len,
            available: buf.len(),
        });
    }

    let value = match len {
        1 => (buf[0] & 0x3F) as u64,
        2 => {
            let mut bytes = [0u8; 2];
            bytes.copy_from_slice(&buf[..2]);
            bytes[0] &= 0x3F;
            u16::from_be_bytes(bytes) as u64
        }
        4 => {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&buf[..4]);
            bytes[0] &= 0x3F;
            u32::from_be_bytes(bytes) as u64
        }
        8 => {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&buf[..8]);
            bytes[0] &= 0x3F;
            u64::from_be_bytes(bytes)
        }
        _ => unreachable!(),
    };

    Ok((value, len))
}

/// Returns the number of bytes needed to encode `value` as a VarInt.
#[inline]
pub const fn encoded_len(value: u64) -> usize {
    if value <= 63 {
        1
    } else if value <= 16_383 {
        2
    } else if value <= 1_073_741_823 {
        4
    } else {
        8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_1byte() {
        let mut buf = [0u8; 8];
        for v in [0, 1, 42, 63] {
            let n = encode(v, &mut buf).unwrap();
            assert_eq!(n, 1, "value {v} should encode in 1 byte");
            let (decoded, consumed) = decode(&buf[..n]).unwrap();
            assert_eq!(decoded, v);
            assert_eq!(consumed, 1);
        }
    }

    #[test]
    fn roundtrip_2bytes() {
        let mut buf = [0u8; 8];
        for v in [64, 100, 15293, 16383] {
            let n = encode(v, &mut buf).unwrap();
            assert_eq!(n, 2, "value {v} should encode in 2 bytes");
            let (decoded, consumed) = decode(&buf[..n]).unwrap();
            assert_eq!(decoded, v);
            assert_eq!(consumed, 2);
        }
    }

    #[test]
    fn roundtrip_4bytes() {
        let mut buf = [0u8; 8];
        for v in [16384, 100_000, 1_073_741_823] {
            let n = encode(v, &mut buf).unwrap();
            assert_eq!(n, 4, "value {v} should encode in 4 bytes");
            let (decoded, consumed) = decode(&buf[..n]).unwrap();
            assert_eq!(decoded, v);
            assert_eq!(consumed, 4);
        }
    }

    #[test]
    fn roundtrip_8bytes() {
        let mut buf = [0u8; 8];
        for v in [1_073_741_824, u64::MAX >> 2, VARINT_MAX] {
            let n = encode(v, &mut buf).unwrap();
            assert_eq!(n, 8, "value {v} should encode in 8 bytes");
            let (decoded, consumed) = decode(&buf[..n]).unwrap();
            assert_eq!(decoded, v);
            assert_eq!(consumed, 8);
        }
    }

    #[test]
    fn encoded_len_matches() {
        assert_eq!(encoded_len(0), 1);
        assert_eq!(encoded_len(63), 1);
        assert_eq!(encoded_len(64), 2);
        assert_eq!(encoded_len(16383), 2);
        assert_eq!(encoded_len(16384), 4);
        assert_eq!(encoded_len(1_073_741_823), 4);
        assert_eq!(encoded_len(1_073_741_824), 8);
    }

    #[test]
    fn overflow_rejected() {
        let mut buf = [0u8; 8];
        assert_eq!(
            encode(VARINT_MAX + 1, &mut buf),
            Err(WireError::InvalidVarInt)
        );
    }

    #[test]
    fn buffer_too_small() {
        let mut buf = [0u8; 1];
        assert!(matches!(
            encode(15293, &mut buf),
            Err(WireError::BufferTooSmall { needed: 2, .. })
        ));
    }

    #[test]
    fn incomplete_decode() {
        let buf = [0x40]; // prefix 01 means 2 bytes, but only 1 available
        assert!(matches!(
            decode(&buf),
            Err(WireError::Incomplete {
                needed_min: 2,
                available: 1
            })
        ));
    }

    #[test]
    fn empty_decode() {
        assert!(matches!(
            decode(&[]),
            Err(WireError::Incomplete {
                needed_min: 1,
                available: 0
            })
        ));
    }

    #[test]
    fn boundary_values() {
        let mut buf = [0u8; 8];
        // Test exact boundary transitions
        let boundaries = [63, 64, 16383, 16384, 1_073_741_823, 1_073_741_824];
        for v in boundaries {
            let n = encode(v, &mut buf).unwrap();
            let (decoded, _) = decode(&buf[..n]).unwrap();
            assert_eq!(decoded, v, "boundary value {v} failed roundtrip");
        }
    }
}
