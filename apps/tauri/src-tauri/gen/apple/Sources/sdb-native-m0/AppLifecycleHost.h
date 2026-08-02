#pragma once

#ifdef __cplusplus
extern "C" {
#endif

void sdb_install_app_lifecycle_host(void);
void sdb_stop_app_lifecycle_host(void);
void sdb_set_arcade_session_active(bool active);

#ifdef __cplusplus
}
#endif
