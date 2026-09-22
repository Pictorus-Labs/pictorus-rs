/*
 * Bindgen entry point: the FSP interface headers this crate binds.
 *
 * Deliberately a subset. Binding all 115 headers under `ra/fsp/inc/api/` is not
 * possible -- several of them include FreeRTOS, ThreadX, LittleFS or FreeRTOS+FAT
 * headers that ship in other packs -- and is not wanted, because every added
 * header widens the surface that a pack upgrade can shift underneath us.
 *
 * Adding a peripheral means adding its `r_*_api.h` to the HEADERS list in
 * `script/vendor-headers.sh`, re-running that script, adding the include here,
 * and refreshing the bindings snapshot. See README.md.
 */

#include "r_ioport_api.h" /* GPIO */
#include "r_timer_api.h"  /* PWM  */
