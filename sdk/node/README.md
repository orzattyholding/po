# PO for Node.js — Protocol Orzatty JavaScript/TypeScript SDK

The official Node.js SDK. Native NAPI-RS bindings for maximum performance.

## Installation

```bash
npm install @orzattyholding/po
```

## Quick Start (ESM)

```javascript
import { PoClient } from '@orzattyholding/po';

const server = await PoClient.bind(4433);
const client = await PoClient.connect("127.0.0.1:4433");

// Send
await client.send(Buffer.from("Hello E2EE QUIC!"));

// Receive
const data = await server.recv();
console.log(data.toString());

// Batch API (>10k msg/s)
const batch = Array(500).fill(Buffer.from("batch-msg"));
await client.sendBatch(batch);

await client.close();
await server.close();
```

## Quick Start (CommonJS)

```javascript
const { PoClient } = require('@orzattyholding/po');

async function main() {
    const client = await PoClient.connect("127.0.0.1:4433");
    await client.send(Buffer.from("Hello!"));
    await client.close();
}
main();
```

## API

| Method | Description |
|---|---|
| `PoClient.bind(port)` | Listen for connections |
| `PoClient.connect(addr)` | Connect to a PO node |
| `client.send(buffer)` | Send encrypted data |
| `client.sendBatch(buffers)` | Send batch (>10k msg/s) |
| `client.recv()` | Receive data |
| `client.nodeId` | Get cryptographic node ID |
| `client.close()` | Graceful disconnect |

---

*Built by [Orzatty Corporation](https://orzatty.com)*
