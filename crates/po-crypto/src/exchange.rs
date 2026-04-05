//! X25519 Elliptic-Curve Diffie-Hellman key exchange for PO sessions.
//!
//! Each connection generates ephemeral X25519 keypairs. The shared secret
//! is derived via ECDH and then passed through HKDF-SHA256 to produce
//! the session encryption key. This provides Perfect Forward Secrecy (PFS).

use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use hkdf::Hkdf;
use sha2::Sha256;
use rand::rngs::OsRng;
use zeroize::Zeroize;

use crate::error::CryptoError;

/// The length of a session key in bytes (256-bit for ChaCha20-Poly1305).
pub const SESSION_KEY_LEN: usize = 32;

/// An ephemeral X25519 keypair for a single session.
pub struct EphemeralKeypair {
    secret: Option<EphemeralSecret>,
    public: PublicKey,
}

impl EphemeralKeypair {
    /// Generate a new ephemeral keypair using the OS CSPRNG.
    pub fn generate() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self {
            secret: Some(secret),
            public,
        }
    }

    /// Get the public key bytes to send to the peer (32 bytes).
    pub fn public_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }

    /// Perform the Diffie-Hellman exchange with the peer's public key and
    /// derive a session key via HKDF-SHA256.
    ///
    /// This **consumes** the ephemeral secret — it can only be used once.
    ///
    /// # Arguments
    /// * `peer_public` - The peer's ephemeral X25519 public key (32 bytes).
    /// * `context` - Additional context bytes for HKDF (e.g., both node IDs concatenated).
    pub fn derive_session_key(
        mut self,
        peer_public: &[u8; 32],
        context: &[u8],
    ) -> Result<SessionKey, CryptoError> {
        let secret = self.secret.take().ok_or_else(|| {
            CryptoError::KeyGeneration("ephemeral secret already consumed".into())
        })?;

        let peer_pk = PublicKey::from(*peer_public);
        let shared_secret = secret.diffie_hellman(&peer_pk);

        // Derive session key via HKDF-SHA256
        let hk = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
        let mut session_key = [0u8; SESSION_KEY_LEN];
        hk.expand(context, &mut session_key).map_err(|e| {
            CryptoError::KeyGeneration(format!("HKDF expand failed: {e}"))
        })?;

        Ok(SessionKey(session_key))
    }
}

/// A static X25519 keypair (for cases where the key persists across sessions,
/// e.g., testing or relay nodes with pre-shared keys).
pub struct StaticKeypair {
    secret: StaticSecret,
    public: PublicKey,
}

impl StaticKeypair {
    /// Generate a new static keypair.
    pub fn generate() -> Self {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    /// Get the public key bytes.
    pub fn public_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }

    /// Perform ECDH and derive a session key. The static secret is NOT consumed.
    pub fn derive_session_key(
        &self,
        peer_public: &[u8; 32],
        context: &[u8],
    ) -> Result<SessionKey, CryptoError> {
        let peer_pk = PublicKey::from(*peer_public);
        let shared_secret = self.secret.diffie_hellman(&peer_pk);

        let hk = Hkdf::<Sha256>::new(None, shared_secret.as_bytes());
        let mut session_key = [0u8; SESSION_KEY_LEN];
        hk.expand(context, &mut session_key).map_err(|e| {
            CryptoError::KeyGeneration(format!("HKDF expand failed: {e}"))
        })?;

        Ok(SessionKey(session_key))
    }
}

/// A derived 256-bit session key for symmetric encryption.
///
/// Automatically zeroized from memory when dropped.
pub struct SessionKey([u8; SESSION_KEY_LEN]);

impl SessionKey {
    /// Get the raw key bytes.
    pub fn as_bytes(&self) -> &[u8; SESSION_KEY_LEN] {
        &self.0
    }
}

impl Drop for SessionKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_key_exchange() {
        // Simulate Alice and Bob
        let alice = EphemeralKeypair::generate();
        let bob = EphemeralKeypair::generate();

        let alice_pub = alice.public_bytes();
        let bob_pub = bob.public_bytes();

        let context = b"po-session-v1";

        // Both derive the same session key
        let alice_key = alice.derive_session_key(&bob_pub, context).unwrap();
        let bob_key = bob.derive_session_key(&alice_pub, context).unwrap();

        assert_eq!(alice_key.as_bytes(), bob_key.as_bytes());
    }

    #[test]
    fn different_peers_different_keys() {
        let alice = EphemeralKeypair::generate();
        let bob = EphemeralKeypair::generate();
        let eve = EphemeralKeypair::generate();

        let alice_pub = alice.public_bytes();
        let eve_pub = eve.public_bytes();
        let context = b"po-session-v1";

        let bob_alice_key = bob.derive_session_key(&alice_pub, context).unwrap();
        let _eve_key = eve.derive_session_key(&alice_pub, context);
        // Eve can derive a key with Alice, but it won't match Bob's
        // (Because Bob used a different ephemeral key)
        // alice's secret is consumed by bob's derivation — but in a real setup each party
        // has their own ephemeral secret
        let alice_eve_key = alice.derive_session_key(&eve_pub, context).unwrap();
        assert_ne!(bob_alice_key.as_bytes(), alice_eve_key.as_bytes());
    }

    #[test]
    fn static_key_exchange() {
        let alice = StaticKeypair::generate();
        let bob = StaticKeypair::generate();

        let context = b"po-static-v1";

        let key_a = alice.derive_session_key(&bob.public_bytes(), context).unwrap();
        let key_b = bob.derive_session_key(&alice.public_bytes(), context).unwrap();

        assert_eq!(key_a.as_bytes(), key_b.as_bytes());
    }

    #[test]
    fn context_affects_key() {
        let alice = StaticKeypair::generate();
        let bob = StaticKeypair::generate();

        let key1 = alice.derive_session_key(&bob.public_bytes(), b"context-1").unwrap();
        let key2 = alice.derive_session_key(&bob.public_bytes(), b"context-2").unwrap();

        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }
}
