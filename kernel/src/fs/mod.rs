
pub mod diskio;
pub mod fat32;
pub mod ntfs;

pub use lumie_std::fs::bpb;
pub use lumie_std::fs::types;

pub use bpb::{FatBpb, FatDirEnt, FAT_ATTR_ARCHIVE, FAT_ATTR_DIRECTORY, FAT_ATTR_HIDDEN,
    FAT_ATTR_LFN, FAT_ATTR_READ_ONLY, FAT_ATTR_SYSTEM, FAT_ATTR_VOLUME_ID, FAT_END_OF_CHAIN};
pub use diskio::{DiskIo, FatReadFn, FatWriteFn};
pub use types::LumieDirEnt;

use core::ffi::c_void;

const FS_NONE: u32 = 0;
const FS_FAT32: u32 = 1;
const FS_NTFS: u32 = 2;

static mut ACTIVE_FS: u32 = FS_NONE;

pub unsafe fn detect_and_init() -> i32 {
    if fat32::init() == 0 {
        ACTIVE_FS = FS_FAT32;
        return 0;
    }
    if ntfs::init() == 0 {
        ACTIVE_FS = FS_NTFS;
        return 0;
    }
    ACTIVE_FS = FS_NONE;
    -1
}

pub unsafe fn set_device(device_handle: *mut c_void) -> i32 {
    if fat32::set_device(device_handle as _) == 0 {
        ACTIVE_FS = FS_FAT32;
        return 0;
    }
    if ntfs::set_device(device_handle as _) == 0 {
        ACTIVE_FS = FS_NTFS;
        return 0;
    }
    -1
}

pub unsafe fn read_file(path: *const u8, buffer: *mut u8, max_size: u32) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::read_file(path, buffer, max_size),
        FS_NTFS => {
            let s = path_to_str(path);
            ntfs::read_file(s, buffer, max_size)
        }
        _ => -1,
    }
}

pub unsafe fn write_file(path: *const u8, data: *const u8, size: u32) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::write_file(path, data, size),
        FS_NTFS => {
            let s = path_to_str(path);
            ntfs::write_file(s, data, size)
        }
        _ => -1,
    }
}

pub unsafe fn list_dir(path: *const u8, entries: *mut LumieDirEnt, max_entries: i32) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::list_dir(path, entries, max_entries),
        FS_NTFS => {
            let s = path_to_str(path);
            let entries_ref = core::slice::from_raw_parts_mut(entries, max_entries as usize);
            ntfs::list_dir(s, entries_ref, max_entries)
        }
        _ => -1,
    }
}

pub unsafe fn exists(path: *const u8) -> bool {
    match ACTIVE_FS {
        FS_FAT32 => fat32::exists(path),
        FS_NTFS => {
            let s = path_to_str(path);
            ntfs::exists(s)
        }
        _ => false,
    }
}

pub unsafe fn get_file_size(path: *const u8) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::get_file_size(path),
        FS_NTFS => {
            let s = path_to_str(path);
            ntfs::get_file_size(s)
        }
        _ => -1,
    }
}

pub unsafe fn delete(path: *const u8) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::delete(path),
        FS_NTFS => {
            let s = path_to_str(path);
            ntfs::delete(s)
        }
        _ => -1,
    }
}

pub unsafe fn mkdir(path: *const u8) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::mkdir(path),
        FS_NTFS => {
            let s = path_to_str(path);
            ntfs::mkdir(s)
        }
        _ => -1,
    }
}

pub unsafe fn format(total_sectors: u64) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::format(total_sectors),
        FS_NTFS => ntfs::format(total_sectors),
        _ => -1,
    }
}

pub unsafe fn format_at(start_lba: u64, total_sectors: u64) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::format_at(start_lba, total_sectors),
        FS_NTFS => -1,
        _ => -1,
    }
}

pub unsafe fn reinit() -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::reinit(),
        FS_NTFS => ntfs::reinit(),
        _ => -1,
    }
}

pub unsafe fn use_ahci() -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::use_ahci(),
        FS_NTFS => ntfs::use_ahci(),
        _ => -1,
    }
}

unsafe fn path_to_str<'a>(path: *const u8) -> &'a str {
    if path.is_null() {
        return "";
    }
    let len = (0..256).find(|&i| *path.add(i) == 0).unwrap_or(0);
    core::str::from_utf8(core::slice::from_raw_parts(path, len)).unwrap_or("")
}
