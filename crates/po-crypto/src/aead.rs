//! ChaCha20-Poly1305 AEAD encryption for PO frame payloads.
//!
//! Provides authenticated encryption with associated data (AEAD) using
//! the ChaCha20-Poly1305 construction (RFC 8439). Each encrypted frame
//! includes a 12-byte nonce (derived from a counter) and a 16-byte
//! authentication tag.
//!
//! ## Overhead per encrypted frame
//! - **Nonce**: 12 bytes (prepended to ciphertext)
//! - **Auth tag**: 16 bytes (appended by ChaCha20-Poly1305)
//! - **Total**: 28 bytes of overhead per frame

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Nonce,
};
use zeroize::Zeroize;
use crate::error::CryptoError;
use crate::exchange::SESSION_KEY_LEN;

/// Overhead added by encryption: 12-byte nonce + 16-byte auth tag.
pub const ENCRYPTION_OVERHEAD: usize = 12 + 16;

/// A session cipher that encrypts/decrypts frame payloads.
///
/// Uses a monotonic nonce counter to ensure nonce uniqueness.
/// The nonce is prepended to the ciphertext so the receiver can extract it.
pub struct SessionCipher {
    cipher: ChaCha20Poly1305,
    nonce_counter: u64,
}

impl SessionCipher {
    /// Create a new session cipher from a derived session key.
    pub fn new(session_key: &[u8; SESSION_KEY_LEN]) -> Self {
        let cipher = ChaCha20Poly1305::new_from_slice(session_key)
            .expect("session key is always 32 bytes");
        Self {
            cipher,
            nonce_counter: 0,
        }
    }

    /// Encrypt a plaintext payload.
    ///
    /// The frame header bytes can be passed as `aad` (Associated Authenticated Data)
    /// to bind the ciphertext to a specific header — preventing header tampering.
    ///
    /// Returns `nonce || ciphertext || tag` (12 + plaintext.len() + 16 bytes).
    pub fn encrypt(&mut self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce_bytes = self.next_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let payload = Payload {
            msg: plaintext,
            aad,
        };

        let ciphertext = self.cipher.encrypt(nonce, payload).map_err(|e| {
            CryptoError::Encrypt(format!("ChaCha20-Poly1305 encrypt failed: {e}"))
        })?;

        // Pre-allocate final buffer: nonce(12) + ciphertext(plaintext.len() + 16 tag)
        // Single allocation, no intermediate Vec.
        let mut output = Vec::with_capacity(12 + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    /// Decrypt a payload that was encrypted by `encrypt()`.
    ///
    /// Expects the input to be `nonce (12) || ciphertext || tag (16)`.
    /// The same `aad` used during encryption must be provided.
    pub fn decrypt(&self, encrypted: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if encrypted.len() < ENCRYPTION_OVERHEAD {
            return Err(CryptoError::Decrypt(format!(
                "ciphertext too short: {} bytes (minimum {})",
                encrypted.len(),
                ENCRYPTION_OVERHEAD
            )));
        }

        let (nonce_bytes, ciphertext) = encrypted.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let payload = Payload {
            msg: ciphertext,
            aad,
        };

        self.cipher.decrypt(nonce, payload).map_err(|_| {
            CryptoError::Decrypt("decryption failed: invalid key, corrupted data, or tampered ciphertext".into())
        })
    }

    /// Generate the next 12-byte nonce from the monotonic counter.
    ///
    /// Layout: 4 zero bytes + 8-byte little-endian counter.
    fn next_nonce(&mut self) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[4..12].copy_from_slice(&self.nonce_counter.to_le_bytes());
        self.nonce_counter += 1;
        nonce
    }

    /// Get the current nonce counter value (useful for debugging).
    pub fn nonce_counter(&self) -> u64 {
        self.nonce_counter
    }
}

impl Drop for SessionCipher {
    fn drop(&mut self) {
        // Zeroize the counter; the cipher key is internal to ChaCha20Poly1305
        self.nonce_counter.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        key[0] = 0x42;
        key[31] = 0xFF;
        key
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = test_key();
        let mut encryptor = SessionCipher::new(&key);
        let decryptor = SessionCipher::new(&key);

        let plaintext = b"Hello from Protocol Orzatty!";
        let aad = b"frame-header-bytes";

        let encrypted = encryptor.encrypt(plaintext, aad).unwrap();
        assert_eq!(encrypted.len(), 12 + plaintext.len() + 16);

        let decrypted = decryptor.decrypt(&encrypted, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let mut encryptor = SessionCipher::new(&test_key());
        let wrong_key = [0xAA; 32];
        let decryptor = SessionCipher::new(&wrong_key);

        let encrypted = encryptor.encrypt(b"secret", b"").unwrap();
        assert!(decryptor.decrypt(&encrypted, b"").is_err());
    }

    #[test]
    fn wrong_aad_fails() {
        let key = test_key();
        let mut encryptor = SessionCipher::new(&key);
        let decryptor = SessionCipher::new(&key);

        let encrypted = encryptor.encrypt(b"data", b"header-v1").unwrap();
        assert!(decryptor.decrypt(&encrypted, b"header-TAMPERED").is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = test_key();
        let mut encryptor = SessionCipher::new(&key);
        let decryptor = SessionCipher::new(&key);

        let mut encrypted = encryptor.encrypt(b"important data", b"").unwrap();
        // Flip a byte in the ciphertext (after the 12-byte nonce)
        encrypted[15] ^= 0xFF;
        assert!(decryptor.decrypt(&encrypted, b"").is_err());
    }

    #[test]
    fn nonce_counter_increments() {
        let mut cipher = SessionCipher::new(&test_key());
        assert_eq!(cipher.nonce_counter(), 0);
        cipher.encrypt(b"a", b"").unwrap();
        assert_eq!(cipher.nonce_counter(), 1);
        cipher.encrypt(b"b", b"").unwrap();
        assert_eq!(cipher.nonce_counter(), 2);
    }

    #[test]
    fn too_short_ciphertext() {
        let cipher = SessionCipher::new(&test_key());
        assert!(cipher.decrypt(&[0u8; 10], b"").is_err());
    }

    #[test]
    fn empty_plaintext() {
        let key = test_key();
        let mut encryptor = SessionCipher::new(&key);
        let decryptor = SessionCipher::new(&key);

        let encrypted = encryptor.encrypt(b"", b"").unwrap();
        assert_eq!(encrypted.len(), ENCRYPTION_OVERHEAD); // just nonce + tag
        let decrypted = decryptor.decrypt(&encrypted, b"").unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn large_payload() {
        let key = test_key();
        let mut encryptor = SessionCipher::new(&key);
        let decryptor = SessionCipher::new(&key);

        let payload = vec![0xAB; 1_000_000]; // 1MB
        let encrypted = encryptor.encrypt(&payload, b"big").unwrap();
        let decrypted = decryptor.decrypt(&encrypted, b"big").unwrap();
        assert_eq!(decrypted, payload);
    }
}
