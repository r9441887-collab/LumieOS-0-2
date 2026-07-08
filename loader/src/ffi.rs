use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;
use crate::{FbInfo, SysBootInfo, SysModule};

use lumie_std::fs::fat32;
use lumie_std::fs::ntfs;
use lumie_std::fs::diskio::{FatReadFn, FatWriteFn, AllocFn, FreeFn};

/* ------------------------------------------------------------------ */
/*  install.pkg format                                                */
/* ------------------------------------------------------------------ */
// [Header: 16 bytes]
//   magic:       u32 = "LPKG" (0x4B47504C LE)
//   version:     u32 = 1
//   file_count:  u32
//   entries_off: u32  (byte offset from file start to first PkgEntry)
// [Entries: file_count * 32 bytes]
//   path_off:     u32 (offset from file start to null-terminated path)
//   data_off:     u32 (offset from file start to raw file data)
//   data_sz:      u32
//   flags:        u8  (bit 0 = directory)
//   reserved:     [u8; 3]
//   reserved2:    [u8; 16]
// [Path strings] (null-terminated, sequential)
// [File data]

type InstallWriteFn = unsafe fn(*const u8, *const c_void, u32) -> i32;
static mut G_PKG_WRITE: Option<InstallWriteFn> = None;

static mut G_BS: *mut EfiBootServices = ptr::null_mut();
static mut G_BLOCK_IO: *mut crate::uefi::EfiBlockIoProtocol = ptr::null_mut();

unsafe fn uefi_read_sectors(lba: u32, count: u32, buffer: *mut u8) -> i32 {
    let block_io = G_BLOCK_IO;
    if block_io.is_null() {
        return -1;
    }
    let sector_size = 512u64;
    let media = (*block_io).media;
    let status = ((*block_io).read_blocks.unwrap())(
        block_io as *mut c_void,
        (*media).media_id,
        lba as u64,
        (count as u64) * sector_size,
        buffer as *mut c_void,
    );
    if status != 0 { -1 } else { 0 }
}

unsafe fn uefi_write_sectors(lba: u32, count: u32, buffer: *const u8) -> i32 {
    let block_io = G_BLOCK_IO;
    if block_io.is_null() {
        return -1;
    }
    let sector_size = 512u64;
    let media = (*block_io).media;
    let status = ((*block_io).write_blocks.unwrap())(
        block_io as *mut c_void,
        (*media).media_id,
        lba as u64,
        (count as u64) * sector_size,
        buffer as *mut c_void,
    );
    if status != 0 { -1 } else { 0 }
}

unsafe fn uefi_alloc(size: usize) -> *mut u8 {
    if G_BS.is_null() {
        return ptr::null_mut();
    }
    let bs: *mut EfiBootServices = G_BS;
    let mut buf: *mut u8 = ptr::null_mut();
    let status = ((*bs).allocate_pool.unwrap())(
        2, // EfiLoaderData
        size as u64,
        &mut buf as *mut *mut u8 as *mut *mut c_void,
    );
    if status != 0 { return ptr::null_mut(); }
    buf
}

unsafe fn uefi_free(ptr: *mut u8, _size: usize) {
    if G_BS.is_null() { return; }
    let bs: *mut EfiBootServices = G_BS;
    if let Some(fp) = (*bs).free_pool {
        fp(ptr as *mut c_void);
    }
}

static mut GOP_PROTO: *mut EfiGopProtocol = ptr::null_mut();
static mut LD_FB: FbInfo = FbInfo {
    base: 0, size: 0, width: 0, height: 0, pitch: 0, bpp: 0, pixel_format: 0,
};

pub unsafe fn gop_init(_image_handle: efi_handle, st: *mut EfiSystemTable) -> u64 {
    let bs = (*st).boot_services;
    let locate_protocol = match (*bs).locate_protocol {
        Some(lp) => lp,
        None => return 1,
    };
    let gop_guid = &EFI_GOP_GUID as *const EfiGuid;
    let mut gop_ptr: *mut c_void = ptr::null_mut();
    let status = locate_protocol(gop_guid, ptr::null_mut(), &mut gop_ptr);
    if status != 0 || gop_ptr.is_null() {
        return status;
    }
    let gop = &mut *(gop_ptr as *mut EfiGopProtocol);
    let mode = &*gop.mode;
    if let Some(sm) = gop.set_mode {
        let s = sm(gop as *mut _ as *mut c_void, mode.mode);
        if (s as i64) < 0 {
            return s;
        }
    }
    let mut info_size: u64 = 0;
    let mut info: *mut EfiGopModeInfo = ptr::null_mut();
    if let Some(qm) = gop.query_mode {
        qm(gop as *mut _ as *mut c_void, mode.mode, &mut info_size, &mut info);
    }
    let fb_info = if !info.is_null() { &*info } else { &*(mode.info) };
    GOP_PROTO = gop_ptr as *mut EfiGopProtocol;
    LD_FB.base = mode.frame_buffer_base;
    LD_FB.size = mode.frame_buffer_size;
    LD_FB.width = fb_info.horizontal_resolution;
    LD_FB.height = fb_info.vertical_resolution;
    LD_FB.pitch = fb_info.pixels_per_scan_line * 4;
    LD_FB.bpp = 32;
    LD_FB.pixel_format = fb_info.pixel_format;
    0
}
pub unsafe fn gop_get_width() -> u32 {
    LD_FB.width
}
pub unsafe fn gop_get_height() -> u32 {
    LD_FB.height
}
pub unsafe fn gop_fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    if LD_FB.base == 0 { return; }
    let pitch_px = LD_FB.pitch / 4;
    let base = LD_FB.base as *mut u32;
    let x = if x >= LD_FB.width { LD_FB.width - 1 } else { x };
    let y = if y >= LD_FB.height { LD_FB.height - 1 } else { y };
    let w = if w > LD_FB.width - x { LD_FB.width - x } else { w };
    let h = if h > LD_FB.height - y { LD_FB.height - y } else { h };
    for j in 0..h {
        let line = base.offset(((y + j) as u64 * pitch_px as u64 + x as u64) as isize);
        for i in 0..w {
            ptr::write_volatile(line.offset(i as isize), color);
        }
    }
}
pub unsafe fn gop_draw_string(x: u32, y: u32, fg: u32, bg: u32, mut s: *const u8) {
    if LD_FB.base == 0 || s.is_null() { return; }
    let pitch_px = LD_FB.pitch / 4;
    let base = LD_FB.base as *mut u32;
    let mut cx = x;
    loop {
        let c = *s;
        if c == 0 { break; }
        s = s.add(1);
        if c < 32 || c > 126 { cx += 8; continue; }
        if cx + 8 > LD_FB.width || y + 16 > LD_FB.height { break; }
        let idx = (c - 32) as usize;
        for row in 0..16 {
            let bits = crate::font::FONT_8X16[idx][row];
            let line = base.offset(((y as u64 + row as u64) * pitch_px as u64 + cx as u64) as isize);
            if bits == 0xFF {
                for col in 0..8 {
                    ptr::write_volatile(line.offset(col as isize), fg);
                }
            } else if bits == 0x00 {
                for col in 0..8 {
                    ptr::write_volatile(line.offset(col as isize), bg);
                }
            } else {
                for col in 0..8 {
                    let color = if bits & (0x80 >> col) != 0 { fg } else { bg };
                    ptr::write_volatile(line.offset(col as isize), color);
                }
            }
        }
        cx += 8;
    }
}
pub unsafe fn gop_put_pixel(x: u32, y: u32, color: u32) {
    if LD_FB.base == 0 { return; }
    if x >= LD_FB.width || y >= LD_FB.height { return; }
    let pitch_px = LD_FB.pitch / 4;
    let base = LD_FB.base as *mut u32;
    ptr::write_volatile(base.offset((y as u64 * pitch_px as u64 + x as u64) as isize), color);
}
pub unsafe fn gop_get_pixel(x: u32, y: u32) -> u32 {
    if LD_FB.base == 0 { return 0; }
    if x >= LD_FB.width || y >= LD_FB.height { return 0; }
    let pitch_px = LD_FB.pitch / 4;
    let base = LD_FB.base as *mut u32;
    ptr::read_volatile(base.offset((y as u64 * pitch_px as u64 + x as u64) as isize))
}
pub unsafe fn gop_get_fb() -> *mut FbInfo {
    &raw mut LD_FB
}
pub unsafe fn gop_nv_init() -> i32 {
    0
}
pub unsafe fn term_init() {}
pub unsafe fn term_clear(_bg: u32) {}
pub unsafe fn term_write(_s: *const u8) {}
pub unsafe fn term_writeln(_s: *const u8) {}
pub unsafe fn term_set_fg(_c: u32) {}
pub unsafe fn term_set_bg(_c: u32) {}
pub unsafe fn kbd_init(_st: *mut EfiSystemTable) {}
pub unsafe fn kbd_switch_to_ps2() {}
pub unsafe fn mouse_init(_st: *mut EfiSystemTable) {}
pub unsafe fn mouse_reinit_ps2() {}
pub unsafe fn mouse_cleanup_uefi() {}
pub unsafe fn fat_init() -> i32 {
    fat32::reinit()
}
pub unsafe fn fat_set_bs(bs: *mut EfiBootServices, _img: efi_handle, _st: *mut EfiSystemTable) {
    G_BS = bs;
    fat32::set_alloc(Some(uefi_alloc as AllocFn), Some(uefi_free as FreeFn));
}
pub unsafe fn fat_set_device(handle: efi_handle) -> i32 {
    let bs = G_BS;
    if bs.is_null() || handle.is_null() {
        return -1;
    }
    let block_io_guid = crate::uefi::EFI_BLOCK_IO_GUID;
    let mut block_io: *mut crate::uefi::EfiBlockIoProtocol = ptr::null_mut();
    let status = ((*bs).handle_protocol.unwrap())(
        handle,
        &block_io_guid as *const crate::uefi::EfiGuid as *mut crate::uefi::EfiGuid,
        &mut block_io as *mut *mut crate::uefi::EfiBlockIoProtocol as *mut *mut c_void,
    );
    if status != 0 || block_io.is_null() {
        return -1;
    }
    G_BLOCK_IO = block_io;
    let read_fn: FatReadFn = core::mem::transmute(uefi_read_sectors as *const () as usize);
    let write_fn: FatWriteFn = core::mem::transmute(uefi_write_sectors as *const () as usize);
    fat32::set_drive(Some(read_fn), Some(write_fn));
    fat32::reinit()
}
pub unsafe fn fat_exists(path: *const u8) -> i32 {
    if fat32::exists(path) { 1 } else { 0 }
}
pub unsafe fn fat_read_file(path: *const u8, buf: *mut c_void, max: u32) -> i32 {
    fat32::read_file(path, buf as *mut u8, max)
}
pub unsafe fn fat_write_file(path: *const u8, data: *const c_void, size: u32) -> i32 {
    fat32::write_file(path, data as *const u8, size)
}
pub unsafe fn fat_format(total_sectors: u64) -> i32 {
    fat32::format_at(0, total_sectors)
}
pub unsafe fn fat_format_at(start_lba: u64, total_sectors: u64) -> i32 {
    fat32::format_at(start_lba, total_sectors)
}
pub unsafe fn fat_delete(path: *const u8) -> i32 {
    fat32::delete(path)
}
pub unsafe fn fat_mkdir(path: *const u8) -> i32 {
    fat32::mkdir(path)
}
pub unsafe fn fat_install_bootloader() -> i32 {
    0
}
pub unsafe fn fat_get_file_size(path: *const u8) -> i32 {
    fat32::get_file_size(path)
}
pub unsafe fn fat_use_ahci() -> i32 {
    0
}
pub unsafe fn fat_set_drive(r: usize, w: usize, _priv_: *mut c_void) -> i32 {
    let read_fn = if r != 0 {
        Some(core::mem::transmute::<usize, FatReadFn>(r))
    } else {
        None
    };
    let write_fn = if w != 0 {
        Some(core::mem::transmute::<usize, FatWriteFn>(w))
    } else {
        None
    };
    fat32::set_drive(read_fn, write_fn)
}
pub unsafe fn fat_reinit() -> i32 {
    fat32::reinit()
}

/* ---- NTFS ---- */
pub unsafe fn ntfs_init() -> i32 {
    ntfs::init()
}
pub unsafe fn ntfs_set_bs(bs: *mut EfiBootServices, _img: efi_handle, _st: *mut EfiSystemTable) {
    G_BS = bs;
    ntfs::set_alloc(Some(uefi_alloc as AllocFn), Some(uefi_free as FreeFn));
}
pub unsafe fn ntfs_set_device(handle: efi_handle) -> i32 {
    let bs = G_BS;
    if bs.is_null() || handle.is_null() {
        return -1;
    }
    let block_io_guid = crate::uefi::EFI_BLOCK_IO_GUID;
    let mut block_io: *mut crate::uefi::EfiBlockIoProtocol = ptr::null_mut();
    let status = ((*bs).handle_protocol.unwrap())(
        handle,
        &block_io_guid as *const crate::uefi::EfiGuid as *mut crate::uefi::EfiGuid,
        &mut block_io as *mut *mut crate::uefi::EfiBlockIoProtocol as *mut *mut c_void,
    );
    if status != 0 || block_io.is_null() {
        return -1;
    }
    G_BLOCK_IO = block_io;
    let read_fn: FatReadFn = core::mem::transmute(uefi_read_sectors as *const () as usize);
    let write_fn: FatWriteFn = core::mem::transmute(uefi_write_sectors as *const () as usize);
    ntfs::set_drive(Some(read_fn), Some(write_fn));
    ntfs::init()
}
pub unsafe fn ntfs_format(_total_sectors: u64) -> i32 {
    -1
}
pub unsafe fn ntfs_format_at(_start_lba: u64, _total_sectors: u64) -> i32 {
    -1
}
unsafe fn cstr_to_str(s: *const u8) -> &'static str {
    if s.is_null() {
        return "";
    }
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    core::str::from_utf8_unchecked(core::slice::from_raw_parts(s, len))
}
pub unsafe fn ntfs_exists(path: *const u8) -> i32 {
    if ntfs::exists(cstr_to_str(path)) { 1 } else { 0 }
}
pub unsafe fn ntfs_read_file(path: *const u8, buf: *mut c_void, max: u32) -> i32 {
    ntfs::read_file(cstr_to_str(path), buf as *mut u8, max)
}
pub unsafe fn ntfs_write_file(path: *const u8, data: *const c_void, size: u32) -> i32 {
    ntfs::write_file(cstr_to_str(path), data as *const u8, size)
}
pub unsafe fn ntfs_delete(path: *const u8) -> i32 {
    ntfs::delete(cstr_to_str(path))
}
pub unsafe fn ntfs_mkdir(path: *const u8) -> i32 {
    ntfs::mkdir(cstr_to_str(path))
}
pub unsafe fn ntfs_get_file_size(path: *const u8) -> i32 {
    ntfs::get_file_size(cstr_to_str(path))
}
pub unsafe fn ntfs_set_drive(r: usize, w: usize, _priv_: *mut c_void) -> i32 {
    let read_fn = if r != 0 {
        Some(core::mem::transmute::<usize, FatReadFn>(r))
    } else {
        None
    };
    let write_fn = if w != 0 {
        Some(core::mem::transmute::<usize, FatWriteFn>(w))
    } else {
        None
    };
    ntfs::set_drive(read_fn, write_fn)
}
pub unsafe fn mm_init(_bs: *mut EfiBootServices, _img: efi_handle) {}
pub unsafe fn ahci_init() {}
pub unsafe fn ahci_is_ready() -> i32 {
    0
}
pub unsafe fn pit_init(_freq: u32) {}
pub unsafe fn pit_stall(_us: u32) {}
pub unsafe fn pit_get_ticks() -> u64 {
    0
}
pub unsafe fn pcspkr_init() {}
pub unsafe fn shell_run() {}
pub unsafe fn exit_boot_services() {}
pub unsafe fn lumie_efi_register_boot_entry() -> i32 {
    0
}
pub unsafe fn lumie_load_shell_module() -> i32 {
    0
}
pub unsafe fn lumie_cache_kernel_image(_base: *const c_void, _size: u32) {}
pub unsafe fn lumie_sched_init() {}
pub unsafe fn ramdisk_init() {}
pub unsafe fn ramdisk_format_fat32() {}
pub unsafe fn ramdisk_read_sector_cb(_lba: u32, _count: u32, _buf: *mut c_void) -> i32 {
    0
}
pub unsafe fn ramdisk_write_sector_cb(_lba: u32, _count: u32, _buf: *mut c_void) -> i32 {
    0
}
pub unsafe fn install_pkg_set_write_fn(f: Option<InstallWriteFn>) {
    G_PKG_WRITE = f;
}

pub unsafe fn install_pkg_open(path: *const u8, pkg: *mut c_void) -> i32 {
    let sz = fat_get_file_size(path);
    if sz <= 0 { return -1; }

    let data = uefi_alloc(sz as usize);
    if data.is_null() { return -1; }

    let r = fat_read_file(path, data as *mut c_void, sz as u32);
    if r != sz {
        uefi_free(data, sz as usize);
        return -1;
    }

    let magic = *(data as *const u32);
    if magic != 0x4B47504C {
        uefi_free(data, sz as usize);
        return -1;
    }

    let file_count = *(data.add(8) as *const u32);
    let entries_off = *(data.add(12) as *const u32);

    let h = pkg as *mut u8;
    *(h as *mut *mut u8) = data;
    *(h.add(8) as *mut u32) = sz as u32;
    *(h.add(12) as *mut u32) = file_count;
    *(h.add(16) as *mut u32) = entries_off;
    0
}

pub unsafe fn install_pkg_extract_all(pkg: *mut c_void, _progress: *mut c_void) -> i32 {
    let h = pkg as *mut u8;
    let data = *(h as *mut *mut u8);
    let entry_count = *(h.add(12) as *mut u32);
    let entries_off = *(h.add(16) as *mut u32);

    let write_fn = G_PKG_WRITE.unwrap_or(fat_write_file as InstallWriteFn);

    for i in 0..entry_count {
        let entry = data.add(entries_off as usize + (i * 32) as usize);
        let path_off = *(entry as *const u32);
        let data_off = *(entry.add(4) as *const u32);
        let data_sz = *(entry.add(8) as *const u32);
        let flags = *(entry.add(12) as *const u8);

        let path = data.add(path_off as usize);
        let file_data = data.add(data_off as usize);

        if flags & 1 != 0 {
            if fat_exists(path) == 0 { fat_mkdir(path); }
        } else {
            write_fn(path, file_data as *const c_void, data_sz);
        }
    }
    0
}

pub unsafe fn install_pkg_close(pkg: *mut c_void) {
    let h = pkg as *mut u8;
    let data = *(h as *mut *mut u8);
    if !data.is_null() {
        uefi_free(data, *(h.add(8) as *mut u32) as usize);
    }
    *(h as *mut *mut u8) = ptr::null_mut();
}
pub unsafe fn ps2mouse_init() {}
pub unsafe fn ps2mouse_is_ready() -> i32 {
    0
}
pub unsafe fn ps2mouse_poll(_dx: *mut i32, _dy: *mut i32, _btns: *mut u8) -> i32 {
    0
}
pub unsafe fn xhci_init() {}
pub unsafe fn xhci_mouse_present() -> i32 {
    0
}
pub unsafe fn xhci_poll_mouse(_dx: *mut i32, _dy: *mut i32, _btns: *mut u8) -> i32 {
    0
}
pub unsafe fn drvcheck_run_scan() {}
pub unsafe fn bootcache_save(_key: *const u8) {}
pub unsafe fn sys_load(_path: *const u8, _boot_info: *mut SysBootInfo, _mod_out: *mut SysModule) -> i32 {
    0
}
pub unsafe fn desktop_init() {}
pub unsafe fn desktop_run() {}
pub unsafe fn sched_init() {}
pub unsafe fn disk_get_info(_index: i32) -> *const c_void {
    core::ptr::null()
}
pub unsafe fn lumie_reboot() {}
#[allow(non_upper_case_globals)]
pub static mut g_nv_gpu_api: *mut c_void = core::ptr::null_mut();
