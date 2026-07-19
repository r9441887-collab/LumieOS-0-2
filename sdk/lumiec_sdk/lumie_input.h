/*
 * LumieOS SDK - Input Handling
 * 
 * Keyboard and mouse input for LumieC programs.
 */

#ifndef LUMIE_OS_INPUT_H
#define LUMIE_OS_INPUT_H

#include "lumie_os.h"

/* Get keyboard scancode (non-blocking, returns -1 if no key) */
i64 input_getkey() {
    return syscall1(SYS_GETKEY, 0);
}

/* Get mouse state (dx, dy, buttons) */
i64 input_getmouse(i64 *dx, i64 *dy) {
    return syscall2(SYS_GETMOUSE, (i64)dx, (i64)dy);
}

/* Wait for a key press and return the scancode */
i64 input_waitkey() {
    i64 key;
    while (1) {
        key = input_getkey();
        if (key != -1) {
            return key;
        }
        syscall0(SYS_YIELD);
    }
}

/* Check if a key is currently pressed */
i64 input_keypressed() {
    return input_getkey() != -1;
}

/* Check for specific key */
i64 input_is_enter(i64 key) {
    return key == KEY_ENTER;
}

i64 input_is_escape(i64 key) {
    return key == KEY_ESC;
}

i64 input_is_backspace(i64 key) {
    return key == KEY_BACKSPACE;
}

i64 input_is_arrow(i64 key) {
    return key == KEY_UP || key == KEY_DOWN || key == KEY_LEFT || key == KEY_RIGHT;
}

#endif /* LUMIE_OS_INPUT_H */
