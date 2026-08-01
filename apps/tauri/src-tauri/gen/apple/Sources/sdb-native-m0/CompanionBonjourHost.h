#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int32_t sdb_companion_bonjour_start(uint16_t port, const char *host_id,
                                    uint16_t protocol_version);
void sdb_companion_bonjour_stop(void);

int32_t sdb_companion_bonjour_browser_start(void);
void sdb_companion_bonjour_browser_stop(void);
int32_t sdb_companion_bonjour_browser_snapshot(uint8_t **bytes, size_t *length);
void sdb_companion_bonjour_browser_snapshot_free(uint8_t *bytes, size_t length);

#ifdef __cplusplus
}
#endif
