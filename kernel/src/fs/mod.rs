
pub mod diskio;
pub mod fat32;
pub mod ntfs;
pub mod lumfs;

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
const FS_LUMFS: u32 = 3;

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
    if lumfs::init() == 0 {
        ACTIVE_FS = FS_LUMFS;
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
    if lumfs::init() == 0 {
        ACTIVE_FS = FS_LUMFS;
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
        FS_LUMFS => lumfs::read_file(path, buffer, max_size),
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
        FS_LUMFS => lumfs::write_file(path, data, size),
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
        FS_LUMFS => lumfs::list_dir(path, entries, max_entries),
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
        FS_LUMFS => lumfs::exists(path),
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
        FS_LUMFS => lumfs::get_file_size(path),
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
        FS_LUMFS => lumfs::delete(path),
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
        FS_LUMFS => lumfs::mkdir(path),
        _ => -1,
    }
}

pub unsafe fn rename(old_path: *const u8, new_path: *const u8) -> i32 {
    match ACTIVE_FS {
        FS_LUMFS => lumfs::rename(old_path, new_path),
        FS_FAT32 => {
            let old = path_to_str(old_path);
            let new = path_to_str(new_path);
            fat32_rename(old, new)
        }
        FS_NTFS => {
            let old = path_to_str(old_path);
            let new = path_to_str(new_path);
            ntfs_rename(old, new)
        }
        _ => -1,
    }
}

pub unsafe fn copy_file(src_path: *const u8, dst_path: *const u8) -> i32 {
    match ACTIVE_FS {
        FS_LUMFS => lumfs::copy_file(src_path, dst_path),
        _ => {
            let src = path_to_str(src_path);
            let dst = path_to_str(dst_path);
            generic_copy(src, dst)
        }
    }
}

unsafe fn generic_copy(src: &str, dst: &str) -> i32 {
    let size = match ACTIVE_FS {
        FS_FAT32 => fat32::get_file_size(src.as_ptr()),
        FS_NTFS => ntfs::get_file_size(src),
        _ => return -1,
    };
    if size <= 0 { return -1; }
    let buf = alloc(size as usize);
    if buf.is_null() { return -1; }
    let nread = match ACTIVE_FS {
        FS_FAT32 => fat32::read_file(src.as_ptr(), buf, size as u32),
        FS_NTFS => ntfs::read_file(src, buf, size as u32),
        _ => { dealloc(buf, size as usize); return -1; }
    };
    if nread <= 0 { dealloc(buf, size as usize); return -1; }
    let ret = match ACTIVE_FS {
        FS_FAT32 => fat32::write_file(dst.as_ptr(), buf, nread as u32),
        FS_NTFS => ntfs::write_file(dst, buf, nread as u32),
        _ => { dealloc(buf, size as usize); return -1; }
    };
    dealloc(buf, size as usize);
    ret
}

unsafe fn fat32_rename(_old: &str, _new: &str) -> i32 {
    -1
}

unsafe fn ntfs_rename(_old: &str, _new: &str) -> i32 {
    -1
}

extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

unsafe fn alloc(size: usize) -> *mut u8 {
    malloc(size)
}

unsafe fn dealloc(ptr: *mut u8, _size: usize) {
    free(ptr)
}

pub unsafe fn format(total_sectors: u64) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::format(total_sectors),
        FS_NTFS => ntfs::format(total_sectors),
        FS_LUMFS => lumfs::format_at(0, total_sectors),
        _ => -1,
    }
}

pub unsafe fn format_at(start_lba: u64, total_sectors: u64) -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::format_at(start_lba, total_sectors),
        FS_LUMFS => lumfs::format_at(start_lba, total_sectors),
        FS_NTFS => -1,
        _ => -1,
    }
}

pub unsafe fn reinit() -> i32 {
    match ACTIVE_FS {
        FS_FAT32 => fat32::reinit(),
        FS_NTFS => ntfs::reinit(),
        FS_LUMFS => lumfs::reinit(),
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

pub fn active_fs_type() -> u32 {
    unsafe { ACTIVE_FS }
}

unsafe fn path_to_str<'a>(path: *const u8) -> &'a str {
    if path.is_null() {
        return "";
    }
    let len = (0..256).find(|&i| *path.add(i) == 0).unwrap_or(0);
    core::str::from_utf8(core::slice::from_raw_parts(path, len)).unwrap_or("")
}
