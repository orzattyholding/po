#include <stdio.h>
#include <string.h>
#include "../crates/po-c/po.h" // Generado por cbindgen

int main() {
    printf("Starting PO Protocol - C Native Client...\n");
    
    // Conectamos como cliente
    PoClientC* client = po_client_new("0", "127.0.0.1:9091");
    if (!client) {
        printf("Error: Fallo de apretón de manos E2EE\n");
        return 1;
    }
    
    const char* msg = "Payload CRITICO C-Native";
    if (po_client_send(client, (const uint8_t*)msg, strlen(msg)) == 0) {
        printf("Paquete QUIC enviado. Cifrado nativo activo.\n");
    }

    po_client_free(client);
    return 0;
}
