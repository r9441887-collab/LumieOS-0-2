#!/usr/bin/env python3
"""Create minimal valid .ldrv stub files for boot check."""
import struct, os

MAGIC_LDRV = 0x5652444C
RELEASE_DIR = os.path.join(os.path.dirname(__file__), '..', 'build', 'release', 'drivers')

def make_ldrv(name):
    hdr = struct.pack('<IIIIIIIIII',
        MAGIC_LDRV,  # magic
        0,           # entry
        0,           # code_size
        0,           # bss_size
        0,           # reloc_off
        0,           # reloc_count
        0,           # import_off
        0,           # import_count
        0,           # strtab_off
        0,           # strtab_size
    )
    name_bytes = name.encode('ascii')[:23].ljust(24, b'\x00')
    return hdr + name_bytes

os.makedirs(RELEASE_DIR, exist_ok=True)
for name in ['kbd', 'fs', 'mouse']:
    path = os.path.join(RELEASE_DIR, f'{name}.ldrv')
    with open(path, 'wb') as f:
        f.write(make_ldrv(name))
    print(f"Created {path} ({os.path.getsize(path)} bytes)")
