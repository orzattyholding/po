//! QUIC transport implementation using Quinn.
//!
//! Provides `QuicTransport` (a single bidirectional QUIC stream wrapped as
//! `AsyncFrameTransport`) and `QuicListener` (accepts incoming connections).
//!
//! Uses self-signed certificates by default — the PO crypto layer handles
//! authentication at the application level via Ed25519 identities.

use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, info};

use crate::traits::{AsyncFrameTransport, TransportError};

/// Configuration for QUIC endpoints.
pub struct QuicConfig {
    /// Address to bind to (e.g., `0.0.0.0:4433`).
    pub bind_addr: SocketAddr,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:4433".parse().unwrap(),
        }
    }
}

/// A QUIC-based transport wrapping a single bidirectional stream.
pub struct QuicTransport {
    send: SendStream,
    recv: RecvStream,
    _connection: Connection,
}

impl QuicTransport {
    /// Wrap an existing Quinn bidirectional stream pair.
    pub fn from_streams(send: SendStream, recv: RecvStream, connection: Connection) -> Self {
        Self {
            send,
            recv,
            _connection: connection,
        }
    }

    /// Connect to a remote PO node as a client.
    pub async fn connect(addr: SocketAddr) -> Result<Self, TransportError> {
        let client_config = Self::insecure_client_config();
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse().unwrap())
            .map_err(|e| TransportError::Io(format!("bind client: {e}")))?;
        endpoint.set_default_client_config(client_config);

        debug!("Connecting to {addr}...");
        let connection = endpoint
            .connect(addr, "po-node")
            .map_err(|e| TransportError::Quic(format!("connect: {e}")))?
            .await
            .map_err(|e| TransportError::Quic(format!("handshake: {e}")))?;

        info!("QUIC connected to {addr}");
        let (send, recv) = connection
            .open_bi()
            .await
            .map_err(|e| TransportError::Quic(format!("open_bi: {e}")))?;

        Ok(Self::from_streams(send, recv, connection))
    }

    /// Create a client config that skips TLS certificate verification.
    /// PO handles authentication at the application layer via Ed25519.
    fn insecure_client_config() -> ClientConfig {
        let mut crypto = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        // Must match the server's ALPN protocol for QUIC/TLS 1.3 handshake
        crypto.alpn_protocols = vec![b"po/1".to_vec()];

        ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
                .expect("failed to create QUIC client config"),
        ))
    }
}

#[async_trait::async_trait]
impl AsyncFrameTransport for QuicTransport {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        match self.recv.read(buf).await {
            Ok(Some(n)) => Ok(n),
            Ok(None) => Err(TransportError::ConnectionClosed),
            Err(e) => Err(TransportError::Quic(format!("read: {e}"))),
        }
    }

    async fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.send
            .write_all(data)
            .await
            .map_err(|e| TransportError::Quic(format!("write: {e}")))
    }

    async fn flush(&mut self) -> Result<(), TransportError> {
        // Quinn auto-flushes, but we can explicitly finish if needed
        Ok(())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.send
            .finish()
            .map_err(|e| TransportError::Quic(format!("finish: {e}")))?;
        Ok(())
    }
}

/// A QUIC listener that accepts incoming connections.
pub struct QuicListener {
    endpoint: Endpoint,
}

impl QuicListener {
    /// Start listening on the configured address.
    pub async fn bind(config: QuicConfig) -> Result<Self, TransportError> {
        let (server_config, _cert) = Self::generate_self_signed_config()
            .map_err(|e| TransportError::Quic(format!("cert generation: {e}")))?;

        let endpoint = Endpoint::server(server_config, config.bind_addr)
            .map_err(|e| TransportError::Io(format!("bind server: {e}")))?;

        info!("PO listening on {}", config.bind_addr);
        Ok(Self { endpoint })
    }

    /// Accept the next incoming connection and return a transport for it.
    pub async fn accept(&self) -> Result<QuicTransport, TransportError> {
        let incoming = self
            .endpoint
            .accept()
            .await
            .ok_or(TransportError::ConnectionClosed)?;

        let connection = incoming
            .await
            .map_err(|e| TransportError::Quic(format!("accept handshake: {e}")))?;

        let peer = connection.remote_address();
        info!("Accepted connection from {peer}");

        let (send, recv) = connection
            .accept_bi()
            .await
            .map_err(|e| TransportError::Quic(format!("accept_bi: {e}")))?;

        Ok(QuicTransport::from_streams(send, recv, connection))
    }

    /// Get the local address the listener is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.endpoint
            .local_addr()
            .map_err(|e| TransportError::Io(format!("local_addr: {e}")))
    }

    /// Generate a self-signed certificate and server config.
    fn generate_self_signed_config(
    ) -> Result<(ServerConfig, rcgen::CertifiedKey), Box<dyn std::error::Error>> {
        let certified_key = rcgen::generate_simple_self_signed(vec!["po-node".into()])?;

        let cert_der = CertificateDer::from(certified_key.cert.der().to_vec());
        let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
            certified_key.key_pair.serialize_der(),
        ));

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)?;

        server_crypto.alpn_protocols = vec![b"po/1".to_vec()];

        let server_config = ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?,
        ));

        Ok((server_config, certified_key))
    }
}

/// Custom certificate verifier that accepts any server certificate.
/// PO authentication happens at the application layer (Ed25519 handshake).
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
