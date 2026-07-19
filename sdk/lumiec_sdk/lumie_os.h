/*
 * LumieOS SDK - System Call Definitions
 * 
 * These are the system call numbers available from LumieC programs.
 * Usage: syscall1(SYS_WRITE, (i64)message);
 */

#ifndef LUMIE_OS_SYSCALL_H
#define LUMIE_OS_SYSCALL_H

/* System call numbers */
#define SYS_EXIT        0x00
#define SYS_WRITE       0x01
#define SYS_READ        0x02
#define SYS_ALLOC       0x03
#define SYS_FREE        0x04
#define SYS_CLEAR       0x05
#define SYS_SETCOLOR    0x06
#define SYS_GETKEY      0x07
#define SYS_SLEEP       0x08
#define SYS_GETMOUSE    0x09
#define SYS_FS_READ     0x0A
#define SYS_FS_WRITE    0x0B
#define SYS_FS_EXISTS   0x0C
#define SYS_FS_MKDIR    0x0D
#define SYS_FS_LIST     0x0E
#define SYS_REBOOT      0x0F
#define SYS_TIME        0x10
#define SYS_YIELD       0x11
#define SYS_GET_FB_INFO 0x12
#define SYS_DRAW_PIXEL  0x13
#define SYS_GPU_RENDER  0x14
#define SYS_GPU_INFO    0x15
#define SYS_GPU_RECT    0x16
#define SYS_GETPID      0x17
#define SYS_WAIT        0x18
#define SYS_SET_TITLE   0x19
#define SYS_FS_DELETE   0x1A
#define SYS_FS_SIZE     0x1B
#define SYS_SPAWN       0x1C
#define SYS_YIELD_TO    0x1D

/* Colors (standard VGA) */
#define COLOR_BLACK     0x00
#define COLOR_BLUE      0x01
#define COLOR_GREEN     0x02
#define COLOR_CYAN      0x03
#define COLOR_RED       0x04
#define COLOR_MAGENTA   0x05
#define COLOR_BROWN     0x06
#define COLOR_LGRAY     0x07
#define COLOR_DGRAY     0x08
#define COLOR_LBLUE     0x09
#define COLOR_LGREEN    0x0A
#define COLOR_LCYAN     0x0B
#define COLOR_LRED      0x0C
#define COLOR_LMAGENTA  0x0D
#define COLOR_YELLOW    0x0E
#define COLOR_WHITE     0x0F

/* Key codes */
#define KEY_ENTER       0x0D
#define KEY_BACKSPACE   0x08
#define KEY_TAB         0x09
#define KEY_ESC         0x1B
#define KEY_UP          0x100
#define KEY_DOWN        0x101
#define KEY_LEFT        0x102
#define KEY_RIGHT       0x103
#define KEY_F1          0x110
#define KEY_F2          0x111
#define KEY_F3          0x112
#define KEY_F4          0x113
#define KEY_F5          0x114
#define KEY_F6          0x115
#define KEY_F7          0x116
#define KEY_F8          0x117
#define KEY_F9          0x118
#define KEY_F10         0x119
#define KEY_F11         0x11A
#define KEY_F12         0x11B

/* User roles */
#define ROLE_USER       0
#define ROLE_ADMIN      1

#endif /* LUMIE_OS_SYSCALL_H */
