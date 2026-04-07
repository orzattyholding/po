# PO for C — Protocol Orzatty Native C SDK

The lowest-level SDK for Protocol Orzatty. Pure C89/C99 compatible.

## Requirements

- Any C compiler (GCC, Clang, MSVC)
- `po.h` + `po_c.dll` / `libpo_c.so` / `libpo_c.a`

## Installation

```bash
# Download from GitHub Releases
# Copy po.h and the library to your project
```

## Build

```bash
# Linux/macOS
gcc -o demo demo_c.c -L. -lpo_c -lpthread -ldl -lm

# Windows (MSVC)
cl demo_c.c po_c.lib
```

## Quick Start

```c
#include <stdio.h>
#include <string.h>
#include "po.h"

int main() {
    PoClientC* client = po_client_new("0", "127.0.0.1:9091");
    if (!client) {
        printf("Connection failed\n");
        return 1;
    }

    const char* msg = "Native C payload";
    po_client_send(client, (const uint8_t*)msg, strlen(msg));

    po_client_free(client);
    return 0;
}
```

---

*Built by [Orzatty Corporation](https://orzatty.com)*
