#include "bindings/bindings.h"
#include "BoardTransportHost.h"
#include "ProjectorDisplayHost.h"

#include <dispatch/dispatch.h>

int main(int argc, char * argv[]) {
	dispatch_after(
		dispatch_time(DISPATCH_TIME_NOW, (int64_t)(NSEC_PER_SEC)),
		dispatch_get_main_queue(),
		^{
			sdb_install_projector_display_host();
		}
	);
	ffi::start_app();
	return 0;
}
