<p align="center">
  <strong>Protocol Orzatty (PO)</strong><br/>
  <em>Encrypted peer-to-peer communication over QUIC</em>
</p>

<p align="center">
  <a href="https://orzatty.com">Website</a> ·
  <a href="./WHITEPAPER.md">White Paper</a> ·
  <a href="#quick-start">Quick Start</a> ·
  <a href="#architecture">Architecture</a>
</p>

---

## What is PO?

**Protocol Orzatty (PO)** is an end-to-end encrypted (E2EE) communication protocol for direct, peer-to-peer data transfer over UDP. It wraps QUIC transport with a custom cryptographic handshake, binary wire format, and session management layer.

Every byte on the wire is encrypted. There is no plaintext mode.

**Key properties:**

- 🔒 **Mandatory E2EE** — ChaCha20-Poly1305 on every data frame
- 🆔 **Decentralized identity** — Ed25519 keypairs, no certificate authority needed
- 🔑 **Perfect forward secrecy** — Ephemeral X25519 ECDH per session
- ⚡ **QUIC/UDP transport** — No TCP head-of-line blocking
- 📦 **Compact wire format** — 4-byte minimum header, `no_std` compatible
- 🎯 **2-line API** — Connect or listen with a single function call

---

## Quick Start

### As a library

```rust
use po_node::Po;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Server — 2 lines to encrypted connection
    let mut server = Po::bind(4433).await?;
    let (channel, data) = server.recv().await?.unwrap();
    println!("Received: {}", String::from_utf8_lossy(&data));

    Ok(())
}
```

```rust
use po_node::Po;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Client — connect and send
    let mut client = Po::connect("192.168.1.5:4433").await?;
    client.send(b"Hello, encrypted world!").await?;
    client.close().await?;

    Ok(())
}
```

### CLI Usage

```bash
# Build the CLI
cargo build --release -p po-cli

# Listen for connections
./target/release/po-cli listen --port 4433

# Connect to a peer
./target/release/po-cli connect 192.168.1.5:4433

# Interactive encrypted chat
./target/release/po-cli chat 4433          # Listen mode
./target/release/po-cli chat 10.0.0.5:4433  # Connect mode

# Show node identity
./target/release/po-cli identity
```

---

## Architecture

PO is a modular stack of five Rust crates:

```
┌──────────────────────────────────────────┐
│               po-node                    │  High-level API
│         Po::connect() / Po::bind()       │  (po-node)
├──────────────────────────────────────────┤
│              po-session                  │  Handshake + encryption
│   Framer · Handshake · State Machine     │  (po-session)
├──────────────────────────────────────────┤
│             po-transport                 │  QUIC streams
│      QuicTransport · QuicListener        │  (po-transport)
├──────────────────────────────────────────┤
│              po-crypto                   │  Cryptographic primitives
│  Ed25519 · X25519 · ChaCha20-Poly1305   │  (po-crypto)
├──────────────────────────────────────────┤
│               po-wire                    │  Binary codec (no_std)
│     FrameHeader · VarInt · FrameType     │  (po-wire)
└──────────────────────────────────────────┘
```

### Crate Responsibilities

| Crate | Role | Dependencies |
|-------|------|-------------|
| `po-wire` | Binary frame encoding/decoding with QUIC-style VarInts | **Zero** (pure `no_std` Rust) |
| `po-crypto` | Ed25519 identity, X25519 key exchange, ChaCha20-Poly1305 AEAD | `ed25519-dalek`, `x25519-dalek`, `chacha20poly1305` |
| `po-transport` | Abstract transport trait + QUIC implementation via Quinn | `quinn`, `rustls`, `tokio` |
| `po-session` | 3-way handshake, frame I/O, session state machine | `po-wire`, `po-crypto`, `po-transport` |
| `po-node` | Public API: `Po::connect()`, `Po::bind()`, `send()`, `recv()` | All of the above |

---

## How It Works

### Connection Flow

```
1. QUIC/UDP connection established
2. 3-way cryptographic handshake:
   a. Initiator → Responder: Ed25519 pubkey + ephemeral X25519 key + signature
   b. Responder → Initiator: Ed25519 pubkey + ephemeral X25519 key + signature
   c. Initiator → Responder: Encrypted "PO_READY" confirmation
3. Session key derived (ECDH → HKDF-SHA256)
4. All subsequent data encrypted with ChaCha20-Poly1305
```

### Wire Format

Every frame starts with a compact binary header (minimum 4 bytes):

```
Byte 0: [CTL|PRI|ENC|RSV|  FrameType  ]
         VarInt: Channel ID
         VarInt: Stream ID
         VarInt: Payload Length
```

### Encryption Overhead

| Component | Bytes |
|-----------|-------|
| Frame header | 4–25 |
| Nonce (per frame) | 12 |
| Auth tag (per frame) | 16 |
| **Total overhead** | **32–53 bytes** |

---

## Supported Frame Types

| Type | ID | Description |
|------|-----|-------------|
| DATA | 0x00 | Application payload |
| HS_INIT | 0x01 | Handshake initiation |
| HS_REPLY | 0x02 | Handshake response |
| HS_COMPLETE | 0x03 | Handshake confirmation |
| PING | 0x04 | Keep-alive |
| PONG | 0x05 | Keep-alive response |
| CLOSE | 0x06 | Graceful disconnect |
| FILE_HDR | 0x07 | File transfer metadata |
| FILE_CHUNK | 0x08 | File data segment |
| ACK | 0x09 | Acknowledgement |

---

## Building

### Requirements

- Rust 1.75+ (2021 edition)
- No system dependencies beyond what Cargo resolves

### Commands

```bash
# Build everything
cargo build --release

# Run all tests
cargo test --workspace

# Build CLI only
cargo build --release -p po-cli

# Run benchmarks (PO vs WebSocket)
cargo run --release -p po-bench
```

### Cross-compilation (Linux from Windows)

```bash
# In WSL2 (Ubuntu), navigate to the project via /mnt/
cd /mnt/c/path/to/PO
cargo build --release -p po-cli
# Outputs native ELF binary at target/release/po-cli
```

---

## Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| QUIC port | `4433` | Data transport port |
| Discovery port | `5433` | LAN peer discovery (UDP broadcast) |
| Max frame size | 10 MB | Maximum payload per frame |
| ALPN protocol | `po/1` | QUIC application layer protocol negotiation ID |
| Timestamp tolerance | ±30s | Handshake replay protection window |

---

## Security

### Cryptographic Primitives

| Purpose | Algorithm | Standard |
|---------|-----------|----------|
| Node identity | Ed25519 | RFC 8032 |
| Key exchange | X25519 ECDH | RFC 7748 |
| Key derivation | HKDF-SHA256 | RFC 5869 |
| Symmetric encryption | ChaCha20-Poly1305 | RFC 8439 |
| Node ID | SHA-256(pubkey) | FIPS 180-4 |
| VarInt encoding | QUIC VarInt | RFC 9000 §16 |

### Security Properties

- ✅ End-to-end encryption (mandatory, no opt-out)
- ✅ Perfect forward secrecy (ephemeral keys, zeroized after use)
- ✅ Mutual authentication (both peers sign during handshake)
- ✅ Replay protection (timestamped handshake)
- ✅ Header integrity (frame header used as AEAD associated data)
- ✅ Memory safety (Rust, no `unsafe` in protocol logic)

---

## Project Structure

```
PO/
├── Cargo.toml              # Workspace root
├── WHITEPAPER.md           # Technical white paper
├── README.md               # This file
├── crates/
│   ├── po-wire/            # Binary wire format (no_std)
│   ├── po-crypto/          # Cryptographic primitives
│   ├── po-transport/       # QUIC transport + abstract trait
│   ├── po-session/         # Handshake + session management
│   └── po-node/            # High-level API
├── po-cli/                 # Command-line interface
├── benches/
│   └── po-bench/           # PO vs WebSocket benchmarks
└── tests/
    └── integration/        # Integration tests
```

---

## Roadmap

- [x] Core wire format with VarInt encoding
- [x] Ed25519 identity + X25519 key exchange + ChaCha20-Poly1305
- [x] QUIC transport via Quinn
- [x] 3-way authenticated handshake with PFS
- [x] Session state machine with encrypted send/recv
- [x] CLI with listen, connect, chat, and identity commands
- [x] LAN peer discovery via UDP broadcast
- [x] In-memory transport for testing
- [ ] Key persistence and identity management
- [ ] Multi-peer connection handling
- [ ] NAT traversal / hole punching
- [ ] File transfer API with progress
- [ ] WASM/browser transport
- [ ] Protocol security audit
- [ ] crates.io publication
- [ ] npm bindings via NAPI-RS

---

## License

MIT — see [LICENSE](./LICENSE) for details.

---

**Built by [Dylan Orzatty](https://orzatty.com) · [Orzatty Holdings](https://orzatty.com)**
