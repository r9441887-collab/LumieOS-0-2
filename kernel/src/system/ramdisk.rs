use core::ptr;

pub const RAMDISK_SIZE: usize = 2 * 1024 * 1024;
pub const RAMDISK_SECTOR_SIZE: u32 = 512;

static mut G_RAMDISK: [u8; RAMDISK_SIZE] = [0u8; RAMDISK_SIZE];
static mut G_RAMDISK_INITIALIZED: bool = false;
static mut G_RAMDISK_FORMATTED: bool = false;

pub unsafe fn ramdisk_init() {
    if !G_RAMDISK_INITIALIZED {
        ptr::write_bytes(G_RAMDISK.as_mut_ptr(), 0, RAMDISK_SIZE);
        G_RAMDISK_INITIALIZED = true;
        G_RAMDISK_FORMATTED = false;
    }
}

unsafe fn ramdisk_write_sector(lba: u32, data: *const u8) {
    if !G_RAMDISK_INITIALIZED || data.is_null() {
        return;
    }
    let off = (lba as usize) * (RAMDISK_SECTOR_SIZE as usize);
    if off + (RAMDISK_SECTOR_SIZE as usize) > RAMDISK_SIZE {
        return;
    }
    let dst = G_RAMDISK.as_mut_ptr().add(off);
    ptr::copy_nonoverlapping(data, dst, RAMDISK_SECTOR_SIZE as usize);
}

unsafe fn ramdisk_read_sector(lba: u32, data: *mut u8) {
    if !G_RAMDISK_INITIALIZED || data.is_null() {
        return;
    }
    let off = (lba as usize) * (RAMDISK_SECTOR_SIZE as usize);
    if off + (RAMDISK_SECTOR_SIZE as usize) > RAMDISK_SIZE {
        return;
    }
    let src = G_RAMDISK.as_ptr().add(off);
    ptr::copy_nonoverlapping(src, data, RAMDISK_SECTOR_SIZE as usize);
}

fn write_le16(p: &mut [u8], v: u16) {
    p[0] = v as u8;
    p[1] = (v >> 8) as u8;
}

fn write_le32(p: &mut [u8], v: u32) {
    p[0] = v as u8;
    p[1] = (v >> 8) as u8;
    p[2] = (v >> 16) as u8;
    p[3] = (v >> 24) as u8;
}

fn read_le16(p: &[u8]) -> u16 {
    p[0] as u16 | ((p[1] as u16) << 8)
}

fn read_le32(p: &[u8]) -> u32 {
    p[0] as u32 | ((p[1] as u32) << 8) | ((p[2] as u32) << 16) | ((p[3] as u32) << 24)
}

pub unsafe fn ramdisk_format_fat32() -> i32 {
    if !G_RAMDISK_INITIALIZED {
        return -1;
    }
    let mut sector = [0u8; 512];

    let bytes_per_sector: u32 = 512;
    let sectors_per_cluster: u8 = 1;
    let reserved_sectors: u32 = 32;
    let num_fats: u8 = 2;
    let root_entries: u32 = 0;
    let total_sectors_16: u16 = 0;
    let media_descriptor: u8 = 0xF8;
    let sectors_per_fat_16: u16 = 0;
    let sectors_per_track: u16 = 63;
    let num_heads: u16 = 255;
    let hidden_sectors: u32 = 0;
    let total_sectors_32 = (RAMDISK_SIZE / bytes_per_sector as usize) as u32;
    let mut sectors_per_fat_32 = (total_sectors_32 + 2 * 128 - 1) / (128 * 2) + 1;
    if sectors_per_fat_32 < 4 {
        sectors_per_fat_32 = 4;
    }
    let root_cluster: u32 = 2;
    let first_data_sector = reserved_sectors + (num_fats as u32) * sectors_per_fat_32;
    let _total_clusters = (total_sectors_32 - first_data_sector) / (sectors_per_cluster as u32);

    sector[0] = 0xEB;
    sector[1] = 0x58;
    sector[2] = 0x90;
    sector[3..11].copy_from_slice(b"LUMIEOS ");
    write_le16(&mut sector[11..13], bytes_per_sector as u16);
    sector[13] = sectors_per_cluster;
    write_le16(&mut sector[14..16], reserved_sectors as u16);
    sector[16] = num_fats;
    write_le16(&mut sector[17..19], root_entries as u16);
    write_le16(&mut sector[19..21], total_sectors_16);
    sector[21] = media_descriptor;
    write_le16(&mut sector[22..24], sectors_per_fat_16);
    write_le16(&mut sector[24..26], sectors_per_track);
    write_le16(&mut sector[26..28], num_heads);
    write_le32(&mut sector[28..32], hidden_sectors);
    write_le32(&mut sector[32..36], total_sectors_32);
    write_le32(&mut sector[36..40], sectors_per_fat_32);
    write_le16(&mut sector[40..42], 0);
    write_le16(&mut sector[42..44], 0);
    write_le32(&mut sector[44..48], root_cluster);
    write_le16(&mut sector[48..50], 1);
    write_le16(&mut sector[50..52], 6);
    sector[52] = 0;
    sector[53] = 0;
    sector[54] = 0;
    sector[55] = 0;
    sector[56] = 0x80;
    sector[57] = 0;
    write_le32(&mut sector[58..62], 0);
    write_le32(&mut sector[62..66], 0);
    write_le16(&mut sector[510..512], 0x55AA);

    ramdisk_write_sector(0, sector.as_ptr());

    for i in 1..reserved_sectors {
        sector.fill(0);
        ramdisk_write_sector(i, sector.as_ptr());
    }

    let fat_sectors = sectors_per_fat_32;
    sector.fill(0);
    write_le32(&mut sector[0..4], 0x0FFFFFF8 | ((media_descriptor as u32) << 24));
    write_le32(&mut sector[4..8], 0x0FFFFFFF);
    write_le32(&mut sector[8..12], 0x0FFFFFFF);
    ramdisk_write_sector(reserved_sectors, sector.as_ptr());

    for i in (reserved_sectors + 1)..(reserved_sectors + fat_sectors) {
        sector.fill(0);
        ramdisk_write_sector(i, sector.as_ptr());
    }

    let fat2_start = reserved_sectors + fat_sectors;
    for i in 0..fat_sectors {
        ramdisk_read_sector(reserved_sectors + i, sector.as_mut_ptr());
        ramdisk_write_sector(fat2_start + i, sector.as_ptr());
    }

    let root_sector = first_data_sector;
    sector.fill(0);
    ramdisk_write_sector(root_sector, sector.as_ptr());

    G_RAMDISK_FORMATTED = true;
    0
}

pub unsafe fn ramdisk_read_sector_cb(lba: u32, count: u32, buf: *mut core::ffi::c_void) -> i32 {
    let data = buf as *mut u8;
    for i in 0..count {
        ramdisk_read_sector(lba + i, data.add(i as usize * RAMDISK_SECTOR_SIZE as usize));
    }
    0
}

pub unsafe fn ramdisk_write_sector_cb(lba: u32, count: u32, buf: *mut core::ffi::c_void) -> i32 {
    let data = buf as *const u8;
    for i in 0..count {
        ramdisk_write_sector(lba + i, data.add(i as usize * RAMDISK_SECTOR_SIZE as usize));
    }
    0
}
