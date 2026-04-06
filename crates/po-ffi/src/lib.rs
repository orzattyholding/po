uniffi::include_scaffolding!("po");

use po_node::Po;
use tokio::runtime::Runtime;

use std::fmt;

#[derive(Debug)]
pub enum PoFfiError {
    Config(String),
    Transport(String),
    Handshake(String),
    Session(String),
    Generic(String),
}

impl fmt::Display for PoFfiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "Config error: {}", msg),
            Self::Transport(msg) => write!(f, "Transport error: {}", msg),
            Self::Handshake(msg) => write!(f, "Handshake error: {}", msg),
            Self::Session(msg) => write!(f, "Session error: {}", msg),
            Self::Generic(msg) => write!(f, "Generic error: {}", msg),
        }
    }
}

impl std::error::Error for PoFfiError {}

pub struct PoClient {
    inner: std::sync::Mutex<Po>,
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

        Ok(Self { inner: std::sync::Mutex::new(inner), rt })
    }

    pub fn node_id(&self) -> String {
        self.inner.lock().unwrap().node_id()
    }

    pub fn send(&self, data: Vec<u8>) -> Result<(), PoFfiError> {
        self.rt.block_on(async {
            self.inner.lock().unwrap().send(&data).await
        }).map_err(|e| match e {
            po_node::node::PoError::Session(msg) => PoFfiError::Session(msg),
            _ => PoFfiError::Generic(e.to_string()),
        })
    }

    pub fn recv(&self) -> Result<Option<Vec<u8>>, PoFfiError> {
        self.rt.block_on(async {
            self.inner.lock().unwrap().recv().await
                .map(|opt| opt.map(|(_channel, packet)| packet))
        }).map_err(|e| match e {
            po_node::node::PoError::Session(msg) => PoFfiError::Session(msg),
            _ => PoFfiError::Generic(e.to_string()),
        })
    }

    pub fn close(&self) -> Result<(), PoFfiError> {
        self.rt.block_on(async {
            self.inner.lock().unwrap().close().await
        }).map_err(|e| match e {
            po_node::node::PoError::Session(msg) => PoFfiError::Session(msg),
            _ => PoFfiError::Generic(e.to_string()),
        })
    }
}
