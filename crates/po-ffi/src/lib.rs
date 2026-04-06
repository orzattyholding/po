uniffi::include_scaffolding!("po");

use po_node::Po;
use tokio::runtime::Runtime;

#[derive(Debug, thiserror::Error)]
pub enum PoFfiError {
    #[error("Config error: {0}")]
    Config(String),
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Handshake error: {0}")]
    Handshake(String),
    #[error("Session error: {0}")]
    Session(String),
    #[error("Generic error: {0}")]
    Generic(String),
}

pub struct PoClient {
    inner: Po,
    rt: Runtime,
}

impl PoClient {
    pub fn new(bind_address_or_port: String, remote_address: Option<String>) -> Result<Self, PoFfiError> {
        let rt = Runtime::new().map_err(|e| PoFfiError::Config(e.to_string()))?;
        
        let inner = rt.block_on(async {
            if let Some(remote) = remote_address {
                // Client mode
                // Note: current Po API doesn't allow explicit bind + connect easily,
                // Po::connect uses ephemeral ports. Since PO is a test bed, 
                // we'll just connect if remote_addr is provided.
                Po::connect(&remote).await
            } else {
                // Server mode
                let port: u16 = bind_address_or_port.parse().map_err(|_| po_node::node::PoError::Config("Invalid port".to_string()))?;
                Po::bind(port).await
            }
        }).map_err(|e| match e {
            po_node::node::PoError::Config(msg) => PoFfiError::Config(msg),
            po_node::node::PoError::Transport(msg) => PoFfiError::Transport(msg),
            po_node::node::PoError::Handshake(msg) => PoFfiError::Handshake(msg),
            po_node::node::PoError::Session(msg) => PoFfiError::Session(msg),
        })?;

        Ok(Self { inner, rt })
    }

    pub fn node_id(&self) -> String {
        self.inner.node_id()
    }

    pub fn send(&self, data: Vec<u8>) -> Result<(), PoFfiError> {
        self.rt.block_on(async {
            self.inner.send(&data).await
        }).map_err(|e| match e {
            po_node::node::PoError::Session(msg) => PoFfiError::Session(msg),
            _ => PoFfiError::Generic(e.to_string()),
        })
    }

    pub fn recv(&self) -> Result<Option<Vec<u8>>, PoFfiError> {
        self.rt.block_on(async {
            self.inner.recv().await
                .map(|opt| opt.map(|(_channel, packet)| packet))
        }).map_err(|e| match e {
            po_node::node::PoError::Session(msg) => PoFfiError::Session(msg),
            _ => PoFfiError::Generic(e.to_string()),
        })
    }

    pub fn close(&mut self) -> Result<(), PoFfiError> {
        self.rt.block_on(async {
            self.inner.close().await
        }).map_err(|e| match e {
            po_node::node::PoError::Session(msg) => PoFfiError::Session(msg),
            _ => PoFfiError::Generic(e.to_string()),
        })
    }
}
