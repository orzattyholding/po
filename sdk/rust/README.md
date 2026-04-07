# PO for Rust — Protocol Orzatty Core SDK

The native, zero-overhead SDK. This is the language PO is built in.

## Installation

```toml
# Cargo.toml
[dependencies]
po-core = { git = "https://github.com/orzattyholding/po.git" }
tokio = { version = "1.0", features = ["full"] }
```

## Quick Start

```rust
use po_node::Po;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Server
    let mut server = Po::bind(4433).await?;

    // Client
    let mut client = Po::connect("127.0.0.1:4433").await?;

    // Send E2EE encrypted data
    client.send(b"Rust native payload").await?;

    // Receive
    let data = server.recv().await?;
    Ok(())
}
```

## Batch API (>10k msg/s)

```rust
// Send 1000 messages in a single encrypted frame
let messages: Vec<&[u8]> = vec![b"msg"; 1000];
client.send_batch(&messages).await?;
```

---

*Built by [Orzatty Corporation](https://orzatty.com)*
