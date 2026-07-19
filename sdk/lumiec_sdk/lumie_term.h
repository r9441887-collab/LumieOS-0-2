/*
 * LumieOS SDK - Terminal I/O Functions
 * 
 * Provides terminal write, color, and cursor control from LumieC.
 */

#ifndef LUMIE_OS_TERM_H
#define LUMIE_OS_TERM_H

#include "lumie_os.h"

/* Write a string to the terminal */
void term_write(const char *s) {
    syscall1(SYS_WRITE, (i64)s);
}

/* Write a newline-terminated string */
void term_writeln(const char *s) {
    term_write(s);
    term_write("\n");
}

/* Clear screen with background color */
void term_clear(u32 bg) {
    syscall1(SYS_CLEAR, (u64)bg);
}

/* Set foreground text color */
void term_set_fg(u32 color) {
    syscall1(SYS_SETCOLOR, (u64)color);
}

/* Print a single character */
void term_putchar(char c) {
    char buf[2];
    buf[0] = c;
    buf[1] = 0;
    syscall1(SYS_WRITE, (i64)buf);
}

/* Print a number as decimal string */
void term_print_int(i64 n) {
    char buf[32];
    int i = 0;
    int neg = 0;
    
    if (n < 0) {
        neg = 1;
        n = -n;
    }
    
    if (n == 0) {
        buf[i++] = '0';
    } else {
        while (n > 0) {
            buf[i++] = '0' + (n % 10);
            n /= 10;
        }
    }
    
    if (neg) {
        buf[i++] = '-';
    }
    
    /* Reverse the string */
    int j;
    for (j = 0; j < i / 2; j++) {
        char tmp = buf[j];
        buf[j] = buf[i - 1 - j];
        buf[i - 1 - j] = tmp;
    }
    
    buf[i] = 0;
    term_write(buf);
}

/* Print a number as hexadecimal */
void term_print_hex(i64 n) {
    char buf[32];
    const char *hex = "0123456789ABCDEF";
    int i = 0;
    
    if (n == 0) {
        buf[i++] = '0';
    } else {
        while (n > 0) {
            buf[i++] = hex[n & 0xF];
            n >>= 4;
        }
    }
    
    int j;
    for (j = 0; j < i / 2; j++) {
        char tmp = buf[j];
        buf[j] = buf[i - 1 - j];
        buf[i - 1 - j] = tmp;
    }
    
    buf[i] = 0;
    term_write("0x");
    term_write(buf);
}

#endif /* LUMIE_OS_TERM_H */
