/*
 * LumieOS SDK - File System Functions
 * 
 * File I/O operations for LumieC programs.
 */

#ifndef LUMIE_OS_FS_H
#define LUMIE_OS_FS_H

#include "lumie_os.h"

/* Read a file into buffer, returns bytes read or -1 */
i64 fs_read(const char *path, void *buf, i64 max_size) {
    return syscall3(SYS_FS_READ, (i64)path, (i64)buf, max_size);
}

/* Write data to a file, returns bytes written or -1 */
i64 fs_write(const char *path, const void *data, i64 size) {
    return syscall3(SYS_FS_WRITE, (i64)path, (i64)data, size);
}

/* Check if file exists, returns 1 if yes, 0 if no */
i64 fs_exists(const char *path) {
    return syscall1(SYS_FS_EXISTS, (i64)path);
}

/* Create a directory */
i64 fs_mkdir(const char *path) {
    return syscall1(SYS_FS_MKDIR, (i64)path);
}

/* Delete a file */
i64 fs_delete(const char *path) {
    return syscall1(SYS_FS_DELETE, (i64)path);
}

/* Get file size */
i64 fs_size(const char *path) {
    return syscall1(SYS_FS_SIZE, (i64)path);
}

/* Read entire file into allocated buffer */
char *fs_read_all(const char *path) {
    i64 sz = fs_size(path);
    if (sz <= 0) {
        return (void *)0;
    }
    
    char *buf = kmalloc(sz + 1);
    if (!buf) {
        return (void *)0;
    }
    
    i64 read = fs_read(path, buf, sz);
    if (read <= 0) {
        kfree(buf);
        return (void *)0;
    }
    
    buf[read] = 0;
    return buf;
}

/* Write string to file */
i64 fs_write_str(const char *path, const char *content) {
    return fs_write(path, content, strlen(content));
}

#endif /* LUMIE_OS_FS_H */
