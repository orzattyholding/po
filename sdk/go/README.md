# PO for Go — Protocol Orzatty Go SDK

Use Protocol Orzatty natively in Go via `cgo` FFI bindings.

## Requirements

- Go 1.20+
- GCC or compatible C compiler (for cgo)
- `po.h` + `po_c.dll` / `libpo_c.so`

## Installation

```bash
# Download the PO native library from GitHub Releases
# Place po.h and libpo_c.so/po_c.lib in your project root
```

## Quick Start

```go
package main

/*
#cgo LDFLAGS: -L. -lpo_c
#include "po.h"
*/
import "C"
import (
    "fmt"
    "unsafe"
)

func main() {
    addr := C.CString("127.0.0.1:9091")
    defer C.free(unsafe.Pointer(addr))
    port := C.CString("0")
    defer C.free(unsafe.Pointer(port))

    client := C.po_client_new(port, addr)
    if client == nil {
        fmt.Println("Connection failed")
        return
    }
    defer C.po_client_free(client)

    msg := "Hello from Go over E2EE QUIC!"
    cmsg := C.CString(msg)
    defer C.free(unsafe.Pointer(cmsg))

    C.po_client_send(client, (*C.uint8_t)(unsafe.Pointer(cmsg)), C.size_t(len(msg)))
    fmt.Println("Payload delivered securely.")
}
```

## Build

```bash
CGO_ENABLED=1 go build -o demo_po demo_go.go
```

---

*Built by [Orzatty Corporation](https://orzatty.com)*
