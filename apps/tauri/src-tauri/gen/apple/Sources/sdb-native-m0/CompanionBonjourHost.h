#pragma once

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int32_t sdb_companion_bonjour_start(uint16_t port, const char *host_id,
                                    uint16_t protocol_version);
void sdb_companion_bonjour_stop(void);

#ifdef __cplusplus
}
#endif
