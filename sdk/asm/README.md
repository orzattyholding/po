# PO for Assembly — Protocol Orzatty x86_64 NASM SDK

**The most bare-metal E2EE networking protocol on Earth.**

No runtime. No garbage collector. No JIT. No abstractions. 
Just raw CPU registers calling XChaCha20-Poly1305 encrypted QUIC tunnels.

## Requirements

- [NASM](https://nasm.us/) (Netwide Assembler) 2.15+
- MSVC Linker (`link.exe`) — comes with Visual Studio Build Tools
- `po_c.lib` — PO native static library (from GitHub Releases)

## Build (Windows x64)

```bash
# 1. Assemble
nasm -f win64 demo_po.asm -o demo_po.obj

# 2. Link against PO + C runtime
link demo_po.obj po_c.lib msvcrt.lib kernel32.lib ws2_32.lib userenv.lib ntdll.lib bcrypt.lib /subsystem:console /entry:main
```

## How It Works

```
┌──────────────┐     C ABI (extern "C")      ┌──────────────────┐
│  x86_64 ASM  │ ──────────────────────────── │  po_c.lib (Rust) │
│  (your code) │   po_client_new()            │  QUIC + E2EE     │
│  registers   │   po_client_send()           │  ChaCha20-Poly   │
│  & stack     │   po_client_free()           │  Ed25519 + X25519│
└──────────────┘                              └──────────────────┘
        │                                              │
        └──────── Windows x64 ABI (rcx,rdx,r8,r9) ────┘
```

## API (Assembly Calling Convention)

All functions follow the **Windows x64 ABI**: first 4 args in `rcx`, `rdx`, `r8`, `r9`.

| Function | Args | Returns |
|---|---|---|
| `po_client_new` | `rcx`=bind_addr, `rdx`=remote_addr | `rax` = handle (NULL on fail) |
| `po_client_send` | `rcx`=handle, `rdx`=data_ptr, `r8`=len | `eax` = 0 success, -1 error |
| `po_client_free` | `rcx`=handle | void |

## Why?

Because sometimes you need to prove that your protocol is so clean, so well-engineered, that even **raw Assembly can use it in 20 lines**. No framework. No SDK wrapper. Just `call po_client_send` and your bytes fly encrypted across the planet.

This is Orzatty-grade engineering.

---

*Built by [Orzatty Corporation](https://orzatty.com) · Created by Dylan Orzatty*
