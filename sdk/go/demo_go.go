package main

/*
#cgo LDFLAGS: -L../target/release -lpo_c -lm -ldl -lws2_32 -luserenv -lntdll
#include "../crates/po-c/po.h"
*/
import "C"
import (
	"fmt"
	"unsafe"
)

func main() {
	fmt.Println("🚀 Dialing PO Protocol natively mapped in Go runtime")
	
	addr := C.CString("127.0.0.1:9091")
	defer C.free(unsafe.Pointer(addr))
	port := C.CString("0")
	defer C.free(unsafe.Pointer(port))

	client := C.po_client_new(port, addr)
	if client == nil {
		fmt.Println("Failed connection")
		return
	}
	defer C.po_client_free(client)

	msg := "Go Routine Async Data Payload E2EE"
	cmsg := C.CString(msg)
	defer C.free(unsafe.Pointer(cmsg))

	res := C.po_client_send(client, (*C.uint8_t)(unsafe.Pointer(cmsg)), C.size_t(len(msg)))
	if res == 0 {
	    fmt.Println("Payload delivered safely bypassing standard TCP limit.")
	}
}
