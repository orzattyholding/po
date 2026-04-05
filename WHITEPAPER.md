# Protocol Orzatty (PO) — White Paper

**Version 1.0 · April 2026**
**Author:** Dylan Orzatty · [Orzatty Holdings](https://orzatty.com)
**License:** MIT

---

## Abstract

Protocol Orzatty (PO) is a lightweight, end-to-end encrypted (E2EE) communication protocol designed for direct peer-to-peer data transfer over UDP. Built on QUIC as a transport substrate, PO implements its own cryptographic handshake, binary framing format, and session management layer to provide a complete, auditable, and dependency-minimal networking stack. The protocol targets sub-millisecond framing overhead, mandatory encryption for every byte on the wire, and an API surface simple enough to establish a secure connection in two lines of code.

---

## 1. Introduction

### 1.1 Problem Statement

Modern peer-to-peer applications face a fundamental tension: security vs. simplicity. Protocols like TLS-over-TCP provide encryption but carry decades of complexity, optional cipher negotiation, and head-of-line blocking. WebSocket — while ubiquitous — operates over TCP with no built-in encryption guarantee, and its framing format was designed for browser compatibility, not protocol efficiency.

PO resolves this by making three architectural commitments:

1. **QUIC as the sole transport** — Multiplexed UDP streams with built-in congestion control and 0-RTT reconnection, eliminating TCP head-of-line blocking.
2. **Mandatory E2EE at the application layer** — Every data frame is encrypted with ChaCha20-Poly1305. There is no "plaintext mode."
3. **A purpose-built wire format** — Compact binary headers with RFC 9000 VarInt encoding; minimum 4 bytes per frame header.

### 1.2 Design Goals

| Goal | Decision |
|---|---|
| Mandatory encryption | ChaCha20-Poly1305 AEAD on every data frame |
| Identity without central authority | Ed25519 keypairs; NodeId = SHA-256(pubkey) |
| Perfect forward secrecy | Ephemeral X25519 ECDH per session |
| Minimal wire overhead | Custom binary framing, 4–25 byte headers |
| Transport agnosticism | Abstract `AsyncFrameTransport` trait |
| Developer ergonomics | `Po::connect()` / `Po::bind()` — 2 lines to E2EE |

---

## 2. Architecture

PO is structured as a layered stack of five crates, each with a single responsibility:

```
┌──────────────────────────────────────────┐
│               po-node                    │  ← High-level API
│         Po::connect() / Po::bind()       │
├──────────────────────────────────────────┤
│              po-session                  │  ← Handshake + session state machine
│   Framer · Handshake · SessionState      │
├──────────────────────────────────────────┤
│             po-transport                 │  ← QUIC streams (AsyncFrameTransport)
│      QuicTransport · QuicListener        │
├──────────────────────────────────────────┤
│              po-crypto                   │  ← Identity · Key exchange · AEAD
│  Ed25519 · X25519 · ChaCha20-Poly1305   │
├──────────────────────────────────────────┤
│               po-wire                    │  ← Binary codec (no_std)
│     FrameHeader · VarInt · FrameType     │
└──────────────────────────────────────────┘
```

### 2.1 Dependency Direction

Dependencies flow strictly downward. `po-wire` has **zero external dependencies** and is `no_std` compatible. `po-crypto` depends only on vetted cryptographic libraries. `po-transport` introduces the async runtime (Tokio) and QUIC (Quinn). `po-session` composes crypto and transport into a state machine. `po-node` provides the public-facing API.

---

## 3. Wire Format (`po-wire`)

### 3.1 Frame Header Layout

Every PO frame begins with a compact binary header:

```text
┌─────────┬──────────────┬──────────────┬──────────────┐
│ Byte 0  │ VarInt       │ VarInt       │ VarInt       │
│ Type +  │ Channel ID   │ Stream ID    │ Payload Len  │
│ Flags   │ (1–8 bytes)  │ (1–8 bytes)  │ (1–8 bytes)  │
└─────────┴──────────────┴──────────────┴──────────────┘
```

**Byte 0** encodes both the frame type (lower 4 bits) and processing flags (upper 4 bits):

```text
┌───┬───┬───┬───┬───┬───┬───┬───┐
│ 7 │ 6 │ 5 │ 4 │ 3 │ 2 │ 1 │ 0 │
│CTL│PRI│ENC│RSV│    FrameType   │
└───┴───┴───┴───┴───┴───┴───┴───┘
```

| Bit | Name | Description |
|-----|------|-------------|
| 7 | CTL | Control frame (not application data) |
| 6 | PRI | High-priority — bypass processing queues |
| 5 | ENC | Payload encrypted with session cipher |
| 4 | RSV | Reserved for future use |
| 3–0 | Type | Frame type identifier (see §3.2) |

**Header size**: Minimum **4 bytes** (type/flags + three 1-byte VarInts), maximum **25 bytes** (type/flags + three 8-byte VarInts).

### 3.2 Frame Types

PO defines 10 frame types in the lower nibble:

| Value | Name | Purpose |
|-------|------|---------|
| 0x00 | DATA | Application payload |
| 0x01 | HS_INIT | Handshake initiation |
| 0x02 | HS_REPLY | Handshake response |
| 0x03 | HS_COMPLETE | Handshake confirmation |
| 0x04 | PING | Keep-alive probe |
| 0x05 | PONG | Keep-alive response |
| 0x06 | CLOSE | Graceful disconnect |
| 0x07 | FILE_HDR | File transfer metadata |
| 0x08 | FILE_CHUNK | File data segment |
| 0x09 | ACK | Frame acknowledgement |
| 0x0A–0x0F | — | Reserved |

### 3.3 VarInt Encoding

All multi-byte header fields use QUIC-style variable-length integers as defined in RFC 9000 §16. The two most significant bits of the first byte encode the wire length:

| Prefix | Wire Length | Usable Bits | Max Value |
|--------|------------|-------------|-----------|
| 00 | 1 byte | 6 | 63 |
| 01 | 2 bytes | 14 | 16,383 |
| 10 | 4 bytes | 30 | 1,073,741,823 |
| 11 | 8 bytes | 62 | 4.6 × 10¹⁸ |

This enables PO to encode small channel IDs and payload lengths with zero overhead while supporting payloads up to 4 exabytes.

---

## 4. Cryptographic Layer (`po-crypto`)

### 4.1 Identity

Each PO node generates a persistent **Ed25519** keypair (RFC 8032):

- **Signing Key**: 32-byte secret, used to sign handshake messages.
- **Verifying Key**: 32-byte public key, shared with peers.
- **Node ID**: `SHA-256(verifying_key)` — a unique 32-byte identifier that serves as the node's address on the network.

Node identities are self-certifying: knowing a node's public key is sufficient to verify its identity. No certificate authority or PKI infrastructure is required.

### 4.2 Key Exchange

PO performs an **Elliptic-Curve Diffie-Hellman** exchange using **X25519** (RFC 7748):

1. Each peer generates an **ephemeral** X25519 keypair for the session.
2. The shared secret is computed via ECDH.
3. A session key is derived via **HKDF-SHA256** (RFC 5869) with a context string that includes both nodes' Ed25519 public keys.

The ephemeral keypair is consumed after use and zeroized from memory, guaranteeing **perfect forward secrecy**: compromise of a node's long-term Ed25519 key does not compromise past sessions.

### 4.3 Session Encryption

Once the session key is derived, all data frames are encrypted with **ChaCha20-Poly1305** (RFC 8439):

- **Nonce**: 12 bytes, derived from a monotonically incrementing counter (4 zero bytes + 8-byte LE counter).
- **AAD (Associated Authenticated Data)**: The encoded frame header bytes, binding the ciphertext to its header and preventing header-tampering attacks.
- **Output**: `nonce (12) || ciphertext || auth_tag (16)` — **28 bytes** of overhead per encrypted frame.

The encryption is **mandatory** — PO has no plaintext data mode.

---

## 5. Handshake Protocol

PO implements a 3-message authenticated key exchange:

```text
     Initiator (A)                          Responder (B)
         │                                       │
         │  HS_INIT                               │
         │  [version, ed25519_pub_A,              │
         │   x25519_eph_A, timestamp,             │
         │   sig_A(version||eph_A||ts)]           │
         │──────────────────────────────────────►  │
         │                                       │
         │  HS_REPLY                              │
         │  [ed25519_pub_B,                       │
         │   x25519_eph_B,                        │
         │   sig_B(eph_A||eph_B)]                 │
         │◄──────────────────────────────────────  │
         │                                       │
         │  HS_COMPLETE                           │
         │  [E(session_key, "PO_READY")]          │
         │──────────────────────────────────────►  │
         │                                       │
         │       ═══ Encrypted Session ═══        │
```

### 5.1 Step 1: HandshakeInit (A → B)

The initiator sends:

| Field | Size | Description |
|-------|------|-------------|
| version | 1 byte | Protocol version (currently `1`) |
| ed25519_pubkey | 32 bytes | Initiator's identity public key |
| x25519_ephemeral | 32 bytes | Ephemeral ECDH public key |
| timestamp | 8 bytes | Unix milliseconds (replay protection) |
| signature | 64 bytes | `Ed25519_sign(version \|\| x25519_eph \|\| timestamp)` |

### 5.2 Step 2: HandshakeReply (B → A)

The responder:

1. Verifies the initiator's signature against their Ed25519 public key.
2. Checks timestamp freshness (±30 seconds tolerance).
3. Generates its own ephemeral X25519 keypair.
4. Signs `(initiator_eph_pub || responder_eph_pub)`.
5. Sends back its Ed25519 public key, ephemeral public key, and signature.

### 5.3 Step 3: HandshakeComplete (A → B)

The initiator:

1. Verifies the responder's signature.
2. Derives the session key via `ECDH(eph_A_secret, eph_B_public) → HKDF-SHA256`.
3. Encrypts `"PO_READY"` with the session cipher.
4. Sends the encrypted confirmation.

The responder decrypts the confirmation and verifies it equals `"PO_READY"`. If verification succeeds, the session is **Established**.

### 5.4 Session Context (HKDF)

The HKDF context string is:

```
"po-v1-" || initiator_ed25519_pubkey || responder_ed25519_pubkey
```

The initiator's key always comes first, ensuring both sides derive identical session keys regardless of who initiated.

---

## 6. Transport Layer (`po-transport`)

### 6.1 Transport Abstraction

PO defines an `AsyncFrameTransport` trait with four methods:

```rust
async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
async fn write_all(&mut self, data: &[u8]) -> Result<(), TransportError>;
async fn flush(&mut self) -> Result<(), TransportError>;
async fn close(&mut self) -> Result<(), TransportError>;
```

Any ordered, reliable byte stream can implement this trait. Current implementations:

- **QuicTransport** — Production transport over QUIC/UDP via Quinn.
- **MemoryTransport** — In-memory channel pair for testing.

Planned implementations include BLE, Wi-Fi Direct, LoRa, and serial.

### 6.2 QUIC Configuration

- **ALPN Protocol ID**: `po/1`
- **TLS Certificates**: Self-signed (generated per-listener via `rcgen`). PO handles authentication at the application layer through its own Ed25519 handshake.
- **Server Certificate Verification**: Bypassed on the client side — peer identity is verified via the PO handshake, not the TLS layer.
- **Default Port**: `4433` (data), `5433` (LAN discovery)

---

## 7. Session Management (`po-session`)

### 7.1 State Machine

A PO connection progresses through five states:

```
New → Handshaking → Established → Closing → Closed
```

| State | Description |
|-------|-------------|
| New | Transport connected, no handshake yet |
| Handshaking | 3-way handshake in progress |
| Established | Session key active, encrypted data flowing |
| Closing | Close frame sent, waiting for peer |
| Closed | Connection fully terminated |

### 7.2 Framer

The `Framer` handles frame I/O over the raw transport:

- **Write path**: Encodes the `FrameHeader` into bytes, then writes header + payload sequentially.
- **Read path**: Accumulates bytes into an internal buffer (`BytesMut`), attempts to parse a header, reads remaining payload bytes, and returns the complete frame.
- **Max frame size**: 10 MB default (configurable).

### 7.3 Channel Multiplexing

A single PO connection supports multiple logical channels via the `channel_id` field in the frame header:

| Channel | Purpose |
|---------|---------|
| 0 | Control (handshake, ping, close) |
| 1 | Default data channel |
| 2 | File transfer |
| 100+ | User-defined channels |

---

## 8. Peer Discovery

PO includes a UDP broadcast-based discovery mechanism for LAN environments:

- **Beacon Format**: `PO|<node_id_hex>|<quic_port>`
- **Broadcast Address**: `255.255.255.255:5433`
- **Interval**: Configurable (default: periodic)

Nodes broadcast their identity and listen for beacons from peers. Discovered peers are stored in a concurrent hashmap (`DashMap`) with deduplication by Node ID and self-filtering.

---

## 9. Security Analysis

### 9.1 Threat Model

PO assumes the network is hostile (untrusted intermediaries, eavesdroppers, active attackers). The protocol provides:

| Property | Mechanism |
|----------|-----------|
| **Confidentiality** | ChaCha20-Poly1305 encryption on all data frames |
| **Integrity** | Poly1305 authentication tag + AAD binding |
| **Authentication** | Ed25519 signatures during handshake |
| **Perfect Forward Secrecy** | Ephemeral X25519 keypairs, zeroized after use |
| **Replay protection** | Timestamp in HandshakeInit (±30s window) |
| **Header integrity** | Frame header used as AAD for encryption |

### 9.2 Known Limitations (v0.1)

- **No key persistence**: Identities are regenerated each session unless the application persists the Ed25519 secret key.
- **No certificate pinning / TOFU**: Peer identity must be verified out-of-band on first contact.
- **Single-connection model**: `Po::bind()` accepts one connection; multi-peer requires spawning multiple instances.
- **Discovery is LAN-only**: No DHT, relay, or NAT traversal in v0.1.

---

## 10. Performance Characteristics

### 10.1 Overhead

| Component | Overhead |
|-----------|----------|
| Frame header | 4–25 bytes |
| Encryption per frame | 28 bytes (12 nonce + 16 tag) |
| Handshake | 3 messages, ~450 bytes total |

### 10.2 Optimizations

- **Release profile**: `opt-level = 3`, LTO = fat, `codegen-units = 1`, symbols stripped.
- **`no_std` wire codec**: Zero-allocation frame parsing suitable for embedded targets.
- **QUIC stream multiplexing**: No head-of-line blocking between channels.
- **Monotonic nonce**: Counter-based nonce derivation avoids random generation overhead.

---

## 11. API Surface

PO exposes a deliberately minimal API:

```rust
// Server — 2 lines
let mut server = Po::bind(4433).await?;
let (channel, data) = server.recv().await?.unwrap();

// Client — 3 lines
let mut client = Po::connect("192.168.1.5:4433").await?;
client.send(b"Hello, encrypted world!").await?;
let response = client.recv().await?;
```

Internally, `Po::connect()` performs QUIC connection establishment, the 3-way cryptographic handshake, and session key derivation — returning a ready-to-use encrypted channel.

---

## 12. Roadmap

| Phase | Features |
|-------|----------|
| v0.1 (current) | Core protocol, E2EE, QUIC transport, CLI, LAN discovery |
| v0.2 | Key persistence, multi-peer connections, NAT hole punching |
| v0.3 | File transfer API, streaming support, progress callbacks |
| v0.4 | WASM/browser support, BLE transport, Wi-Fi Direct |
| v1.0 | Protocol audit, crates.io publication, npm bindings |

---

## 13. Conclusion

Protocol Orzatty demonstrates that encrypted peer-to-peer communication does not require complex infrastructure. By composing proven primitives — Ed25519, X25519, ChaCha20-Poly1305, HKDF, QUIC — into a purpose-built stack with a minimal API, PO provides a foundation for applications where privacy and performance are non-negotiable.

---

**Protocol Orzatty is developed by [Orzatty Holdings](https://orzatty.com).**
**© 2026 Dylan Orzatty. All rights reserved.**
