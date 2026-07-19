/*
 * LumieOS SDK - Memory Management
 * 
 * Kernel heap allocation functions for LumieC programs.
 */

#ifndef LUMIE_OS_MEM_H
#define LUMIE_OS_MEM_H

#include "lumie_os.h"

/* Allocate memory from kernel heap */
void *kmalloc(i64 size) {
    return (void *)syscall1(SYS_ALLOC, size);
}

/* Free allocated memory */
void kfree(void *ptr) {
    syscall1(SYS_FREE, (i64)ptr);
}

/* Set memory to value */
void memset(void *dst, i64 val, i64 size) {
    unsigned char *d = (unsigned char *)dst;
    i64 i;
    for (i = 0; i < size; i++) {
        d[i] = (unsigned char)val;
    }
}

/* Copy memory */
void memcpy(void *dst, void *src, i64 size) {
    unsigned char *d = (unsigned char *)dst;
    unsigned char *s = (unsigned char *)src;
    i64 i;
    for (i = 0; i < size; i++) {
        d[i] = s[i];
    }
}

/* Get string length */
i64 strlen(const char *s) {
    i64 len = 0;
    while (s[len] != 0) {
        len++;
    }
    return len;
}

/* Compare strings */
i64 strcmp(const char *a, const char *b) {
    while (*a && *b) {
        if (*a != *b) {
            return *a - *b;
        }
        a++;
        b++;
    }
    return *a - *b;
}

/* Copy string */
void strcpy(char *dst, const char *src) {
    while (*src) {
        *dst++ = *src++;
    }
    *dst = 0;
}

/* Concatenate strings */
void strcat(char *dst, const char *src) {
    while (*dst) {
        dst++;
    }
    while (*src) {
        *dst++ = *src++;
    }
    *dst = 0;
}

/* Sleep for milliseconds */
void sleep_ms(i64 ms) {
    syscall1(SYS_SLEEP, ms * 1000);
}

/* Get system tick count */
i64 get_ticks() {
    return syscall1(SYS_TIME, 0);
}

#endif /* LUMIE_OS_MEM_H */
