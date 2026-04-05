//! Ed25519 identity management for PO nodes.
//!
//! Every PO node has a persistent Ed25519 keypair. The `NodeId` is derived
//! from the SHA-256 hash of the public key, giving a unique 32-byte identifier.

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;

/// A 32-byte node identifier derived from the SHA-256 of the Ed25519 public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    /// Derive a `NodeId` from an Ed25519 public key.
    pub fn from_public_key(pubkey: &VerifyingKey) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(pubkey.as_bytes());
        let hash = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&hash);
        Self(id)
    }

    /// Display as a short hex string (first 8 chars).
    pub fn short(&self) -> String {
        hex_encode(&self.0[..4])
    }

    /// Display as full hex string.
    pub fn to_hex(&self) -> String {
        hex_encode(&self.0)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short())
    }
}

/// A PO node's cryptographic identity (Ed25519 keypair).
pub struct Identity {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    node_id: NodeId,
}

impl Identity {
    /// Generate a new random identity.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let node_id = NodeId::from_public_key(&verifying_key);
        Self { signing_key, verifying_key, node_id }
    }

    /// Reconstruct identity from raw Ed25519 secret key bytes (32 bytes).
    pub fn from_bytes(secret: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(secret);
        let verifying_key = signing_key.verifying_key();
        let node_id = NodeId::from_public_key(&verifying_key);
        Self { signing_key, verifying_key, node_id }
    }

    /// Get the node's unique identifier.
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    /// Get the Ed25519 public key (for sharing with peers).
    pub fn public_key(&self) -> &VerifyingKey {
        &self.verifying_key
    }

    /// Get the raw public key bytes (32 bytes).
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the raw secret key bytes (32 bytes). Handle with care.
    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign arbitrary data with this node's Ed25519 key.
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }

    /// Verify a signature against a public key.
    pub fn verify(pubkey: &VerifyingKey, data: &[u8], signature: &Signature) -> bool {
        pubkey.verify(data, signature).is_ok()
    }
}

/// Simple hex encoding without external deps.
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_identity() {
        let id = Identity::generate();
        assert_eq!(id.node_id().0.len(), 32);
        assert!(!id.node_id().short().is_empty());
    }

    #[test]
    fn identity_from_bytes_deterministic() {
        let id1 = Identity::generate();
        let secret = id1.secret_key_bytes();
        let id2 = Identity::from_bytes(&secret);
        assert_eq!(id1.node_id(), id2.node_id());
        assert_eq!(id1.public_key_bytes(), id2.public_key_bytes());
    }

    #[test]
    fn sign_and_verify() {
        let id = Identity::generate();
        let data = b"Protocol Orzatty";
        let sig = id.sign(data);
        assert!(Identity::verify(id.public_key(), data, &sig));
    }

    #[test]
    fn wrong_data_fails_verify() {
        let id = Identity::generate();
        let sig = id.sign(b"correct data");
        assert!(!Identity::verify(id.public_key(), b"wrong data", &sig));
    }

    #[test]
    fn wrong_key_fails_verify() {
        let id1 = Identity::generate();
        let id2 = Identity::generate();
        let sig = id1.sign(b"test");
        assert!(!Identity::verify(id2.public_key(), b"test", &sig));
    }

    #[test]
    fn node_id_display() {
        let id = Identity::generate();
        let short = id.node_id().short();
        let full = id.node_id().to_hex();
        assert_eq!(short.len(), 8);
        assert_eq!(full.len(), 64);
    }
}
