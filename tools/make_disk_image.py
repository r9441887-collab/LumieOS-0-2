#!/usr/bin/env python3
"""Create a GPT+FAT32 bootable disk image for LumieOS installer.

Usage:
    python make_disk_image.py <output.img> [--size MB] [--bootloader path] [--pkg path]

Creates a disk image with:
  Partition 1: ESP (FAT32) — EFI/BOOT/BOOTX64.EFI
  Partition 2: LumieOS data (FAT32) — install.pkg
"""

import struct
import os
import sys
import math

SECTOR = 512

def align_lba(pos):
    return (pos + 7) & ~7

def crc32(data: bytes) -> int:
    crc = 0xFFFFFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0xEDB88320
            else:
                crc >>= 1
    return crc ^ 0xFFFFFFFF

def name_to_83(name):
    name = name.upper()
    if '.' in name:
        base, ext = name.rsplit('.', 1)
    else:
        base, ext = name, ''
    base = base[:8].ljust(8)
    ext = ext[:3].ljust(3)
    return (base + ext).encode('ascii')


class Fat32Builder:
    def __init__(self, label, total_sectors):
        self.total_sectors = total_sectors
        self.sectors_per_cluster = 1
        self.reserved_sectors = 32
        self.num_fats = 2
        self.root_cluster = 2
        self.img = bytearray(total_sectors * SECTOR)

        self.fat_entries = (total_sectors - self.reserved_sectors) // self.sectors_per_cluster + 2
        self.fat_size_sectors = max(1, math.ceil(self.fat_entries * 4 / SECTOR))
        self.data_start = self.reserved_sectors + self.fat_size_sectors * self.num_fats

        self.fat = [0] * self.fat_entries
        self.fat[0] = 0x0FFFFFF8
        self.fat[1] = 0x0FFFFFFF

        self._alloc_chain([self.root_cluster])
        self.fat[self.root_cluster] = 0x0FFFFFFF

    def _alloc_cluster(self):
        for i in range(self.root_cluster + 1, self.fat_entries):
            if self.fat[i] == 0:
                self.fat[i] = 1  # mark in-use (will be overwritten by _alloc_chain)
                return i
        return None

    def _alloc_chain(self, clusters):
        for c in clusters:
            self.fat[c] = 0x0FFFFFFF
        for i in range(len(clusters) - 1):
            self.fat[clusters[i]] = clusters[i + 1]
        return clusters

    def _cluster_off(self, cluster):
        return (self.data_start + (cluster - 2) * self.sectors_per_cluster) * SECTOR

    def _find_dir_entry(self, dir_cluster, name_83):
        c = dir_cluster
        for _ in range(100):
            off = self._cluster_off(c)
            for i in range(16 * self.sectors_per_cluster):
                e = off + i * 32
                if self.img[e] == 0x00:
                    return None, -1
                if self.img[e] == 0xE5 or self.img[e + 11] == 0x0F:
                    continue
                if bytes(self.img[e:e+8]) == name_83[:8] and bytes(self.img[e+8:e+11]) == name_83[8:11]:
                    first_clus = struct.unpack_from('<H', self.img, e + 26)[0]
                    first_clus |= (struct.unpack_from('<H', self.img, e + 20)[0] << 16)
                    return e, first_clus
            nxt = self.fat[c]
            if nxt >= 0x0FFFFFF8:
                break
            c = nxt
        return None, -1

    def _add_dir_entry(self, dir_cluster, name_83, attr, first_cluster, size=0):
        c = dir_cluster
        for _ in range(100):
            off = self._cluster_off(c)
            for i in range(16 * self.sectors_per_cluster):
                e = off + i * 32
                if self.img[e] == 0x00 or self.img[e] == 0xE5:
                    self.img[e:e+8] = name_83[:8]
                    self.img[e+8:e+11] = name_83[8:11]
                    self.img[e+11] = attr
                    struct.pack_into('<H', self.img, e + 26, first_cluster & 0xFFFF)
                    struct.pack_into('<H', self.img, e + 20, (first_cluster >> 16) & 0xFFFF)
                    struct.pack_into('<I', self.img, e + 28, size)
                    return True
            nxt = self.fat[c]
            if nxt >= 0x0FFFFFF8:
                break
            c = nxt
        return False

    def _ensure_dir(self, path_parts):
        current = self.root_cluster
        for part in path_parts:
            name83 = name_to_83(part)
            _, clus = self._find_dir_entry(current, name83)
            if clus < 2:
                chain = self._alloc_chain([self._alloc_cluster()])
                self.fat[chain[-1]] = 0x0FFFFFFF
                self._add_dir_entry(current, name83, 0x10, chain[0])
                current = chain[0]
            else:
                current = clus
        return current

    def add_file(self, virtual_path, data):
        parts = [p for p in virtual_path.strip('/').split('/') if p]
        dir_parts = parts[:-1]
        file_name = parts[-1]

        dir_cluster = self._ensure_dir(dir_parts)
        name83 = name_to_83(file_name)

        n_clusters = max(1, math.ceil(len(data) / (self.sectors_per_cluster * SECTOR))) if data else 0
        chain = []
        if n_clusters > 0:
            chain = [self._alloc_cluster() for _ in range(n_clusters)]
            self._alloc_chain(chain)
            self.fat[chain[-1]] = 0x0FFFFFFF

            for ci, c in enumerate(chain):
                off = self._cluster_off(c)
                for s in range(self.sectors_per_cluster):
                    byte_off = ci * self.sectors_per_cluster * SECTOR + s * SECTOR
                    chunk = data[byte_off: byte_off + SECTOR]
                    self.img[off + s * SECTOR: off + s * SECTOR + len(chunk)] = chunk

        first = chain[0] if chain else 0
        self._add_dir_entry(dir_cluster, name83, 0x20, first, len(data))

    def add_dir(self, virtual_path):
        parts = [p for p in virtual_path.strip('/').split('/') if p]
        self._ensure_dir(parts)

    def build(self):
        for i in range(self.num_fats):
            fat_off = (self.reserved_sectors + i * self.fat_size_sectors) * SECTOR
            for j, entry in enumerate(self.fat):
                struct.pack_into('<I', self.img, fat_off + j * 4, entry)
        self._write_bpb()
        return bytes(self.img)

    def _write_bpb(self):
        img = self.img
        o = 0
        img[o:o+3] = b'\xEB\x58\x90'
        img[o+3:o+11] = b'MSWIN4.1'
        struct.pack_into('<H', img, o + 11, SECTOR)
        img[o + 13] = self.sectors_per_cluster
        struct.pack_into('<H', img, o + 14, self.reserved_sectors)
        img[o + 16] = self.num_fats
        img[o + 21] = 0xF8
        struct.pack_into('<H', img, o + 24, 63)
        struct.pack_into('<H', img, o + 26, 255)
        struct.pack_into('<I', img, o + 32, self.total_sectors)
        struct.pack_into('<I', img, o + 36, self.fat_size_sectors)
        struct.pack_into('<H', img, o + 42, 0)
        struct.pack_into('<I', img, o + 44, self.root_cluster)
        struct.pack_into('<H', img, o + 48, 1)
        img[o + 64] = 0x29
        label = b'LUMIEOS   '[:11].ljust(11, b' ')
        img[o + 71:o+82] = label
        img[o + 82:o+90] = b'FAT32   '
        struct.pack_into('<I', img, o + 94, 0)

        fsi_off = SECTOR
        img[fsi_off:fsi_off+4] = b'\x52\x52\x61\x41'
        img[fsi_off+484:fsi_off+488] = b'\x72\x72\x41\x61'
        struct.pack_into('<I', img, fsi_off + 488, 0)
        struct.pack_into('<I', img, fsi_off + 492, 0)
        img[fsi_off+508:fsi_off+512] = b'\x00\x00\x55\xAA'

        sig_off = self.reserved_sectors * SECTOR - 2
        img[sig_off] = 0x55
        img[sig_off + 1] = 0xAA


def make_gpt_disk_image(esp_img_bytes, data_img_bytes, output_path, disk_size_mb=64):
    disk_size_sectors = disk_size_mb * 1024 * 1024 // SECTOR

    header_lba = 1
    backup_lba = disk_size_sectors - 1
    first_usable_lba = 34
    last_usable_lba = backup_lba - 33

    esp_start = align_lba(34)
    esp_sectors = len(esp_img_bytes) // SECTOR
    esp_end = align_lba(esp_start + esp_sectors - 1)

    data_start = align_lba(esp_end + 1)
    data_sectors = len(data_img_bytes) // SECTOR
    data_end = align_lba(data_start + data_sectors - 1)

    disk = bytearray(disk_size_sectors * SECTOR)

    disk[446] = 0x00
    struct.pack_into('<BBBBI', disk, 446, 0x00, 0x01, 0x00, 0xEE, 0xFFFFFFFF)
    struct.pack_into('<I', disk, 454, 1)
    struct.pack_into('<I', disk, 458, disk_size_sectors - 1)
    disk[510] = 0x55
    disk[511] = 0xAA

    esp_guid = bytes([0x12,0x34,0x56,0x78,0xAB,0xCD,0xEF,0x01,
                       0x23,0x45,0x67,0x89,0xAB,0xCD,0xEF,0x01])
    data_guid = bytes([0x12,0x34,0x56,0x78,0xAB,0xCD,0xEF,0x01,
                        0x23,0x45,0x67,0x89,0xAB,0xCD,0xEF,0x02])
    disk_guid = bytes([0xDE,0xAD,0xBE,0xEF,0xCA,0xFE,0xBA,0xBE,
                        0x11,0x22,0x33,0x44,0x55,0x66,0x77,0x88])

    esp_type = bytes([0x28,0x73,0x2A,0xC1,0x1F,0xF8,0xD2,0x11,
                       0xBA,0x4B,0x00,0xA0,0xC9,0x3E,0xC9,0x3B])
    data_type = bytes([0xA2,0xA0,0xD0,0xEB,0xE5,0xB9,0x33,0x44,
                        0x87,0xC0,0x68,0xB6,0xB7,0x26,0x99,0xC7])

    entries_lba = 2
    entries_sectors = 32
    entries_size = entries_sectors * SECTOR

    entries = bytearray(entries_size)

    def write_entry(buf, off, type_g, unique_g, start, end, attrs, name):
        buf[off:off+16] = type_g
        buf[off+16:off+32] = unique_g
        struct.pack_into('<QQ', buf, off + 32, start, end)
        struct.pack_into('<Q', buf, off + 48, attrs)
        n = name.encode('utf-16-le')[:72]
        buf[off+56:off+56+len(n)] = n

    write_entry(entries, 0, esp_type, esp_guid, esp_start, esp_end, 0x01, "EFI System Partition")
    write_entry(entries, 128, data_type, data_guid, data_start, data_end, 0x00, "LumieOS Data")

    entries_crc = crc32(bytes(entries))

    header = bytearray(92)
    header[0:8] = b'EFI PART'
    struct.pack_into('<I', header, 8, 0x00010000)       # Revision
    struct.pack_into('<I', header, 12, 92)              # Header size
    struct.pack_into('<I', header, 16, 0)               # Header CRC32 (computed below)
    struct.pack_into('<I', header, 20, 0)               # Reserved
    struct.pack_into('<Q', header, 24, 1)               # My LBA
    struct.pack_into('<Q', header, 32, backup_lba)      # Alternate LBA
    struct.pack_into('<Q', header, 40, 34)              # First usable LBA
    struct.pack_into('<Q', header, 48, last_usable_lba) # Last usable LBA
    header[56:72] = disk_guid                           # Disk GUID (16 bytes)
    struct.pack_into('<Q', header, 72, entries_lba)     # Partition entries LBA
    struct.pack_into('<I', header, 80, 128)             # Number of entries
    struct.pack_into('<I', header, 84, entries_size)    # Entry size
    struct.pack_into('<I', header, 88, entries_crc)     # Entries CRC32
    struct.pack_into('<I', header, 16, crc32(bytes(header)))  # Header CRC32

    hdr_sector = bytearray(SECTOR)
    hdr_sector[0:92] = header
    disk[header_lba*SECTOR : header_lba*SECTOR+SECTOR] = hdr_sector
    disk[entries_lba*SECTOR : entries_lba*SECTOR+entries_size] = entries
    disk[backup_lba*SECTOR : backup_lba*SECTOR+SECTOR] = hdr_sector
    backup_entries_lba = backup_lba - entries_sectors
    disk[backup_entries_lba*SECTOR : backup_entries_lba*SECTOR+entries_size] = entries

    disk[esp_start*SECTOR : esp_start*SECTOR+len(esp_img_bytes)] = esp_img_bytes
    disk[data_start*SECTOR : data_start*SECTOR+len(data_img_bytes)] = data_img_bytes

    with open(output_path, 'wb') as f:
        f.write(disk)

    return esp_start, esp_end, data_start, data_end


def main():
    import argparse
    parser = argparse.ArgumentParser(description='Create LumieOS bootable disk image')
    parser.add_argument('output', help='Output .img file path')
    parser.add_argument('--size', type=int, default=64, help='Disk size in MB (default: 64)')
    parser.add_argument('--bootloader', default=None, help='Path to BOOTX64.EFI')
    parser.add_argument('--pkg', default=None, help='Path to install.pkg')
    args = parser.parse_args()

    project_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))

    bootloader_path = args.bootloader or os.path.join(project_root, 'build', 'release', 'EFI', 'BOOT', 'BOOTX64.EFI')
    pkg_path = args.pkg or os.path.join(project_root, 'build', 'install.pkg')

    if not os.path.exists(bootloader_path):
        print(f"ERROR: BOOTX64.EFI not found at {bootloader_path}", file=sys.stderr)
        sys.exit(1)
    if not os.path.exists(pkg_path):
        print(f"ERROR: install.pkg not found at {pkg_path}", file=sys.stderr)
        sys.exit(1)

    bootloader_data = open(bootloader_path, 'rb').read()
    pkg_data = open(pkg_path, 'rb').read()

    esp_mb = 32
    data_mb = 32
    esp_sectors = esp_mb * 1024 * 1024 // SECTOR
    data_sectors = data_mb * 1024 * 1024 // SECTOR

    print(f"Creating ESP ({esp_mb} MB)...")
    esp = Fat32Builder('ESP', esp_sectors)
    esp.add_file('/EFI/BOOT/BOOTX64.EFI', bootloader_data)
    esp_img = esp.build()

    print(f"Creating data partition ({data_mb} MB)...")
    dat = Fat32Builder('LUMIEOS', data_sectors)
    dat.add_file('/install.pkg', pkg_data)
    dat_img = dat.build()

    print(f"Creating GPT disk image ({args.size} MB)...")
    esp_s, esp_e, dat_s, dat_e = make_gpt_disk_image(esp_img, dat_img, args.output, args.size)

    sz = os.path.getsize(args.output)
    print(f"\nDone! Created {args.output} ({sz / 1024 / 1024:.1f} MB)")
    print(f"  ESP:      LBA {esp_s} - {esp_e}")
    print(f"  LumieOS:  LBA {dat_s} - {dat_e}")
    print(f"\n  ESP contents:  EFI/BOOT/BOOTX64.EFI ({len(bootloader_data)} bytes)")
    print(f"  Data contents: install.pkg ({len(pkg_data)} bytes)")


if __name__ == '__main__':
    main()
