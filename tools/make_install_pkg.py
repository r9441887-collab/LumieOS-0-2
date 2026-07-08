#!/usr/bin/env python3
"""Packages files into install.pkg for LumieOS loader.

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
import hashlib

PKG_MAGIC = 0x4B47504C  # "LPKG" LE
PKG_VERSION = 1
ENTRY_SIZE = 32


def make_install_pkg(source_dir: str, output: str):
    # Collect all files from source_dir
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

    # Deduplicate and sort: files first then dirs, alphabetical
    seen = set()
    unique = []
    for rel, full, size, is_dir in entries:
        if rel not in seen:
            seen.add(rel)
            unique.append((rel, full, size, is_dir))
    # Ensure parent dirs exist before children
    unique.sort(key=lambda x: (x[3], x[0]))

    file_count = len(unique)

    # Build the package in memory
    buf = bytearray()

    # -- Header (16 bytes) --
    buf += struct.pack('<IIII', PKG_MAGIC, PKG_VERSION, file_count, 0)  # entries_off placeholder

    entries_off = 16
    struct.pack_into('<I', buf, 12, entries_off)

    # -- Entries (file_count * 32 bytes) --
    for _ in unique:
        buf += b'\x00' * ENTRY_SIZE

    # -- Path strings --
    path_offs = []
    for rel, _, _, _ in unique:
        path_offs.append(len(buf))
        buf += rel.encode('utf-8') + b'\x00'

    # Pad to 4 bytes
    while len(buf) % 4 != 0:
        buf += b'\x00'

    # -- File data --
    data_offs = []
    for i, (rel, full, size, is_dir) in enumerate(unique):
        data_offs.append(len(buf))
        if not is_dir:
            with open(full, 'rb') as fh:
                data = fh.read()
            buf += data
            # Pad to 4 bytes
            while len(buf) % 4 != 0:
                buf += b'\x00'

    # Write back entry data
    for i, (rel, _, size, is_dir) in enumerate(unique):
        off = entries_off + i * ENTRY_SIZE
        flags = 1 if is_dir else 0
        struct.pack_into('<IIII', buf, off,
                         path_offs[i],       # path_off
                         data_offs[i],       # data_off
                         size,               # data_sz
                         flags)             # flags + reserved

    with open(output, 'wb') as fh:
        fh.write(buf)

    print(f"Created {output}: {file_count} entries, {len(buf)} bytes total")
    for i, (rel, _, size, is_dir) in enumerate(unique):
        tag = "DIR " if is_dir else "FILE"
        print(f"  [{tag}] {rel} ({size} bytes)")


if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        sys.exit(1)
    make_install_pkg(sys.argv[1], sys.argv[2])
