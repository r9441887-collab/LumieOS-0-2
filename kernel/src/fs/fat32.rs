
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::ptr;

use crate::uefi::guid::EfiGuid;
use crate::uefi::guid::{
    EFI_BLOCK_IO_GUID, EFI_LOADED_IMAGE_PROTOCOL_GUID,
};
use crate::uefi::protocols::block_io::EfiBlockIoProtocol;
use crate::uefi::protocols::loaded_image::EfiLoadedImageProtocol;
use crate::uefi::tables::{EfiBootServices, EfiSystemTable};
use crate::uefi::types::*;

use crate::fs::bpb::*;
use crate::fs::diskio::*;
use crate::fs::types::LumieDirEnt;
use crate::drivers::ahci as ahci_drv;

unsafe fn kmalloc(size: usize) -> *mut u8 {
    crate::mm::heap::kmalloc(size as u64)
}
unsafe fn kfree(ptr: *mut u8) {
    crate::mm::heap::kfree(ptr)
}

#[allow(dead_code)]
const EFI_LOCATE_BY_PROTOCOL: u32 = 0;

pub struct Fat32 {
    pub bpb: FatBpb,
    pub initialized: bool,
    pub first_data_sector: u32,
    pub root_dir_sectors: u32,
    pub total_clusters: u32,
    pub fat_size: u32,
    pub disk_io: DiskIo,
}

static mut FAT_DRIVER: MaybeUninit<Fat32> = MaybeUninit::uninit();
static mut G_BS: *mut EfiBootServices = ptr::null_mut();
static mut G_IMAGE: efi_handle = ptr::null_mut();
static mut G_ST: *mut EfiSystemTable = ptr::null_mut();

#[inline]
unsafe fn fat() -> &'static mut Fat32 {
    &mut *FAT_DRIVER.as_mut_ptr()
}

unsafe fn read_sectors(lba: u32, count: u32, buffer: *mut u8) -> i32 {
    let disk = (*fat()).disk_io;
    if let Some(read) = disk.read_cb {
        return read(lba, count, buffer);
    }
    if disk.use_ahci {
        return ahci_drv::read_sectors(lba, count, buffer);
    }
    if let Some(block_io) = disk.block_io {
        let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
        let sector_size = if bpb.bytes_per_sector != 0 {
            bpb.bytes_per_sector as u64
        } else {
            512
        };
        let media = (*block_io).media;
        let status = ((*block_io).read_blocks.unwrap())(
            block_io as *mut c_void,
            (*media).media_id,
            lba as u64,
            (count as u64) * sector_size,
            buffer as *mut c_void,
        );
        if efi_error(status) {
            return -1;
        }
        return 0;
    }
    -1
}

unsafe fn write_sectors(lba: u32, count: u32, buffer: *const u8) -> i32 {
    let disk = (*fat()).disk_io;
    if let Some(write) = disk.write_cb {
        return write(lba, count, buffer);
    }
    if disk.use_ahci {
        return ahci_drv::write_sectors(lba, count, buffer);
    }
    if let Some(block_io) = disk.block_io {
        let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
        let sector_size = if bpb.bytes_per_sector != 0 {
            bpb.bytes_per_sector as u64
        } else {
            512
        };
        let media = (*block_io).media;
        let status = ((*block_io).write_blocks.unwrap())(
            block_io as *mut c_void,
            (*media).media_id,
            lba as u64,
            (count as u64) * sector_size,
            buffer as *mut c_void,
        );
        if efi_error(status) {
            return -1;
        }
        return 0;
    }
    -1
}

unsafe fn fat_read_fat_entry(cluster: u32) -> u32 {
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let fat_offset = cluster * 4;
    let fat_sector = (bpb.reserved_sectors as u32) + (fat_offset / bpb.bytes_per_sector as u32);
    let byte_offset = (fat_offset % bpb.bytes_per_sector as u32) as usize;
    let sector = kmalloc(bpb.bytes_per_sector as usize);
    if sector.is_null() {
        return 0xFFFFFFFF;
    }
    if read_sectors(fat_sector, 1, sector) != 0 {
        kfree(sector);
        return 0xFFFFFFFF;
    }
    let val = ptr::read_unaligned(sector.add(byte_offset) as *const u32) & 0x0FFFFFFF;
    kfree(sector);
    val
}

unsafe fn fat_write_fat_entry(cluster: u32, value: u32) -> i32 {
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let fat_offset = cluster * 4;
    let fat_sector = (bpb.reserved_sectors as u32) + (fat_offset / bpb.bytes_per_sector as u32);
    let byte_offset = (fat_offset % bpb.bytes_per_sector as u32) as usize;
    let sector = kmalloc(bpb.bytes_per_sector as usize);
    if sector.is_null() {
        return -1;
    }
    if read_sectors(fat_sector, 1, sector) != 0 {
        kfree(sector);
        return -1;
    }
    let orig = ptr::read_unaligned(sector.add(byte_offset) as *const u32);
    let new_val = (orig & 0xF0000000) | (value & 0x0FFFFFFF);
    ptr::write_unaligned(sector.add(byte_offset) as *mut u32, new_val);
    for fat_idx in 0..bpb.num_fats {
        let fs = fat_sector + (fat_idx as u32) * bpb.sectors_per_fat_32;
        if write_sectors(fs, 1, sector) != 0 {
            kfree(sector);
            return -1;
        }
    }
    kfree(sector);
    0
}

unsafe fn fat_cluster_to_sector(cluster: u32) -> u32 {
    if cluster < 2 {
        return 0;
    }
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let first_data = (*fat()).first_data_sector;
    first_data + (cluster - 2) * bpb.sectors_per_cluster as u32
}

unsafe fn fat_get_next_cluster(cluster: u32) -> u32 {
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let fat_offset = cluster * 4;
    let fat_sector = (bpb.reserved_sectors as u32) + (fat_offset / bpb.bytes_per_sector as u32);
    let byte_offset = (fat_offset % bpb.bytes_per_sector as u32) as usize;
    let sector = kmalloc(bpb.bytes_per_sector as usize);
    if sector.is_null() {
        return FAT_END_OF_CHAIN;
    }
    if read_sectors(fat_sector, 1, sector) != 0 {
        kfree(sector);
        return FAT_END_OF_CHAIN;
    }
    let next = ptr::read_unaligned(sector.add(byte_offset) as *const u32) & 0x0FFFFFFF;
    kfree(sector);
    next
}

unsafe fn fat_read_cluster(cluster: u32, buffer: *mut u8) -> i32 {
    let sector = fat_cluster_to_sector(cluster);
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    read_sectors(sector, bpb.sectors_per_cluster as u32, buffer)
}

unsafe fn parse_filename_char(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' {
        return c - b'A' + b'a';
    }
    c
}

unsafe fn compare_filename(fat_name: *const u8, name: *const u8) -> i32 {
    let mut name_len: usize = 0;
    while ptr::read(name.add(name_len)) != 0 && ptr::read(name.add(name_len)) != b'.' {
        name_len += 1;
    }
    for i in 0..8 {
        let fc = if i < name_len {
            ptr::read(name.add(i))
        } else {
            b' '
        };
        if parse_filename_char(ptr::read(fat_name.add(i))) != parse_filename_char(fc) {
            return 0;
        }
    }
    let mut ext_len: usize = 0;
    if ptr::read(name.add(name_len)) == b'.' {
        while ptr::read(name.add(name_len + 1 + ext_len)) != 0 {
            ext_len += 1;
        }
    }
    for i in 0..3 {
        let fc = if i < ext_len {
            ptr::read(name.add(name_len + 1 + i))
        } else {
            b' '
        };
        if parse_filename_char(ptr::read(fat_name.add(8 + i))) != parse_filename_char(fc) {
            return 0;
        }
    }
    1
}

unsafe fn dir_name_to_str(fat_name: *const u8, out: *mut u8) {
    let mut oi: usize = 0;
    let mut trailing: i32 = 1;
    let mut i: i32 = 9;
    while i >= 0 {
        if ptr::read(fat_name.add(i as usize)) != b' ' {
            trailing = 0;
            break;
        }
        i -= 1;
    }
    if trailing != 0 && ptr::read(fat_name.add(10)) == b' ' {
        let mut j = 0;
        while j < 8 && ptr::read(fat_name.add(j)) != b' ' {
            ptr::write(out.add(oi), parse_filename_char(ptr::read(fat_name.add(j))));
            oi += 1;
            j += 1;
        }
        ptr::write(out.add(oi), 0);
        return;
    }
    let mut j = 0;
    while j < 8 && ptr::read(fat_name.add(j)) != b' ' {
        ptr::write(out.add(oi), parse_filename_char(ptr::read(fat_name.add(j))));
        oi += 1;
        j += 1;
    }
    if ptr::read(fat_name.add(8)) != b' ' {
        ptr::write(out.add(oi), b'.');
        oi += 1;
        let mut j = 8;
        while j < 11 && ptr::read(fat_name.add(j)) != b' ' {
            ptr::write(out.add(oi), parse_filename_char(ptr::read(fat_name.add(j))));
            oi += 1;
            j += 1;
        }
    }
    ptr::write(out.add(oi), 0);
}

unsafe fn fat_create_8dot3_name(name: *const u8, fat_name: *mut u8) -> i32 {
    let mut name_len: usize = 0;
    while ptr::read(name.add(name_len)) != 0 && ptr::read(name.add(name_len)) != b'.' {
        name_len += 1;
    }
    let mut ext_len: usize = 0;
    if ptr::read(name.add(name_len)) == b'.' {
        while ptr::read(name.add(name_len + 1 + ext_len)) != 0 {
            ext_len += 1;
        }
    }
    for i in 0..8 {
        if i < name_len {
            let mut c = ptr::read(name.add(i));
            if c >= b'a' && c <= b'z' {
                c = c - b'a' + b'A';
            }
            ptr::write(fat_name.add(i), c);
        } else {
            ptr::write(fat_name.add(i), b' ');
        }
    }
    for i in 0..3 {
        if i < ext_len {
            let mut c = ptr::read(name.add(name_len + 1 + i));
            if c >= b'a' && c <= b'z' {
                c = c - b'a' + b'A';
            }
            ptr::write(fat_name.add(8 + i), c);
        } else {
            ptr::write(fat_name.add(8 + i), b' ');
        }
    }
    11
}

unsafe fn init_bpb() -> i32 {
    let mut sector: [u8; 512] = [0; 512];
    if read_sectors(0, 1, sector.as_mut_ptr()) != 0 {
        return -1;
    }
    let bpb: FatBpb = ptr::read_unaligned(sector.as_ptr() as *const FatBpb);
    if bpb.sectors_per_fat_32 == 0 {
        return -1;
    }
    ptr::write_unaligned(&mut (*fat()).bpb as *mut FatBpb, bpb);
    (*fat()).fat_size = bpb.sectors_per_fat_32 * bpb.bytes_per_sector as u32;
    (*fat()).root_dir_sectors = 0;
    (*fat()).first_data_sector =
        bpb.reserved_sectors as u32 + (bpb.num_fats as u32) * bpb.sectors_per_fat_32;
    (*fat()).total_clusters =
        (bpb.total_sectors_32 - (*fat()).first_data_sector) / bpb.sectors_per_cluster as u32;
    0
}

unsafe fn fat_find_cluster(path: *const u8, out_ent: *mut FatDirEnt) -> i32 {
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let cluster_size = (bpb.sectors_per_cluster as u32) * bpb.bytes_per_sector as u32;
    let sector = kmalloc(cluster_size as usize);
    if sector.is_null() {
        return 0;
    }
    let mut p = path;
    if ptr::read(p) == b'/' {
        p = p.add(1);
    }
    let mut cluster: u32 = bpb.root_cluster;
    let orig_p = p;
    let mut path_len: usize = 0;
    while ptr::read(orig_p.add(path_len)) != 0 {
        path_len += 1;
    }
    if path_len == 0 {
        kfree(sector);
        let ent = &mut *out_ent;
        ptr::write_unaligned(core::ptr::addr_of_mut!(ent.cluster_low), (cluster & 0xFFFF) as u16);
        ptr::write_unaligned(
            core::ptr::addr_of_mut!(ent.cluster_high),
            ((cluster >> 16) & 0xFFFF) as u16,
        );
        ptr::write_unaligned(core::ptr::addr_of_mut!(ent.attr), FAT_ATTR_DIRECTORY);
        ptr::write_unaligned(core::ptr::addr_of_mut!(ent.size), 0);
        return 1;
    }
    let mut component: [u8; 256] = [0; 256];
    loop {
        while ptr::read(p) == b'/' {
            p = p.add(1);
        }
        let mut ci: usize = 0;
        while ptr::read(p) != 0 && ptr::read(p) != b'/' && ci < 255 {
            ptr::write(component.as_mut_ptr().add(ci), ptr::read(p));
            ci += 1;
            p = p.add(1);
        }
        ptr::write(component.as_mut_ptr().add(ci), 0);
        if ci == 0 {
            break;
        }
        let mut found: i32 = 0;
        while cluster >= 2 && cluster < FAT_END_OF_CHAIN {
            if fat_read_cluster(cluster, sector) != 0 {
                break;
            }
            let entries_per_cluster = cluster_size / 32;
            let dent = sector as *const FatDirEnt;
            let mut i: u32 = 0;
            while i < entries_per_cluster {
                let name0 = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).name));
                if name0[0] == 0 {
                    break;
                }
                if name0[0] == 0xE5 {
                    i += 1;
                    continue;
                }
                let attr = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).attr));
                if (attr & FAT_ATTR_LFN) == FAT_ATTR_LFN {
                    i += 1;
                    continue;
                }
                if compare_filename(
                    (*dent.add(i as usize)).name.as_ptr(),
                    component.as_mut_ptr(),
                ) != 0
                {
                    let cl = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).cluster_low)) as u32;
                    let ch =
                        ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).cluster_high)) as u32;
                    let next_cluster = cl | (ch << 16);
                    ptr::copy_nonoverlapping(
                        dent.add(i as usize) as *const u8,
                        out_ent as *mut u8,
                        core::mem::size_of::<FatDirEnt>(),
                    );
                    if ptr::read(p) == 0 {
                        kfree(sector);
                        return 1;
                    }
                    if (ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).attr)) & FAT_ATTR_DIRECTORY)
                        != 0
                    {
                        cluster = next_cluster;
                        found = 1;
                    } else {
                        kfree(sector);
                        return 0;
                    }
                    break;
                }
                i += 1;
            }
            if found != 0 {
                break;
            }
            cluster = fat_get_next_cluster(cluster);
        }
        if found == 0 {
            kfree(sector);
            return 0;
        }
    }
    kfree(sector);
    1
}

unsafe fn fat_find_dir_slot(
    dir_cluster: u32,
    name: *const u8,
    out_ent: *mut FatDirEnt,
    out_sector: *mut u32,
    out_offset: *mut u32,
) -> i32 {
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let cluster_size = (bpb.sectors_per_cluster as u32) * bpb.bytes_per_sector as u32;
    let sector = kmalloc(cluster_size as usize);
    if sector.is_null() {
        return 0;
    }
    let mut cluster = dir_cluster;
    let entries_per_sector = (bpb.bytes_per_sector as u32) / 32;
    let mut cluster_start = fat_cluster_to_sector(cluster);
    let mut fat_name: [u8; 11] = [0; 11];
    fat_create_8dot3_name(name, fat_name.as_mut_ptr());
    while cluster >= 2 && cluster < FAT_END_OF_CHAIN {
        if fat_read_cluster(cluster, sector) != 0 {
            break;
        }
        let entries_per_cluster = cluster_size / 32;
        let dent = sector as *const FatDirEnt;
        let mut i: u32 = 0;
        while i < entries_per_cluster {
            let sector_in_cluster = i / entries_per_sector;
            let entry_sector = cluster_start + sector_in_cluster;
            let entry_byte_offset = (i % entries_per_sector) * 32;
            let name0 = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).name));
            if name0[0] == 0 || name0[0] == 0xE5 {
                ptr::copy_nonoverlapping(
                    dent.add(i as usize) as *const u8,
                    out_ent as *mut u8,
                    core::mem::size_of::<FatDirEnt>(),
                );
                ptr::write(out_sector, entry_sector);
                ptr::write(out_offset, entry_byte_offset);
                kfree(sector);
                return 1;
            }
            let attr = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).attr));
            if (attr & FAT_ATTR_LFN) == FAT_ATTR_LFN {
                i += 1;
                continue;
            }
            if memcmp(
                (*dent.add(i as usize)).name.as_ptr(),
                fat_name.as_ptr(),
                11,
            ) == 0
            {
                ptr::copy_nonoverlapping(
                    dent.add(i as usize) as *const u8,
                    out_ent as *mut u8,
                    core::mem::size_of::<FatDirEnt>(),
                );
                ptr::write(out_sector, entry_sector);
                ptr::write(out_offset, entry_byte_offset);
                kfree(sector);
                return 2;
            }
            i += 1;
        }
        cluster = fat_get_next_cluster(cluster);
        cluster_start = fat_cluster_to_sector(cluster);
    }
    kfree(sector);
    0
}

unsafe fn fat_extend_directory(dir_cluster: u32) -> u32 {
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let cluster_size = (bpb.sectors_per_cluster as u32) * bpb.bytes_per_sector as u32;
    let fat_size = (*fat()).fat_size;
    let fat_entries = fat_size / 4;
    let zero_buf = kmalloc(cluster_size as usize);
    if zero_buf.is_null() {
        return !0u32;
    }
    let mut i: u32 = 2;
    while i < fat_entries {
        if fat_read_fat_entry(i) == 0 {
            ptr::write_bytes(zero_buf, 0, cluster_size as usize);
            let sector = fat_cluster_to_sector(i);
            if write_sectors(sector, bpb.sectors_per_cluster as u32, zero_buf) != 0 {
                kfree(zero_buf);
                return !0u32;
            }
            if fat_write_fat_entry(i, 0x0FFFFFFF) != 0 {
                kfree(zero_buf);
                return !0u32;
            }
            let mut c = dir_cluster;
            while c >= 2 {
                let next = fat_read_fat_entry(c);
                if next >= FAT_END_OF_CHAIN {
                    fat_write_fat_entry(c, i);
                    break;
                }
                if next < 2 {
                    break;
                }
                c = next;
            }
            kfree(zero_buf);
            return i;
        }
        i += 1;
    }
    kfree(zero_buf);
    !0u32
}

unsafe fn strlen(s: *const u8) -> usize {
    let mut len: usize = 0;
    while ptr::read(s.add(len)) != 0 {
        len += 1;
    }
    len
}

unsafe fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i: usize = 0;
    while i < n {
        let ca = ptr::read(a.add(i));
        let cb = ptr::read(b.add(i));
        if ca != cb {
            return if ca < cb { -1 } else { 1 };
        }
        i += 1;
    }
    0
}

pub unsafe fn init() -> i32 {
    let bs = G_BS;
    if bs.is_null() {
        return -1;
    }
    let block_io_guid = EFI_BLOCK_IO_GUID;
    let loaded_image_guid = EFI_LOADED_IMAGE_PROTOCOL_GUID;
    let mut loaded_image: *mut EfiLoadedImageProtocol = ptr::null_mut();
    let status = ((*bs).handle_protocol.unwrap())(
        G_IMAGE,
        &loaded_image_guid as *const EfiGuid as *mut EfiGuid,
        &mut loaded_image as *mut *mut EfiLoadedImageProtocol as *mut *mut c_void,
    );
    if efi_error(status) {
        return -1;
    }
    let mut block_io: *mut EfiBlockIoProtocol = ptr::null_mut();
    let status = ((*bs).handle_protocol.unwrap())(
        (*loaded_image).device_handle,
        &block_io_guid as *const EfiGuid as *mut EfiGuid,
        &mut block_io as *mut *mut EfiBlockIoProtocol as *mut *mut c_void,
    );
    if efi_error(status) {
        return -1;
    }
    (*fat()).disk_io.block_io = Some(block_io);
    if init_bpb() != 0 {
        return -1;
    }
    (*fat()).initialized = true;
    0
}

pub unsafe fn reinit() -> i32 {
    (*fat()).initialized = false;
    if init_bpb() != 0 {
        return -1;
    }
    (*fat()).initialized = true;
    0
}

pub unsafe fn format(total_sectors: u64) -> i32 {
    format_at(0, total_sectors)
}

pub unsafe fn format_at(start_lba: u64, total_sectors: u64) -> i32 {
    let mut sector: [u8; 512] = [0; 512];
    sector[0] = 0xEB;
    sector[1] = 0x58;
    sector[2] = 0x90;
    let _b = sector.as_mut_ptr() as *mut FatBpb;
    let bpb_src = FatBpb {
        jmp: [0xEB, 0x58, 0x90],
        oem: *b"LUMIEOS ",
        bytes_per_sector: 512,
        sectors_per_cluster: 1,
        reserved_sectors: 32,
        num_fats: 2,
        root_entries: 0,
        total_sectors_16: 0,
        media_descriptor: 0xF8,
        sectors_per_fat_16: 0,
        sectors_per_track: 0x3F,
        num_heads: 0xFF,
        hidden_sectors: 0,
        total_sectors_32: total_sectors as u32,
        sectors_per_fat_32: 0,
        ext_flags: 0,
        fs_version: 0,
        root_cluster: 2,
        fs_info: 0xFFFF,
        backup_boot_sector: 6,
        reserved: [0; 12],
        drive_number: 0x80,
        reserved1: 0,
        boot_signature: 0x29,
        volume_id: 0,
        volume_label: *b"LUMIEOS    ",
        fs_type: *b"FAT32   ",
    };

    let mut fat32_calc_sectors_per_fat =
        ((total_sectors / (128 * bpb_src.sectors_per_cluster as u64) + 32) & !1) as u32;
    if fat32_calc_sectors_per_fat < 2 {
        fat32_calc_sectors_per_fat = 2;
    }

    ptr::copy_nonoverlapping(
        &bpb_src as *const FatBpb as *const u8,
        sector.as_mut_ptr(),
        core::mem::size_of::<FatBpb>(),
    );
    let b = sector.as_mut_ptr() as *mut FatBpb;
    let spf_ptr = core::ptr::addr_of_mut!((*b).sectors_per_fat_32);
    ptr::write_unaligned(spf_ptr, fat32_calc_sectors_per_fat);

    sector[510] = 0x55;
    sector[511] = 0xAA;

    if write_sectors(start_lba as u32 + 0, 1, sector.as_mut_ptr()) != 0 {
        return -1;
    }
    let zero_buf = kmalloc(512);
    if zero_buf.is_null() {
        return -1;
    }
    ptr::write_bytes(zero_buf, 0, 512);
    let mut i: u32 = 1;
    while i < 32 {
        if write_sectors(start_lba as u32 + i, 1, zero_buf) != 0 {
            kfree(zero_buf);
            return -1;
        }
        i += 1;
    }
    let fat_sector = kmalloc(512);
    if fat_sector.is_null() {
        kfree(zero_buf);
        return -1;
    }
    ptr::write_bytes(fat_sector, 0, 512);
    ptr::write_unaligned(fat_sector.add(0) as *mut u32, 0x0FFFFFF8);
    ptr::write_unaligned(fat_sector.add(4) as *mut u32, 0x0FFFFFFF);
    ptr::write_unaligned(fat_sector.add(8) as *mut u32, 0x0FFFFFFF);
    let num_fats = 2;
    for f in 0..num_fats {
        let fat_start = 32 + (f as u32) * fat32_calc_sectors_per_fat;
        if write_sectors(start_lba as u32 + fat_start, 1, fat_sector) != 0 {
            kfree(fat_sector);
            kfree(zero_buf);
            return -1;
        }
        let mut j: u32 = 1;
        while j < fat32_calc_sectors_per_fat {
            if write_sectors(start_lba as u32 + fat_start + j, 1, zero_buf) != 0 {
                kfree(fat_sector);
                kfree(zero_buf);
                return -1;
            }
            j += 1;
        }
    }
    kfree(fat_sector);
    let root_sector = 32 + num_fats * fat32_calc_sectors_per_fat;
    if write_sectors(start_lba as u32 + root_sector, 1, zero_buf) != 0 {
        kfree(zero_buf);
        return -1;
    }
    kfree(zero_buf);
    if reinit() != 0 {
        return -1;
    }
    0
}

pub unsafe fn read_file(path: *const u8, buffer: *mut u8, max_size: u32) -> i32 {
    if !(*fat()).initialized {
        return -1;
    }
    let mut ent: FatDirEnt = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    if fat_find_cluster(path, &mut ent as *mut FatDirEnt) == 0 {
        return -1;
    }
    let attr = ptr::read_unaligned(core::ptr::addr_of!(ent.attr));
    if (attr & FAT_ATTR_DIRECTORY) != 0 {
        return -1;
    }
    let cl = ptr::read_unaligned(core::ptr::addr_of!(ent.cluster_low)) as u32;
    let ch = ptr::read_unaligned(core::ptr::addr_of!(ent.cluster_high)) as u32;
    let mut cluster = cl | (ch << 16);
    let size = ptr::read_unaligned(core::ptr::addr_of!(ent.size));
    let read_size = if size < max_size { size } else { max_size };
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let cluster_size = (bpb.sectors_per_cluster as u32) * bpb.bytes_per_sector as u32;
    let temp = kmalloc(cluster_size as usize);
    if temp.is_null() {
        return -1;
    }
    let mut offset: u32 = 0;
    while cluster < FAT_END_OF_CHAIN && offset < read_size {
        if fat_read_cluster(cluster, temp) != 0 {
            break;
        }
        let to_copy = if (read_size - offset) < cluster_size {
            read_size - offset
        } else {
            cluster_size
        };
        ptr::copy_nonoverlapping(temp, buffer.add(offset as usize), to_copy as usize);
        offset += to_copy;
        cluster = fat_get_next_cluster(cluster);
    }
    kfree(temp);
    offset as i32
}

pub unsafe fn write_file(path: *const u8, data: *const u8, size: u32) -> i32 {
    if !(*fat()).initialized {
        return -1;
    }
    let path_len = strlen(path);
    let mut last_slash: i32 = -1;
    let mut i: usize = 0;
    while i < path_len {
        if ptr::read(path.add(i)) == b'/' {
            last_slash = i as i32;
        }
        i += 1;
    }
    let mut dir_path: [u8; 256] = [0; 256];
    let mut fname: [u8; 256] = [0; 256];
    if last_slash < 0 {
        let src = b"/\0";
        ptr::copy_nonoverlapping(src.as_ptr(), dir_path.as_mut_ptr(), 2);
        let mut fnlen = path_len;
        if fnlen >= 256 {
            fnlen = 255;
        }
        ptr::copy_nonoverlapping(path, fname.as_mut_ptr(), fnlen);
        ptr::write(fname.as_mut_ptr().add(fnlen), 0);
    } else {
        let len = last_slash as usize;
        let actual_len = if len >= 256 { 255 } else { len };
        ptr::copy_nonoverlapping(path, dir_path.as_mut_ptr(), actual_len);
        ptr::write(dir_path.as_mut_ptr().add(actual_len), 0);
        if actual_len == 0 {
            ptr::write(dir_path.as_mut_ptr(), b'/');
            ptr::write(dir_path.as_mut_ptr().add(1), 0);
        }
        let mut fnlen = path_len - (last_slash as usize) - 1;
        if fnlen >= 256 {
            fnlen = 255;
        }
        ptr::copy_nonoverlapping(
            path.add(last_slash as usize + 1),
            fname.as_mut_ptr(),
            fnlen,
        );
        ptr::write(fname.as_mut_ptr().add(fnlen), 0);
    }
    let mut parent_ent: FatDirEnt =
        ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    let parent_cluster: u32;
    if fat_find_cluster(dir_path.as_mut_ptr(), &mut parent_ent as *mut FatDirEnt) != 0 {
        let pa = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.attr));
        if (pa & FAT_ATTR_DIRECTORY) == 0 {
            return -1;
        }
        let pcl = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.cluster_low)) as u32;
        let pch = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.cluster_high)) as u32;
        parent_cluster = pcl | (pch << 16);
    } else {
        return -1;
    }
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let cluster_size = (bpb.sectors_per_cluster as u32) * bpb.bytes_per_sector as u32;
    let mut parent_cluster = parent_cluster;
    if parent_cluster == 0 {
        parent_cluster = bpb.root_cluster;
    }
    let needed = (size + cluster_size - 1) / cluster_size;
    let needed = if needed == 0 { 1 } else { needed };
    if needed > 256 {
        return -1;
    }
    let mut entry_sector: u32 = 0;
    let mut entry_offset: u32 = 0;
    let mut entry_buf: FatDirEnt =
        ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    let mut slot_type = fat_find_dir_slot(
        parent_cluster,
        fname.as_mut_ptr(),
        &mut entry_buf as *mut FatDirEnt,
        &mut entry_sector as *mut u32,
        &mut entry_offset as *mut u32,
    );
    if slot_type == 0 {
        let new_cluster = fat_extend_directory(parent_cluster);
        if new_cluster == !0u32 {
            return -1;
        }
        slot_type = fat_find_dir_slot(
            parent_cluster,
            fname.as_mut_ptr(),
            &mut entry_buf as *mut FatDirEnt,
            &mut entry_sector as *mut u32,
            &mut entry_offset as *mut u32,
        );
        if slot_type == 0 {
            return -1;
        }
    }
    let file_exists = slot_type == 2;
    if file_exists {
        let ecl = ptr::read_unaligned(core::ptr::addr_of!(entry_buf.cluster_low)) as u32;
        let ech = ptr::read_unaligned(core::ptr::addr_of!(entry_buf.cluster_high)) as u32;
        let mut old_cluster = ecl | (ech << 16);
        if old_cluster >= 2 {
            while old_cluster >= 2 && old_cluster < FAT_END_OF_CHAIN {
                let next = fat_read_fat_entry(old_cluster);
                fat_write_fat_entry(old_cluster, 0);
                old_cluster = next;
            }
        }
    }
    let mut clusters: [u32; 256] = [0; 256];
    let mut found: u32 = 0;
    let fat_size = (*fat()).fat_size;
    let fat_entries = fat_size / 4;
    let mut i: u32 = 2;
    while i < fat_entries && found < needed {
        if fat_read_fat_entry(i) == 0 {
            clusters[found as usize] = i;
            found += 1;
        }
        i += 1;
    }
    if found < needed {
        return -1;
    }
    i = 0;
    while i < needed {
        let next = if i < needed - 1 {
            clusters[(i + 1) as usize]
        } else {
            0x0FFFFFFF
        };
        if fat_write_fat_entry(clusters[i as usize], next) != 0 {
            return -1;
        }
        i += 1;
    }
    let mut offset: u32 = 0;
    let temp = kmalloc(cluster_size as usize);
    if temp.is_null() {
        return -1;
    }
    i = 0;
    while i < needed && offset < size {
        let to_write = if size - offset > cluster_size {
            cluster_size
        } else {
            size - offset
        };
        ptr::copy_nonoverlapping(data.add(offset as usize), temp, to_write as usize);
        if to_write < cluster_size {
            ptr::write_bytes(temp.add(to_write as usize), 0, (cluster_size - to_write) as usize);
        }
        let sector = fat_cluster_to_sector(clusters[i as usize]);
        if write_sectors(sector, bpb.sectors_per_cluster as u32, temp) != 0 {
            kfree(temp);
            return -1;
        }
        offset += to_write;
        i += 1;
    }
    kfree(temp);
    let mut sect: [u8; 512] = [0; 512];
    if read_sectors(entry_sector, 1, sect.as_mut_ptr()) != 0 {
        return -1;
    }
    let entry = sect.as_mut_ptr().add(entry_offset as usize) as *mut FatDirEnt;
    ptr::write_bytes(core::ptr::addr_of_mut!((*entry).name) as *mut u8, b' ', 11);
    fat_create_8dot3_name(fname.as_mut_ptr(), core::ptr::addr_of_mut!((*entry).name) as *mut u8);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).attr), FAT_ATTR_ARCHIVE);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).nt_reserved), 0);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).tenths), 0);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).time_created), 0);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).date_created), 0);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).date_accessed), 0);
    ptr::write_unaligned(
        core::ptr::addr_of_mut!((*entry).cluster_high),
        ((clusters[0] >> 16) & 0xFFFF) as u16,
    );
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).time_modified), 0);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).date_modified), 0);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).cluster_low), (clusters[0] & 0xFFFF) as u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).size), size);
    if write_sectors(entry_sector, 1, sect.as_mut_ptr()) != 0 {
        return -1;
    }
    0
}

pub unsafe fn list_dir(path: *const u8, entries: *mut LumieDirEnt, max_entries: i32) -> i32 {
    if !(*fat()).initialized {
        return -1;
    }
    let mut ent: FatDirEnt = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    if fat_find_cluster(path, &mut ent as *mut FatDirEnt) == 0 {
        if path.is_null()
            || ptr::read(path) == 0
            || (ptr::read(path) == b'/' && ptr::read(path.add(1)) == 0)
        {
            return list_dir(
                b"/\0" as *const u8,
                entries,
                max_entries,
            );
        }
        return -1;
    }
    let attr = ptr::read_unaligned(core::ptr::addr_of!(ent.attr));
    if (attr & FAT_ATTR_DIRECTORY) == 0 {
        return -1;
    }
    let ecl = ptr::read_unaligned(core::ptr::addr_of!(ent.cluster_low)) as u32;
    let ech = ptr::read_unaligned(core::ptr::addr_of!(ent.cluster_high)) as u32;
    let mut cluster = ecl | (ech << 16);
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    if cluster == 0 {
        cluster = bpb.root_cluster;
    }
    let mut count: i32 = 0;
    let cluster_size = (bpb.sectors_per_cluster as u32) * bpb.bytes_per_sector as u32;
    let sector = kmalloc(cluster_size as usize);
    if sector.is_null() {
        return -1;
    }
    while cluster >= 2 && cluster < FAT_END_OF_CHAIN && count < max_entries {
        if fat_read_cluster(cluster, sector) != 0 {
            break;
        }
        let entries_per_cluster = cluster_size / 32;
        let dent = sector as *const FatDirEnt;
        let mut i: u32 = 0;
        while i < entries_per_cluster && count < max_entries {
            let name0 = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).name));
            if name0[0] == 0 {
                break;
            }
            if name0[0] == 0xE5 {
                i += 1;
                continue;
            }
            let attr = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).attr));
            if (attr & FAT_ATTR_LFN) == FAT_ATTR_LFN {
                i += 1;
                continue;
            }
            dir_name_to_str(
                core::ptr::addr_of!((*dent.add(i as usize)).name) as *const u8,
                (*entries.add(count as usize)).name.as_mut_ptr(),
            );
            ptr::write(
                &mut (*entries.add(count as usize)).is_dir,
                if (attr & FAT_ATTR_DIRECTORY) != 0 { 1 } else { 0 },
            );
            let sz = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).size));
            ptr::write(&mut (*entries.add(count as usize)).size, sz);
            count += 1;
            i += 1;
        }
        cluster = fat_get_next_cluster(cluster);
    }
    kfree(sector);
    count
}

pub unsafe fn exists(path: *const u8) -> bool {
    if !(*fat()).initialized {
        return false;
    }
    let mut ent: FatDirEnt = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    fat_find_cluster(path, &mut ent as *mut FatDirEnt) != 0
}

pub unsafe fn get_file_size(path: *const u8) -> i32 {
    if !(*fat()).initialized {
        return -1;
    }
    let mut ent: FatDirEnt = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    if fat_find_cluster(path, &mut ent as *mut FatDirEnt) == 0 {
        return -1;
    }
    let attr = ptr::read_unaligned(core::ptr::addr_of!(ent.attr));
    if (attr & FAT_ATTR_DIRECTORY) != 0 {
        return -1;
    }
    let ent_size_ptr = core::ptr::addr_of!(ent.size);
    ptr::read_unaligned(ent_size_ptr) as i32
}

pub unsafe fn delete(path: *const u8) -> i32 {
    if !(*fat()).initialized {
        return -1;
    }
    let path_len = strlen(path);
    let mut last_slash: i32 = -1;
    let mut i: usize = 0;
    while i < path_len {
        if ptr::read(path.add(i)) == b'/' {
            last_slash = i as i32;
        }
        i += 1;
    }
    let mut dir_path: [u8; 256] = [0; 256];
    let mut fname: [u8; 256] = [0; 256];
    if last_slash < 0 {
        let src = b"/\0";
        ptr::copy_nonoverlapping(src.as_ptr(), dir_path.as_mut_ptr(), 2);
        let mut fnlen = path_len;
        if fnlen >= 256 {
            fnlen = 255;
        }
        ptr::copy_nonoverlapping(path, fname.as_mut_ptr(), fnlen);
        ptr::write(fname.as_mut_ptr().add(fnlen), 0);
    } else {
        let len = last_slash as usize;
        let actual_len = if len >= 256 { 255 } else { len };
        ptr::copy_nonoverlapping(path, dir_path.as_mut_ptr(), actual_len);
        ptr::write(dir_path.as_mut_ptr().add(actual_len), 0);
        if actual_len == 0 {
            ptr::write(dir_path.as_mut_ptr(), b'/');
            ptr::write(dir_path.as_mut_ptr().add(1), 0);
        }
        let mut fnlen = path_len - (last_slash as usize) - 1;
        if fnlen >= 256 {
            fnlen = 255;
        }
        ptr::copy_nonoverlapping(
            path.add(last_slash as usize + 1),
            fname.as_mut_ptr(),
            fnlen,
        );
        ptr::write(fname.as_mut_ptr().add(fnlen), 0);
    }
    let mut parent_ent: FatDirEnt =
        ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    if fat_find_cluster(dir_path.as_mut_ptr(), &mut parent_ent as *mut FatDirEnt) == 0 {
        return -1;
    }
        let pa = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.attr));
        if (pa & FAT_ATTR_DIRECTORY) == 0 {
            return -1;
        }
        let pcl = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.cluster_low)) as u32;
        let pch = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.cluster_high)) as u32;
    let mut parent_cluster = pcl | (pch << 16);
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    if parent_cluster == 0 {
        parent_cluster = bpb.root_cluster;
    }
    let mut entry_sector: u32 = 0;
    let mut entry_offset: u32 = 0;
    let mut entry_buf: FatDirEnt =
        ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    let slot_type = fat_find_dir_slot(
        parent_cluster,
        fname.as_mut_ptr(),
        &mut entry_buf as *mut FatDirEnt,
        &mut entry_sector as *mut u32,
        &mut entry_offset as *mut u32,
    );
    if slot_type != 2 {
        return -1;
    }
    let eb_attr = ptr::read_unaligned(core::ptr::addr_of!(entry_buf.attr));
    if (eb_attr & FAT_ATTR_DIRECTORY) != 0 {
        let ecl = ptr::read_unaligned(core::ptr::addr_of!(entry_buf.cluster_low)) as u32;
        let ech = ptr::read_unaligned(core::ptr::addr_of!(entry_buf.cluster_high)) as u32;
        let mut cluster = ecl | (ech << 16);
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let mut _parent_cluster = parent_cluster;
    if _parent_cluster == 0 {
        _parent_cluster = bpb.root_cluster;
    }
    let cluster_size = (bpb.sectors_per_cluster as u32) * bpb.bytes_per_sector as u32;
        let sector = kmalloc(cluster_size as usize);
        if sector.is_null() {
            return -1;
        }
        let mut empty: i32 = 1;
        while cluster >= 2 && cluster < FAT_END_OF_CHAIN {
            if fat_read_cluster(cluster, sector) != 0 {
                break;
            }
            let entries_per_cluster = cluster_size / 32;
            let dent = sector as *const FatDirEnt;
            let mut i: u32 = 0;
            while i < entries_per_cluster {
                let name0 = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).name));
                if name0[0] == 0 {
                    break;
                }
                if name0[0] == 0xE5 {
                    i += 1;
                    continue;
                }
                let a = ptr::read_unaligned(core::ptr::addr_of!((*dent.add(i as usize)).attr));
                if (a & FAT_ATTR_LFN) == FAT_ATTR_LFN {
                    i += 1;
                    continue;
                }
                if name0[0] == b'.' && name0[1] == b' ' {
                    i += 1;
                    continue;
                }
                if name0[0] == b'.' && name0[1] == b'.' && name0[2] == b' ' {
                    i += 1;
                    continue;
                }
                empty = 0;
                break;
            }
            if empty == 0 {
                break;
            }
            cluster = fat_get_next_cluster(cluster);
        }
        kfree(sector);
        if empty == 0 {
            return -2;
        }
    }
        let ecl = ptr::read_unaligned(core::ptr::addr_of!(entry_buf.cluster_low)) as u32;
        let ech = ptr::read_unaligned(core::ptr::addr_of!(entry_buf.cluster_high)) as u32;
    let mut cluster = ecl | (ech << 16);
    if cluster >= 2 {
        while cluster >= 2 && cluster < FAT_END_OF_CHAIN {
            let next = fat_read_fat_entry(cluster);
            fat_write_fat_entry(cluster, 0);
            cluster = next;
        }
    }
    let mut sect: [u8; 512] = [0; 512];
    if read_sectors(entry_sector, 1, sect.as_mut_ptr()) != 0 {
        return -1;
    }
    let entry = sect.as_mut_ptr().add(entry_offset as usize) as *mut FatDirEnt;
    ptr::write(core::ptr::addr_of_mut!((*entry).name) as *mut u8, 0xE5);
    if write_sectors(entry_sector, 1, sect.as_mut_ptr()) != 0 {
        return -1;
    }
    0
}

pub unsafe fn mkdir(path: *const u8) -> i32 {
    if !(*fat()).initialized {
        return -1;
    }
    let path_len = strlen(path);
    let mut last_slash: i32 = -1;
    let mut i: usize = 0;
    while i < path_len {
        if ptr::read(path.add(i)) == b'/' {
            last_slash = i as i32;
        }
        i += 1;
    }
    let mut dir_path: [u8; 256] = [0; 256];
    let mut fname: [u8; 256] = [0; 256];
    if last_slash < 0 {
        let src = b"/\0";
        ptr::copy_nonoverlapping(src.as_ptr(), dir_path.as_mut_ptr(), 2);
        let mut fnlen = path_len;
        if fnlen >= 256 {
            fnlen = 255;
        }
        ptr::copy_nonoverlapping(path, fname.as_mut_ptr(), fnlen);
        ptr::write(fname.as_mut_ptr().add(fnlen), 0);
    } else {
        let len = last_slash as usize;
        let actual_len = if len >= 256 { 255 } else { len };
        ptr::copy_nonoverlapping(path, dir_path.as_mut_ptr(), actual_len);
        ptr::write(dir_path.as_mut_ptr().add(actual_len), 0);
        if actual_len == 0 {
            ptr::write(dir_path.as_mut_ptr(), b'/');
            ptr::write(dir_path.as_mut_ptr().add(1), 0);
        }
        let mut fnlen = path_len - (last_slash as usize) - 1;
        if fnlen >= 256 {
            fnlen = 255;
        }
        ptr::copy_nonoverlapping(
            path.add(last_slash as usize + 1),
            fname.as_mut_ptr(),
            fnlen,
        );
        ptr::write(fname.as_mut_ptr().add(fnlen), 0);
    }
    let mut parent_ent: FatDirEnt =
        ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    let parent_cluster: u32;
    if fat_find_cluster(dir_path.as_mut_ptr(), &mut parent_ent as *mut FatDirEnt) != 0 {
        let pa = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.attr));
        if (pa & FAT_ATTR_DIRECTORY) == 0 {
            return -1;
        }
        let pcl = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.cluster_low)) as u32;
        let pch = ptr::read_unaligned(core::ptr::addr_of!(parent_ent.cluster_high)) as u32;
        parent_cluster = pcl | (pch << 16);
    } else {
        return -1;
    }
    let bpb: FatBpb = ptr::read_unaligned(&(*fat()).bpb as *const FatBpb);
    let mut parent_cluster = parent_cluster;
    if parent_cluster == 0 {
        parent_cluster = bpb.root_cluster;
    }
    let cluster_size = (bpb.sectors_per_cluster as u32) * bpb.bytes_per_sector as u32;
    let fat_size = (*fat()).fat_size;
    let fat_entries = fat_size / 4;
    let mut new_cluster: u32 = 0;
    let mut i: u32 = 2;
    while i < fat_entries {
        if fat_read_fat_entry(i) == 0 {
            new_cluster = i;
            break;
        }
        i += 1;
    }
    if new_cluster == 0 {
        return -1;
    }
    if fat_write_fat_entry(new_cluster, 0x0FFFFFFF) != 0 {
        return -1;
    }
    let zero_buf = kmalloc(cluster_size as usize);
    if zero_buf.is_null() {
        return -1;
    }
    ptr::write_bytes(zero_buf, 0, cluster_size as usize);
    let dot = zero_buf as *mut FatDirEnt;
    ptr::write_bytes(core::ptr::addr_of_mut!((*dot).name) as *mut u8, b' ', 11);
    ptr::write(core::ptr::addr_of_mut!((*dot).name) as *mut u8, b'.');
    ptr::write_unaligned(core::ptr::addr_of_mut!((*dot).attr), FAT_ATTR_DIRECTORY);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*dot).cluster_low), (new_cluster & 0xFFFF) as u16);
    ptr::write_unaligned(
        core::ptr::addr_of_mut!((*dot).cluster_high),
        ((new_cluster >> 16) & 0xFFFF) as u16,
    );
    let dotdot = zero_buf.add(32) as *mut FatDirEnt;
    ptr::write_bytes(core::ptr::addr_of_mut!((*dotdot).name) as *mut u8, b' ', 11);
    ptr::write(core::ptr::addr_of_mut!((*dotdot).name) as *mut u8, b'.');
    ptr::write((core::ptr::addr_of_mut!((*dotdot).name) as *mut u8).add(1), b'.');
    ptr::write_unaligned(core::ptr::addr_of_mut!((*dotdot).attr), FAT_ATTR_DIRECTORY);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*dotdot).cluster_low), (parent_cluster & 0xFFFF) as u16);
    ptr::write_unaligned(
        core::ptr::addr_of_mut!((*dotdot).cluster_high),
        ((parent_cluster >> 16) & 0xFFFF) as u16,
    );
    let sector = fat_cluster_to_sector(new_cluster);
    if write_sectors(sector, bpb.sectors_per_cluster as u32, zero_buf) != 0 {
        kfree(zero_buf);
        return -1;
    }
    kfree(zero_buf);
    let mut entry_sector: u32 = 0;
    let mut entry_offset: u32 = 0;
    let mut entry_buf: FatDirEnt =
        ptr::read_unaligned(&(*fat()).bpb as *const FatBpb as *const FatDirEnt);
    let mut slot_type = fat_find_dir_slot(
        parent_cluster,
        fname.as_mut_ptr(),
        &mut entry_buf as *mut FatDirEnt,
        &mut entry_sector as *mut u32,
        &mut entry_offset as *mut u32,
    );
    if slot_type == 0 {
        let ext = fat_extend_directory(parent_cluster);
        if ext == !0u32 {
            return -1;
        }
        slot_type = fat_find_dir_slot(
            parent_cluster,
            fname.as_mut_ptr(),
            &mut entry_buf as *mut FatDirEnt,
            &mut entry_sector as *mut u32,
            &mut entry_offset as *mut u32,
        );
        if slot_type == 0 {
            return -1;
        }
    }
    let mut sect: [u8; 512] = [0; 512];
    if read_sectors(entry_sector, 1, sect.as_mut_ptr()) != 0 {
        return -1;
    }
    let entry = sect.as_mut_ptr().add(entry_offset as usize) as *mut FatDirEnt;
    ptr::write_bytes(core::ptr::addr_of_mut!((*entry).name) as *mut u8, b' ', 11);
    fat_create_8dot3_name(fname.as_mut_ptr(), core::ptr::addr_of_mut!((*entry).name) as *mut u8);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).attr), FAT_ATTR_DIRECTORY);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).cluster_low), (new_cluster & 0xFFFF) as u16);
    ptr::write_unaligned(
        core::ptr::addr_of_mut!((*entry).cluster_high),
        ((new_cluster >> 16) & 0xFFFF) as u16,
    );
    ptr::write_unaligned(core::ptr::addr_of_mut!((*entry).size), 0);
    if write_sectors(entry_sector, 1, sect.as_mut_ptr()) != 0 {
        return -1;
    }
    0
}

pub unsafe fn install_bootloader() -> i32 {
    // Handled by loader; no-op.
    0
}

pub unsafe fn set_device(device_handle: efi_handle) -> i32 {
    let bs = G_BS;
    if bs.is_null() {
        return -1;
    }
    let block_io_guid = EFI_BLOCK_IO_GUID;
    let mut block_io: *mut EfiBlockIoProtocol = ptr::null_mut();
    let status = ((*bs).handle_protocol.unwrap())(
        device_handle,
        &block_io_guid as *const EfiGuid as *mut EfiGuid,
        &mut block_io as *mut *mut EfiBlockIoProtocol as *mut *mut c_void,
    );
    if efi_error(status) || block_io.is_null() {
        return -1;
    }
    (*fat()).disk_io.block_io = Some(block_io);
    if init_bpb() != 0 {
        return -1;
    }
    0
}

pub unsafe fn use_ahci() -> i32 {
    if ahci_drv::is_ready() == 0 {
        return -1;
    }
    if init_bpb() != 0 {
        return -1;
    }
    (*fat()).disk_io.use_ahci = true;
    0
}

pub unsafe fn set_drive(read_cb: Option<FatReadFn>, write_cb: Option<FatWriteFn>) -> i32 {
    (*fat()).disk_io.read_cb = read_cb;
    (*fat()).disk_io.write_cb = write_cb;
    if read_cb.is_none() && write_cb.is_none() {
        return 0;
    }
    reinit()
}

pub unsafe fn set_bs(bs: *mut EfiBootServices, img: efi_handle, st: *mut EfiSystemTable) {
    G_BS = bs;
    G_IMAGE = img;
    G_ST = st;
}
