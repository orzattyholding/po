# PO for Python — Protocol Orzatty Python SDK

Python bindings via UniFFI (Rust FFI).

## Installation

```bash
pip install protocol-orzatty
```

## Quick Start

```python
from po import PoClient

# Server
server = PoClient("4433", None)
print(f"Server Node ID: {server.node_id()}")

# Client
client = PoClient("0", "127.0.0.1:4433")
client.send(b"Python E2EE QUIC payload")

# Receive
data = server.recv()
print(f"Received: {bytes(data).decode()}")

client.close()
```

## Async Usage (Threading)

```python
import threading
from po import PoClient

def server_loop():
    server = PoClient("4433", None)
    while True:
        data = server.recv()
        if data:
            print(f"Got: {bytes(data).decode()}")

thread = threading.Thread(target=server_loop, daemon=True)
thread.start()
```

---

*Built by [Orzatty Corporation](https://orzatty.com)*
