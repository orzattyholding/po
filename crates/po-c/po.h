#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct PoClientC PoClientC;

struct PoClientC *po_client_new(const char *bind_address_or_port, const char *remote_address);

int32_t po_client_send(struct PoClientC *client, const uint8_t *data, uintptr_t len);

void po_client_free(struct PoClientC *client);
