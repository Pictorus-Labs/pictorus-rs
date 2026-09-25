/*
 * Stand-in for the generated rm_pictorus_app_cfg.h, used only by bindgen.
 *
 * In a real project the RA Smart Configurator writes this file into
 * `ra_cfg/fsp_cfg/` from the module description's <config> element, filling in
 * whatever the user set in the Properties view. Here there is no project and no
 * configurator, and `rm_pictorus_app.h` includes it unconditionally, so bindgen
 * needs something to find.
 *
 * This file is not shipped in the generated pack. Only `rm_pictorus_app.{h,c}`
 * are; a project that received this stub as well would shadow its own
 * generated configuration.
 */

#ifndef RM_PICTORUS_APP_CFG_H_
#define RM_PICTORUS_APP_CFG_H_

#define RM_PICTORUS_APP_CFG_HEAP_BYTES        (1024)

#endif
