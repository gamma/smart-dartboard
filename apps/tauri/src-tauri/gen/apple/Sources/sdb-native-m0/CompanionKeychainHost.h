#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int32_t sdb_keychain_load(const char *account, uint8_t **bytes, size_t *length);
bool sdb_keychain_save(const char *account, const uint8_t *bytes, size_t length);
void sdb_keychain_free(uint8_t *bytes, size_t length);

#ifdef __cplusplus
}
#endif
