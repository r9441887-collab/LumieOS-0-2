/*
 * LumieOS SDK - All-in-one Header
 * 
 * Include this single header to get access to all LumieOS SDK functions.
 * 
 * Usage:
 *   #include "lumie_os_all.h"
 *   
 *   int lumiec_main(void) {
 *       term_clear(COLOR_BLUE);
 *       term_write("Hello from LumieC!");
 *       draw_rect(100, 100, 200, 100, CLR_RED);
 *       return 0;
 *   }
 */

#ifndef LUMIE_OS_ALL_H
#define LUMIE_OS_ALL_H

#include "lumie_os.h"
#include "lumie_term.h"
#include "lumie_mem.h"
#include "lumie_gfx.h"
#include "lumie_fs.h"
#include "lumie_input.h"

#endif /* LUMIE_OS_ALL_H */
