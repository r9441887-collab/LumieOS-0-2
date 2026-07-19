import struct, sys

with open('build/lumieos-installer.img', 'rb') as f:
    d = f.read()

print('=== GPT Header (LBA 1) ===')
hdr = d[512:1024]
print(f'  Magic: {hdr[0:8]}')

e1 = d[1024:1024+128]
p1_start = struct.unpack_from('<Q', e1, 32)[0]
p1_end = struct.unpack_from('<Q', e1, 40)[0]
e2 = d[1024+128:1024+256]
p2_start = struct.unpack_from('<Q', e2, 32)[0]
p2_end = struct.unpack_from('<Q', e2, 40)[0]
print(f'  ESP: LBA {p1_start}-{p1_end}  Data: LBA {p2_start}-{p2_end}')

def scan_fat32(disk, part_lba, label):
    base = part_lba * 512
    bpb_off = base
    fat_size = struct.unpack_from('<I', disk, bpb_off+36)[0]
    reserved = struct.unpack_from('<H', disk, bpb_off+14)[0]
    num_fats = disk[bpb_off+16]
    spc = disk[bpb_off+13]
    vol_label = disk[bpb_off+71:bpb_off+82].decode('ascii', errors='replace').strip()
    data_start_lba = reserved + fat_size * num_fats
    print(f'\n=== {label} FAT32 (label={vol_label}) ===')
    
    def read_dir(cluster, indent):
        dir_off = (base + (data_start_lba + (cluster - 2) * spc) * 512)
        for i in range(16 * spc):
            e = dir_off + i * 32
            if disk[e] == 0x00:
                return
            if disk[e] == 0xE5:
                continue
            if disk[e+11] == 0x0F:
                continue
            name = disk[e:e+8].decode('ascii', errors='replace').rstrip()
            ext = disk[e+8:e+11].decode('ascii', errors='replace').rstrip()
            attr = disk[e+11]
            fclus = struct.unpack_from('<H', disk, e+26)[0] | (struct.unpack_from('<H', disk, e+20)[0] << 16)
            fsize = struct.unpack_from('<I', disk, e+28)[0]
            fname = f'{name}.{ext}' if ext else name
            tag = 'DIR' if attr & 0x10 else 'FILE'
            print(f'{indent}[{tag}] {fname} cluster={fclus} size={fsize}')
            if attr & 0x10 and fclus >= 2:
                read_dir(fclus, indent + '  ')

    root_clus = struct.unpack_from('<I', disk, bpb_off+44)[0]
    print(f'  Root cluster: {root_clus}')
    read_dir(root_clus, '  ')

    fat_off = base + reserved * 512
    for c in range(2, min(10, (fat_size * 512) // 4)):
        entry = struct.unpack_from('<I', disk, fat_off + c * 4)[0]
        print(f'  FAT[{c}] = {entry:#010x}')

scan_fat32(d, p1_start, 'ESP')
scan_fat32(d, p2_start, 'Data')
print(f'\nImage: {len(d)} bytes')
