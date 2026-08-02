#pragma once

#ifdef __cplusplus
extern "C" {
#endif

void sdb_install_projector_display_host(void);
void sdb_projector_update(const char *state_json);
void sdb_set_arcade_session_active(bool active);

#ifdef __cplusplus
}
#endif
