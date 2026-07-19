#!/usr/bin/env python3
"""Packages files into install.pkg for LumieOS loader (with LZ4 compression).

Usage:
    python make_install_pkg.py <source_dir> <output_file>

Example:
    python make_install_pkg.py os_files/ install.pkg

The source_dir should contain files with paths like:
    system/kernel.lkrn
    drivers/kbd.ldrv
    drivers/fs.ldrv
    drivers/mouse.ldrv
    system/shell.lsh
    drivers/nv_gpu.sys
    EFI/BOOT/BOOTX64.EFI

Directories are automatically created and don't need explicit entries.
"""

import struct
import sys
import os

PKG_MAGIC = 0x4B47504C  # "LPKG" LE
PKG_VERSION = 2  # bumped for compression support
ENTRY_SIZE = 32
FLAG_DIR = 1
FLAG_LZ1 = 2

USE_LZ1 = True


def _lz1_compress(data: bytes) -> bytes:
    from lzss import compress
    return compress(data)


def make_install_pkg(source_dir: str, output: str, compress: bool = True):
    entries = []
    for root, dirs, files in os.walk(source_dir):
        for f in sorted(files):
            full = os.path.join(root, f)
            rel = '/' + os.path.relpath(full, source_dir).replace('\\', '/')
            size = os.path.getsize(full)
            entries.append((rel, full, size, False))
        for d in sorted(dirs):
            full = os.path.join(root, d)
            rel = '/' + os.path.relpath(full, source_dir).replace('\\', '/')
            entries.append((rel, full, 0, True))

    seen = set()
    unique = []
    for rel, full, size, is_dir in entries:
        if rel not in seen:
            seen.add(rel)
            unique.append((rel, full, size, is_dir))
    unique.sort(key=lambda x: (x[3], x[0]))

    file_count = len(unique)

    buf = bytearray()
    buf += struct.pack('<IIII', PKG_MAGIC, PKG_VERSION, file_count, 0)

    entries_off = 16
    struct.pack_into('<I', buf, 12, entries_off)

    for _ in unique:
        buf += b'\x00' * ENTRY_SIZE

    path_offs = []
    for rel, _, _, _ in unique:
        path_offs.append(len(buf))
        buf += rel.encode('utf-8') + b'\x00'

    while len(buf) % 4 != 0:
        buf += b'\x00'

    # File data
    file_infos = []
    for rel, full, size, is_dir in unique:
        data_off = len(buf)
        orig_size = 0
        store_size = 0
        is_lz1 = False
        if not is_dir:
            with open(full, 'rb') as fh:
                raw = fh.read()
            orig_size = len(raw)
            if compress:
                c = _lz1_compress(raw)
                if len(c) < len(raw):
                    buf += c
                    store_size = len(c)
                    is_lz1 = True
                else:
                    buf += raw
                    store_size = len(raw)
            else:
                buf += raw
                store_size = len(raw)
            while len(buf) % 4 != 0:
                buf += b'\x00'
        file_infos.append((data_off, orig_size, store_size, is_lz1))

    for i, (rel, _, raw_size, is_dir) in enumerate(unique):
        off = entries_off + i * ENTRY_SIZE
        flags = 0
        if is_dir:
            flags |= FLAG_DIR
        if file_infos[i][3]:
            flags |= FLAG_LZ1
        store_size = file_infos[i][2]
        orig_size = file_infos[i][1]
        struct.pack_into('<IIII', buf, off,
                         path_offs[i],
                         file_infos[i][0],
                         store_size,
                         flags)
        struct.pack_into('<I', buf, off + 16, orig_size)

    with open(output, 'wb') as fh:
        fh.write(buf)

    print(f"Created {output}: {file_count} entries, {len(buf)} bytes total")
    for i, (rel, _, size, is_dir) in enumerate(unique):
        tag = "DIR " if is_dir else "FILE"
        if file_infos[i][3]:
            tag += "(LZ1)"
            print(f"  [{tag}] {rel} ({size} -> {file_infos[i][1]} bytes, stored={file_infos[i][2]} bytes)")
        else:
            print(f"  [{tag}] {rel} ({size} bytes)")


if __name__ == '__main__':
    if len(sys.argv) not in (3, 4):
        print(__doc__, file=sys.stderr)
        sys.exit(1)
    compress = True
    if len(sys.argv) == 4 and sys.argv[3] == '--no-compress':
        compress = False
    make_install_pkg(sys.argv[1], sys.argv[2], compress=compress)
