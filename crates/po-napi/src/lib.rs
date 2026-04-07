#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use po_node::Po;
use std::sync::Arc;
use tokio::sync::Mutex;

#[napi]
pub struct PoClient {
    inner: Arc<Mutex<Po>>,
    node_id: String,
}

#[napi]
impl PoClient {
    /// Bind to a local port to accept incoming connections.
    #[napi(factory)]
    pub async fn bind(port: u32) -> Result<PoClient> {
        let po = Po::bind(port as u16)
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("Bind error: {e:?}")))?;

        let node_id = po.node_id();
        Ok(PoClient {
            inner: Arc::new(Mutex::new(po)),
            node_id,
        })
    }

    /// Connect to a remote PO node.
    #[napi(factory)]
    pub async fn connect(address: String) -> Result<PoClient> {
        let po = Po::connect(&address)
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("Connect error: {e:?}")))?;

        let node_id = po.node_id();
        Ok(PoClient {
            inner: Arc::new(Mutex::new(po)),
            node_id,
        })
    }

    /// Get the cryptographic Node ID of this client.
    #[napi(getter)]
    pub fn get_node_id(&self) -> String {
        self.node_id.clone()
    }

    /// Send a Buffer of data to the peer.
    #[napi]
    pub async fn send(&self, data: Buffer) -> Result<()> {
        let mut po = self.inner.lock().await;
        po.send(&data)
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("Send error: {e:?}")))?;
        Ok(())
    }

    /// Receive a Buffer of data from the peer.
    /// Returns undefined if the stream is closed gracefully.
    #[napi]
    pub async fn recv(&self) -> Result<Option<Buffer>> {
        let mut po = self.inner.lock().await;
        match po.recv().await {
            Ok(Some((_channel, data))) => Ok(Some(data.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::new(
                Status::GenericFailure,
                format!("Recv error: {e:?}"),
            )),
        }
    }

    /// Send multiple messages as a single encrypted batch.
    ///
    /// This amortizes crypto overhead: one encrypt call for the entire
    /// batch instead of one per message. Use this for high-throughput
    /// workloads to exceed 10k msg/sec.
    #[napi]
    pub async fn send_batch(&self, messages: Vec<Buffer>) -> Result<()> {
        let mut po = self.inner.lock().await;
        let slices: Vec<&[u8]> = messages.iter().map(|b| b.as_ref()).collect();
        po.send_batch(&slices)
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("SendBatch error: {e:?}")))?;
        Ok(())
    }

    /// Gracefully close the connection.
    #[napi]
    pub async fn close(&self) -> Result<()> {
        let mut po = self.inner.lock().await;
        po.close()
            .await
            .map_err(|e| Error::new(Status::GenericFailure, format!("Close error: {e:?}")))?;
        Ok(())
    }
}
