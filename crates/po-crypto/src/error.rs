//! Cryptographic error types for PO.

use std::fmt;

/// Errors from cryptographic operations.
#[derive(Debug)]
pub enum CryptoError {
    /// Key generation failed.
    KeyGeneration(String),
    /// Encryption failed.
    Encrypt(String),
    /// Decryption failed (bad key, corrupted data, or tampered ciphertext).
    Decrypt(String),
    /// Signature verification failed.
    InvalidSignature,
    /// The handshake message is malformed or has unexpected length.
    MalformedHandshake(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyGeneration(e) => write!(f, "key generation failed: {e}"),
            Self::Encrypt(e) => write!(f, "encryption failed: {e}"),
            Self::Decrypt(e) => write!(f, "decryption failed: {e}"),
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::MalformedHandshake(e) => write!(f, "malformed handshake: {e}"),
        }
    }
}

impl std::error::Error for CryptoError {}
