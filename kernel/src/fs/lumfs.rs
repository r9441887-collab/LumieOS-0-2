pub use lumie_std::fs::lumfs;

pub unsafe fn init() -> i32 {
    lumfs::init()
}

pub unsafe fn read_file(path: *const u8, buffer: *mut u8, max_size: u32) -> i32 {
    lumfs::read_file(path, buffer, max_size)
}

pub unsafe fn write_file(path: *const u8, data: *const u8, size: u32) -> i32 {
    lumfs::write_file(path, data, size)
}

pub unsafe fn list_dir(path: *const u8, entries: *mut super::LumieDirEnt, max_entries: i32) -> i32 {
    lumfs::list_dir(path, entries, max_entries)
}

pub unsafe fn exists(path: *const u8) -> bool {
    lumfs::exists(path)
}

pub unsafe fn get_file_size(path: *const u8) -> i32 {
    lumfs::get_file_size(path)
}

pub unsafe fn delete(path: *const u8) -> i32 {
    lumfs::delete(path)
}

pub unsafe fn mkdir(path: *const u8) -> i32 {
    lumfs::mkdir(path)
}

pub unsafe fn rename(old_path: *const u8, new_path: *const u8) -> i32 {
    lumfs::rename(old_path, new_path)
}

pub unsafe fn copy_file(src_path: *const u8, dst_path: *const u8) -> i32 {
    lumfs::copy_file(src_path, dst_path)
}

pub unsafe fn format_at(start_lba: u64, total_sectors: u64) -> i32 {
    lumfs::format_at(start_lba, total_sectors)
}

pub unsafe fn reinit() -> i32 {
    lumfs::reinit()
}
