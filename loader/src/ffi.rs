use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;
use crate::{FbInfo, SysBootInfo, SysModule};

use lumie_std::fs::fat32;
use lumie_std::fs::ntfs;
use lumie_std::fs::lumfs;
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
static mut G_PARTITION_OFFSET: u64 = 0;
static mut G_IMAGE_HANDLE: efi_handle = core::ptr::null_mut();

pub unsafe fn lumie_set_image_handle(h: efi_handle) {
    G_IMAGE_HANDLE = h;
}

unsafe fn uefi_read_sectors(lba: u32, count: u32, buffer: *mut u8) -> i32 {
    let block_io = G_BLOCK_IO;
    if block_io.is_null() {
        return -1;
    }
    let media = (*block_io).media;
    let sector_size = (*media).block_size as u64;
    let status = ((*block_io).read_blocks.unwrap())(
        block_io as *mut c_void,
        (*media).media_id,
        (lba as u64) + G_PARTITION_OFFSET,
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
    let media = (*block_io).media;
    let sector_size = (*media).block_size as u64;
    let status = ((*block_io).write_blocks.unwrap())(
        block_io as *mut c_void,
        (*media).media_id,
        (lba as u64) + G_PARTITION_OFFSET,
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

/* ---- Terminal on framebuffer ---- */
const TAB_WIDTH: i32 = 4;

static mut TERM_X: i32 = 0;
static mut TERM_Y: i32 = 0;
static mut TERM_COLS: i32 = 0;
static mut TERM_ROWS: i32 = 0;
static mut TERM_FG: u32 = 0xFFFFFF;
static mut TERM_BG: u32 = 0;

pub unsafe fn term_init() {
    TERM_COLS = (gop_get_width() / 8) as i32;
    TERM_ROWS = (gop_get_height() / 16) as i32;
    TERM_X = 0;
    TERM_Y = 0;
    TERM_FG = 0xFFFFFF;
    TERM_BG = 0;
}

pub unsafe fn term_clear(bg: u32) {
    TERM_BG = bg;
    TERM_X = 0;
    TERM_Y = 0;
    gop_fill_rect(0, 0, gop_get_width(), gop_get_height(), bg);
}

pub unsafe fn term_newline() {
    TERM_X = 0;
    TERM_Y += 1;
    if TERM_Y >= TERM_ROWS {
        TERM_Y = TERM_ROWS - 1;
        /* Scroll: copy each row up */
        let w = gop_get_width();
        let row_h = 16u32;
        let pitch = LD_FB.pitch;
        let base = LD_FB.base as *mut u8;
        if base as u64 != 0 && pitch > 0 {
            for y in 0..(TERM_ROWS - 1) as u32 * row_h {
                let src_off = ((y + row_h) as u64) * pitch as u64;
                let dst_off = (y as u64) * pitch as u64;
                core::ptr::copy(
                    base.add(src_off as usize),
                    base.add(dst_off as usize),
                    w as usize * 4,
                );
            }
            /* Clear last row */
            let last_row_off = ((TERM_ROWS - 1) as u32 * row_h) as u64 * pitch as u64;
            for y in 0..row_h {
                let off = last_row_off + (y as u64) * pitch as u64;
                for x in 0..w as usize {
                    core::ptr::write_volatile(
                        base.add(off as usize).add(x * 4) as *mut u32,
                        TERM_BG,
                    );
                }
            }
        }
    }
}

pub unsafe fn term_putchar(c: u8) {
    match c {
        b'\n' => { term_newline(); return; }
        b'\r' => { TERM_X = 0; return; }
        b'\x08' => { if TERM_X > 0 { TERM_X -= 1; } return; }
        b'\t' => {
            let next = (TERM_X / TAB_WIDTH + 1) * TAB_WIDTH;
            while TERM_X < next && TERM_X < TERM_COLS {
                term_putchar(b' ');
            }
            return;
        }
        _ => {}
    }

    if TERM_X >= TERM_COLS {
        term_newline();
    }

    if c < 32 || c > 126 {
        TERM_X += 1;
        return;
    }

    let px = (TERM_X * 8) as u32;
    let py = (TERM_Y * 16) as u32;
    let w = gop_get_width();
    let h = gop_get_height();
    if px + 8 > w || py + 16 > h {
        TERM_X += 1;
        return;
    }

    let idx = (c - 32) as usize;
    let pitch_px = LD_FB.pitch / 4;
    let base = LD_FB.base as *mut u32;
    for row in 0..16 {
        let bits = crate::font::FONT_8X16[idx][row];
        let line = base.offset(((py as u64 + row as u64) * pitch_px as u64 + px as u64) as isize);
        if bits == 0xFF {
            for col in 0..8 {
                core::ptr::write_volatile(line.add(col), TERM_FG);
            }
        } else if bits == 0x00 {
            for col in 0..8 {
                core::ptr::write_volatile(line.add(col), TERM_BG);
            }
        } else {
            for col in 0..8 {
                let color = if bits & (0x80 >> col) != 0 { TERM_FG } else { TERM_BG };
                core::ptr::write_volatile(line.add(col), color);
            }
        }
    }
    TERM_X += 1;
}

pub unsafe fn term_write(s: *const u8) {
    if s.is_null() { return; }
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 { break; }
        term_putchar(c);
        i += 1;
    }
}

pub unsafe fn term_writeln(s: *const u8) {
    term_write(s);
    term_newline();
}

pub unsafe fn term_set_fg(c: u32) {
    TERM_FG = c;
}

pub unsafe fn term_set_bg(c: u32) {
    TERM_BG = c;
}

pub unsafe fn term_set_pos(x: i32, y: i32) {
    if x >= 0 && x < TERM_COLS { TERM_X = x; }
    if y >= 0 && y < TERM_ROWS { TERM_Y = y; }
}

pub fn term_get_width() -> i32 {
    unsafe { TERM_COLS }
}

pub fn term_get_height() -> i32 {
    unsafe { TERM_ROWS }
}
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
pub unsafe fn fat_set_partition_offset(offset: u64) {
    G_PARTITION_OFFSET = offset;
}
pub unsafe fn fat_install_bootloader(target_handle: efi_handle, part_start_lba: u64) -> i32 {
    let boot_dev = crate::get_boot_device();
    if boot_dev.is_null() { return -1; }

    let saved_offset = G_PARTITION_OFFSET;
    let saved_block_io = G_BLOCK_IO;

    /* Switch to boot device and read BOOTX64.EFI */
    G_PARTITION_OFFSET = 0;
    if fat_set_device(boot_dev) != 0 {
        G_PARTITION_OFFSET = saved_offset;
        G_BLOCK_IO = saved_block_io;
        return -1;
    }
    let path = b"/EFI/BOOT/BOOTX64.EFI\0" as *const u8;
    let file_sz = fat_get_file_size(path);
    if file_sz <= 0 {
        G_PARTITION_OFFSET = saved_offset;
        G_BLOCK_IO = saved_block_io;
        return -1;
    }
    let buf = uefi_alloc(file_sz as usize);
    if buf.is_null() {
        G_PARTITION_OFFSET = saved_offset;
        G_BLOCK_IO = saved_block_io;
        return -1;
    }
    if fat_read_file(path, buf as *mut c_void, file_sz as u32) != file_sz {
        uefi_free(buf, file_sz as usize);
        G_PARTITION_OFFSET = saved_offset;
        G_BLOCK_IO = saved_block_io;
        return -1;
    }

    /* Switch to target partition and write BOOTX64.EFI */
    G_PARTITION_OFFSET = part_start_lba;
    if fat_set_device(target_handle) != 0 {
        uefi_free(buf, file_sz as usize);
        G_PARTITION_OFFSET = saved_offset;
        G_BLOCK_IO = saved_block_io;
        return -1;
    }
    /* Ensure /EFI and /EFI/BOOT directories exist on target */
    if fat_exists(b"/EFI\0" as *const u8) == 0 { fat_mkdir(b"/EFI\0" as *const u8); }
    if fat_exists(b"/EFI/BOOT\0" as *const u8) == 0 { fat_mkdir(b"/EFI/BOOT\0" as *const u8); }
    let rc = fat_write_file(path, buf as *const c_void, file_sz as u32);

    uefi_free(buf, file_sz as usize);
    G_PARTITION_OFFSET = saved_offset;
    G_BLOCK_IO = saved_block_io;
    rc
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

/* ---- LumFS ---- */
pub unsafe fn lumfs_init() -> i32 {
    lumfs::init()
}
pub unsafe fn lumfs_set_bs(bs: *mut EfiBootServices, _img: efi_handle, _st: *mut EfiSystemTable) {
    G_BS = bs;
    lumfs::set_alloc(Some(uefi_alloc as AllocFn), Some(uefi_free as FreeFn));
}
pub unsafe fn lumfs_set_device(handle: efi_handle) -> i32 {
    let bs = G_BS;
    if bs.is_null() || handle.is_null() { return -1; }
    let block_io_guid = crate::uefi::EFI_BLOCK_IO_GUID;
    let mut block_io: *mut crate::uefi::EfiBlockIoProtocol = ptr::null_mut();
    let status = ((*bs).handle_protocol.unwrap())(
        handle,
        &block_io_guid as *const crate::uefi::EfiGuid as *mut crate::uefi::EfiGuid,
        &mut block_io as *mut *mut crate::uefi::EfiBlockIoProtocol as *mut *mut c_void,
    );
    if status != 0 || block_io.is_null() { return -1; }
    G_BLOCK_IO = block_io;
    let read_fn: FatReadFn = core::mem::transmute(uefi_read_sectors as *const () as usize);
    let write_fn: FatWriteFn = core::mem::transmute(uefi_write_sectors as *const () as usize);
    lumfs::set_drive(Some(read_fn), Some(write_fn));
    lumfs::init()
}
pub unsafe fn lumfs_format_at(start_lba: u64, total_sectors: u64) -> i32 {
    lumfs::format_at(start_lba, total_sectors)
}
pub unsafe fn lumfs_read_file(path: *const u8, buf: *mut c_void, max: u32) -> i32 {
    lumfs::read_file(path, buf as *mut u8, max)
}
pub unsafe fn lumfs_write_file(path: *const u8, data: *const c_void, size: u32) -> i32 {
    lumfs::write_file(path, data as *const u8, size)
}
pub unsafe fn lumfs_exists(path: *const u8) -> i32 {
    if lumfs::exists(path) { 1 } else { 0 }
}
pub unsafe fn lumfs_delete(path: *const u8) -> i32 {
    lumfs::delete(path)
}
pub unsafe fn lumfs_mkdir(path: *const u8) -> i32 {
    lumfs::mkdir(path)
}
pub unsafe fn lumfs_get_file_size(path: *const u8) -> i32 {
    lumfs::get_file_size(path)
}
pub unsafe fn lumfs_rename(old_path: *const u8, new_path: *const u8) -> i32 {
    lumfs::rename(old_path, new_path)
}
pub unsafe fn lumfs_copy_file(src_path: *const u8, dst_path: *const u8) -> i32 {
    lumfs::copy_file(src_path, dst_path)
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
pub unsafe fn exit_boot_services() {
    /* In the loader context, this is intentionally a no-op.
     * The loader uses UEFI boot services throughout its lifecycle.
     * exit_boot_services() is only meaningful for the kernel itself,
     * which manages its own memory after this point. */
}

pub unsafe fn lumie_efi_register_boot_entry() -> i32 {
    let st_ptr = crate::input::get_ld_st();
    if st_ptr.is_null() { return -1; }
    let st = &*st_ptr;
    let rt = st.runtime_services;
    if rt.is_null() { return -1; }
    let bs = st.boot_services;
    if bs.is_null() { return -1; }

    let image_handle = G_IMAGE_HANDLE;
    if image_handle.is_null() { return -1; }

    /* Get loaded image protocol to find the file path */
    let li_guid = &EFI_LOADED_IMAGE_PROTOCOL_GUID as *const EfiGuid;
    let mut li: *mut c_void = ptr::null_mut();
    if let Some(hp) = (*bs).handle_protocol {
        if hp(image_handle, li_guid, &mut li) != 0 || li.is_null() {
            return -1;
        }
    } else {
        return -1;
    }

    /* Get FilePath from loaded image */
    let li_typed = &*(li as *mut crate::uefi::EfiLoadedImageProtocol);
    let file_path_ptr = li_typed.file_path as *const u8;
    if file_path_ptr.is_null() { return -1; }

    /* Calculate device path length (walk the path nodes) */
    let mut dp_len: usize = 0;
    let mut p = file_path_ptr;
    loop {
        let node_type = *p;
        let node_subtype = *p.add(1);
        let node_len = (*p.add(2) as usize) | ((*p.add(3) as usize) << 8);
        if node_len < 4 { break; }
        dp_len += node_len;
        if node_type == 0x7F && node_subtype == 0xFF { break; }
        p = p.add(node_len);
        if dp_len > 2048 { break; }
    }
    if dp_len == 0 { return -1; }

    /* Build EFI_LOAD_OPTION:
     *   Attributes:           u32  = 1 (ACTIVE)
     *   FilePathListLength:   u16  = dp_len
     *   Description:          "LumieOS\0" as UTF-16 (16 bytes)
     *   FilePathList:         device path bytes
     */
    let desc_utf16: [u16; 8] = [
        b'L' as u16, b'u' as u16, b'm' as u16, b'i' as u16,
        b'e' as u16, b'O' as u16, b'S' as u16, 0,
    ];
    let desc_byte_len: usize = 16; /* 8 chars * 2 bytes */
    let total_size = 4 + 2 + desc_byte_len + dp_len;

    let buf = uefi_alloc(total_size);
    if buf.is_null() { return -1; }

    *(buf as *mut u32) = 0x01; /* EFI_LOAD_OPTION_ACTIVE */
    *(buf.add(4) as *mut u16) = dp_len as u16;
    core::ptr::copy_nonoverlapping(desc_utf16.as_ptr(), buf.add(6) as *mut u16, 8);
    core::ptr::copy_nonoverlapping(file_path_ptr, buf.add(6 + desc_byte_len), dp_len);

    /* Find free Boot#### number */
    let global_guid = &EFI_GLOBAL_VARIABLE_GUID as *const EfiGuid;
    let mut boot_num: u16 = 0xFFFF;
    let hex_digits = b"0123456789ABCDEF";
    for try_num in 0x0000u16..0x0100u16 {
        let mut name_buf: [u16; 9] = [0u16; 9];
        name_buf[0] = b'B' as u16;
        name_buf[1] = b'o' as u16;
        name_buf[2] = b'o' as u16;
        name_buf[3] = b't' as u16;
        name_buf[4] = hex_digits[((try_num >> 12) & 0xF) as usize] as u16;
        name_buf[5] = hex_digits[((try_num >> 8) & 0xF) as usize] as u16;
        name_buf[6] = hex_digits[((try_num >> 4) & 0xF) as usize] as u16;
        name_buf[7] = hex_digits[(try_num & 0xF) as usize] as u16;
        name_buf[8] = 0;

        let mut existing_size: u64 = 0;
        if let Some(gv) = (*rt).get_variable {
            let status = gv(name_buf.as_mut_ptr(), global_guid, ptr::null_mut(), &mut existing_size, ptr::null_mut());
            if status != 0 {
                boot_num = try_num;
                break;
            }
        } else {
            uefi_free(buf, total_size);
            return -1;
        }
    }

    if boot_num == 0xFFFF {
        uefi_free(buf, total_size);
        return -1;
    }

    /* Set Boot#### variable */
    let mut var_name: [u16; 9] = [0u16; 9];
    var_name[0] = b'B' as u16;
    var_name[1] = b'o' as u16;
    var_name[2] = b'o' as u16;
    var_name[3] = b't' as u16;
    var_name[4] = hex_digits[((boot_num >> 12) & 0xF) as usize] as u16;
    var_name[5] = hex_digits[((boot_num >> 8) & 0xF) as usize] as u16;
    var_name[6] = hex_digits[((boot_num >> 4) & 0xF) as usize] as u16;
    var_name[7] = hex_digits[(boot_num & 0xF) as usize] as u16;
    var_name[8] = 0;

    let var_attrs: u32 = 0x01 | 0x04 | 0x08; /* NON_VOLATILE | BOOTSERVICE_ACCESS | RUNTIME_ACCESS */
    if let Some(sfunc) = (*rt).set_variable {
        let result = sfunc(
            var_name.as_mut_ptr(),
            global_guid,
            var_attrs,
            total_size as u64,
            buf as *mut c_void,
        );
        if result != 0 {
            term_set_fg(0xFF0000);
            let mut errmsg: [u8; 64] = [0u8; 64];
            let mut ep = 0;
            for &c in b"BOOT VAR WRITE FAIL: " { if ep < 63 { errmsg[ep] = c; ep += 1; } }
            lumie_std::format::lumie_itoa(boot_num as i64, errmsg.as_mut_ptr().add(ep), 10);
            term_writeln(errmsg.as_ptr());
            term_set_fg(0xFFFFFF);
            uefi_free(buf, total_size);
            return -1;
        }
    } else {
        uefi_free(buf, total_size);
        return -1;
    }

    uefi_free(buf, total_size);

    /* Update BootOrder: append at end (not prepend) so existing OSes keep their position */
    let boot_order_name: [u16; 10] = [
        b'B' as u16, b'o' as u16, b'o' as u16, b't' as u16,
        b'O' as u16, b'r' as u16, b'd' as u16, b'e' as u16,
        b'r' as u16, 0,
    ];
    let mut boot_order_buf: [u16; 128] = [0u16; 128];
    let mut boot_order_size: u64 = 256;
    let mut bo_attrs: u32 = 0;

    let existing_count = if let Some(gv) = (*rt).get_variable {
        let status = gv(boot_order_name.as_ptr() as *mut u16, global_guid, &mut bo_attrs, &mut boot_order_size, boot_order_buf.as_mut_ptr() as *mut c_void);
        if status == 0 { (boot_order_size / 2) as usize } else { 0 }
    } else { 0 };

    let new_bo_size = (existing_count + 1) * 2;
    let new_bo = uefi_alloc(new_bo_size);
    if new_bo.is_null() { return -1; }

    *(new_bo as *mut u16) = boot_num;
    if existing_count > 0 {
        core::ptr::copy_nonoverlapping(
            boot_order_buf.as_ptr(),
            new_bo.add(2) as *mut u16,
            existing_count,
        );
    }

    if let Some(sfunc) = (*rt).set_variable {
        let bo_result = sfunc(
            boot_order_name.as_ptr() as *mut u16,
            global_guid,
            var_attrs,
            new_bo_size as u64,
            new_bo as *mut c_void,
        );
        if bo_result != 0 {
            term_set_fg(0xFF0000);
            term_writeln(b"BOOTORDER WRITE FAIL\0" as *const u8);
            term_set_fg(0xFFFFFF);
        }
    }

    uefi_free(new_bo, new_bo_size);
    0
}
pub unsafe fn lumie_efi_register_boot_entry_for_target(
    target_handle: efi_handle,
    part_start_lba: u64,
    part_sectors: u64,
) -> i32 {
    let st_ptr = crate::input::get_ld_st();
    if st_ptr.is_null() { return -1; }
    let st = &*st_ptr;
    let rt = st.runtime_services;
    if rt.is_null() { return -1; }
    let bs = st.boot_services;
    if bs.is_null() { return -1; }

    let dp_guid = &EFI_DEVICE_PATH_PROTOCOL_GUID as *const EfiGuid;
    let mut hw_dp: *mut c_void = ptr::null_mut();
    if let Some(hp) = (*bs).handle_protocol {
        if hp(target_handle, dp_guid, &mut hw_dp) != 0 || hw_dp.is_null() {
            term_set_fg(0xFF0000);
            term_writeln(b"ERROR: Cannot get target device path.\0" as *const u8);
            term_set_fg(0xFFFFFF);
            return -1;
        }
    } else {
        return -1;
    }

    let mut hw_dp_len: usize = 0;
    let mut p = hw_dp as *const u8;
    loop {
        let node_type = *p;
        let node_subtype = *p.add(1);
        let node_len = (*p.add(2) as usize) | ((*p.add(3) as usize) << 8);
        if node_len < 4 { break; }
        if node_type == 0x7F && node_subtype == 0xFF { break; }
        hw_dp_len += node_len;
        p = p.add(node_len);
        if hw_dp_len > 2048 { break; }
    }
    if hw_dp_len == 0 { return -1; }

    const PART_GUID: [u8; 16] = [
        0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88,
        0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00,
    ];
    let mut hd_node: [u8; 42] = [0; 42];
    hd_node[0] = 4;
    hd_node[1] = 1;
    hd_node[2] = 42;
    hd_node[3] = 0;
    *(hd_node.as_mut_ptr().add(4) as *mut u32) = 1;
    *(hd_node.as_mut_ptr().add(8) as *mut u64) = part_start_lba;
    *(hd_node.as_mut_ptr().add(16) as *mut u64) = part_sectors * 512;
    core::ptr::copy_nonoverlapping(PART_GUID.as_ptr(), hd_node.as_mut_ptr().add(24), 16);
    hd_node[40] = 2;
    hd_node[41] = 2;

    let boot_path_utf16: [u16; 22] = [
        '\\' as u16, 'E' as u16, 'F' as u16, 'I' as u16,
        '\\' as u16, 'B' as u16, 'O' as u16, 'O' as u16,
        'T' as u16, '\\' as u16, 'B' as u16, 'O' as u16,
        'O' as u16, 'T' as u16, 'X' as u16, '6' as u16,
        '4' as u16, '.' as u16, 'E' as u16, 'F' as u16,
        'I' as u16, 0,
    ];
    let fp_path_bytes = 44usize;
    let fp_node_len: u16 = (4 + fp_path_bytes) as u16;
    let mut fp_node: [u8; 48] = [0; 48];
    fp_node[0] = 4;
    fp_node[1] = 2;
    fp_node[2] = fp_node_len as u8;
    fp_node[3] = (fp_node_len >> 8) as u8;
    core::ptr::copy_nonoverlapping(boot_path_utf16.as_ptr(), fp_node.as_mut_ptr().add(4) as *mut u16, 22);

    let end_node: [u8; 4] = [0x7F, 0xFF, 4, 0];

    let total_dp_len = hw_dp_len + 42 + fp_node_len as usize + 4;

    let desc_utf16: [u16; 8] = [
        'L' as u16, 'u' as u16, 'm' as u16, 'i' as u16,
        'e' as u16, 'O' as u16, 'S' as u16, 0,
    ];
    let desc_byte_len: usize = 16;
    let total_size = 4 + 2 + desc_byte_len + total_dp_len;

    let buf = uefi_alloc(total_size);
    if buf.is_null() { return -1; }

    *(buf as *mut u32) = 0x01;
    *(buf.add(4) as *mut u16) = total_dp_len as u16;
    core::ptr::copy_nonoverlapping(desc_utf16.as_ptr(), buf.add(6) as *mut u16, 8);

    let dp_off = 6 + desc_byte_len;
    core::ptr::copy_nonoverlapping(hw_dp as *const u8, buf.add(dp_off), hw_dp_len);
    let hd_off = dp_off + hw_dp_len;
    core::ptr::copy_nonoverlapping(hd_node.as_ptr(), buf.add(hd_off), 42);
    let fp_off = hd_off + 42;
    core::ptr::copy_nonoverlapping(fp_node.as_ptr(), buf.add(fp_off), fp_node_len as usize);
    let end_off = fp_off + fp_node_len as usize;
    core::ptr::copy_nonoverlapping(end_node.as_ptr(), buf.add(end_off), 4);

    let global_guid = &EFI_GLOBAL_VARIABLE_GUID as *const EfiGuid;
    let mut boot_num: u16 = 0xFFFF;
    let hex_digits = b"0123456789ABCDEF";
    for try_num in 0x0000u16..0x0100u16 {
        let mut name_buf: [u16; 9] = [0u16; 9];
        name_buf[0] = 'B' as u16;
        name_buf[1] = 'o' as u16;
        name_buf[2] = 'o' as u16;
        name_buf[3] = 't' as u16;
        name_buf[4] = hex_digits[((try_num >> 12) & 0xF) as usize] as u16;
        name_buf[5] = hex_digits[((try_num >> 8) & 0xF) as usize] as u16;
        name_buf[6] = hex_digits[((try_num >> 4) & 0xF) as usize] as u16;
        name_buf[7] = hex_digits[(try_num & 0xF) as usize] as u16;
        name_buf[8] = 0;
        let mut existing_size: u64 = 0;
        if let Some(gv) = (*rt).get_variable {
            let status = gv(name_buf.as_mut_ptr(), global_guid, ptr::null_mut(), &mut existing_size, ptr::null_mut());
            if status != 0 {
                boot_num = try_num;
                break;
            }
        } else {
            uefi_free(buf, total_size);
            return -1;
        }
    }
    if boot_num == 0xFFFF {
        uefi_free(buf, total_size);
        return -1;
    }

    let mut var_name: [u16; 9] = [0u16; 9];
    var_name[0] = 'B' as u16;
    var_name[1] = 'o' as u16;
    var_name[2] = 'o' as u16;
    var_name[3] = 't' as u16;
    var_name[4] = hex_digits[((boot_num >> 12) & 0xF) as usize] as u16;
    var_name[5] = hex_digits[((boot_num >> 8) & 0xF) as usize] as u16;
    var_name[6] = hex_digits[((boot_num >> 4) & 0xF) as usize] as u16;
    var_name[7] = hex_digits[(boot_num & 0xF) as usize] as u16;
    var_name[8] = 0;

    let var_attrs: u32 = 0x01 | 0x04 | 0x08;
    if let Some(sfunc) = (*rt).set_variable {
        let result = sfunc(
            var_name.as_mut_ptr(),
            global_guid,
            var_attrs,
            total_size as u64,
            buf as *mut c_void,
        );
        if result != 0 {
            term_set_fg(0xFF0000);
            term_writeln(b"ERROR: Failed to write Boot variable.\0" as *const u8);
            term_set_fg(0xFFFFFF);
            uefi_free(buf, total_size);
            return -1;
        }
    } else {
        uefi_free(buf, total_size);
        return -1;
    }

    uefi_free(buf, total_size);

    let mut boot_order_name: [u16; 10] = [
        'B' as u16, 'o' as u16, 'o' as u16, 't' as u16,
        'O' as u16, 'r' as u16, 'd' as u16, 'e' as u16,
        'r' as u16, 0,
    ];
    let mut cur_order: *mut u8 = ptr::null_mut();
    let mut cur_order_size: u64 = 0;
    if let Some(gv) = (*rt).get_variable {
        let _ = gv(boot_order_name.as_mut_ptr(), global_guid, ptr::null_mut(), &mut cur_order_size, ptr::null_mut());
    }
    if cur_order_size > 0 {
        cur_order = uefi_alloc(cur_order_size as usize);
    }
    if !cur_order.is_null() {
        if let Some(gv) = (*rt).get_variable {
            let _ = gv(boot_order_name.as_mut_ptr(), global_guid, ptr::null_mut(), &mut cur_order_size, cur_order as *mut c_void);
        }
    }

    let new_bo_size = 2 + cur_order_size as usize + 2;
    let new_bo = uefi_alloc(new_bo_size);
    if new_bo.is_null() {
        if !cur_order.is_null() { uefi_free(cur_order, cur_order_size as usize); }
        return -1;
    }
    *(new_bo as *mut u16) = boot_num;
    if !cur_order.is_null() && cur_order_size > 0 {
        core::ptr::copy_nonoverlapping(cur_order, new_bo.add(2), cur_order_size as usize);
    }
    if let Some(sfunc) = (*rt).set_variable {
        let _ = sfunc(
            boot_order_name.as_mut_ptr(),
            global_guid,
            var_attrs,
            new_bo_size as u64,
            new_bo as *mut c_void,
        );
    }
    if !cur_order.is_null() { uefi_free(cur_order, cur_order_size as usize); }
    uefi_free(new_bo, new_bo_size);
    0
}
pub unsafe fn lumie_load_shell_module() -> i32 {
    0
}
pub unsafe fn lumie_cache_kernel_image(_base: *const c_void, _size: u32) {}
pub unsafe fn lumie_sched_init() {}
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

    if file_count > 4096 { uefi_free(data, sz as usize); return -6; }
    if entries_off as u64 >= sz as u64 || entries_off < 16 { uefi_free(data, sz as usize); return -7; }
    let min_size = entries_off as u64 + (file_count as u64) * 32;
    if min_size > sz as u64 { uefi_free(data, sz as usize); return -8; }

    let h = pkg as *mut u8;
    *(h as *mut *mut u8) = data;
    *(h.add(8) as *mut u32) = sz as u32;
    *(h.add(12) as *mut u32) = file_count;
    *(h.add(16) as *mut u32) = entries_off;
    0
}

/// LZ1 block decompression.
/// Custom format: control byte (8 flags MSB), literals (0) or 3-byte references (1: u16 LE offset + u8 length-3).
/// Returns 0 on success, negative on error.
pub unsafe fn lz1_decompress(src: *const u8, src_len: u32, dst: *mut u8, dst_len: u32) -> i32 {
    let mut ip: usize = 0;
    let mut op: usize = 0;
    let src_end = src_len as usize;
    let dst_end = dst_len as usize;

    while ip < src_end && op < dst_end {
        let mut ctrl = *src.add(ip) as u32;
        ip += 1;
        for _ in 0..8 {
            if ip >= src_end || op >= dst_end { break; }
            if ctrl & 0x80 != 0 {
                // match: 3 bytes follow
                if ip + 3 > src_end { return -1; }
                let offset = *src.add(ip) as usize | ((*src.add(ip + 1) as usize) << 8);
                let length = *src.add(ip + 2) as usize + 3;
                ip += 3;
                if offset == 0 || offset > op { return -1; }
                if op + length > dst_end { return -1; }
                let mut match_pos = op - offset;
                for _ in 0..length {
                    *dst.add(op) = *dst.add(match_pos);
                    op += 1;
                    match_pos += 1;
                }
            } else {
                // literal
                *dst.add(op) = *src.add(ip);
                ip += 1;
                op += 1;
            }
            ctrl <<= 1;
        }
    }

    if op != dst_len as usize { -2 } else { 0 }
}

pub unsafe fn install_pkg_extract_all(pkg: *mut c_void, _progress: *mut c_void) -> i32 {
    let h = pkg as *mut u8;
    let data = *(h as *mut *mut u8);
    let entry_count = *(h.add(12) as *mut u32);
    let entries_off = *(h.add(16) as *mut u32);

    let write_fn = G_PKG_WRITE.unwrap_or(fat_write_file as InstallWriteFn);
    let is_ntfs = write_fn as usize == ntfs_write_file as *const () as usize;

    for i in 0..entry_count {
        let entry = data.add(entries_off as usize + (i * 32) as usize);
        let path_off = *(entry as *const u32);
        let data_off = *(entry.add(4) as *const u32);
        let store_sz = *(entry.add(8) as *const u32);
        let flags = *(entry.add(12) as *const u8);

        let path = data.add(path_off as usize);
        let file_data = data.add(data_off as usize);

        if flags & 1 != 0 {
            if is_ntfs {
                if ntfs_exists(path) == 0 { ntfs_mkdir(path); }
            } else {
                if fat_exists(path) == 0 { fat_mkdir(path); }
            }
        } else if flags & 2 != 0 {
            let orig_sz = *(entry.add(16) as *const u32);
            let buf = uefi_alloc(orig_sz as usize);
            if buf.is_null() { return -1; }
            let ret = lz1_decompress(file_data, store_sz, buf, orig_sz);
            if ret != 0 {
                uefi_free(buf, orig_sz as usize);
                return -2;
            }
            write_fn(path, buf as *const c_void, orig_sz);
            uefi_free(buf, orig_sz as usize);
        } else {
            write_fn(path, file_data as *const c_void, store_sz);
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
pub unsafe fn sys_load(path: *const u8, _boot_info: *mut SysBootInfo, mod_out: *mut SysModule) -> i32 {
    if path.is_null() || mod_out.is_null() {
        return -1;
    }
    ptr::write_bytes(mod_out as *mut u8, 0, core::mem::size_of::<SysModule>());

    let sz = fat_get_file_size(path);
    if sz <= 64 { return -2; }

    let raw = uefi_alloc(sz as usize);
    if raw.is_null() { return -3; }

    let r = fat_read_file(path, raw as *mut c_void, sz as u32);
    if r != sz {
        uefi_free(raw, sz as usize);
        return -4;
    }

    let magic = *(raw as *const u32);
    if magic != 0x4E524B4C
        && magic != 0x48534C4C
        && magic != 0x5652444C
        && magic != 0x01535953
    {
        uefi_free(raw, sz as usize);
        return -5;
    }

    let entry_off  = *(raw.add(4)  as *const u32);
    let code_size  = *(raw.add(8)  as *const u32);
    let bss_size   = *(raw.add(12) as *const u32);
    let reloc_off  = *(raw.add(16) as *const u32);
    let reloc_count = *(raw.add(20) as *const u32);

    let total_img = code_size as u64 + bss_size as u64;
    if code_size == 0 || total_img > 16 * 1024 * 1024 {
        uefi_free(raw, sz as usize);
        return -6;
    }

    let load_base = uefi_alloc(total_img as usize);
    if load_base.is_null() {
        uefi_free(raw, sz as usize);
        return -7;
    }

    ptr::write_bytes(load_base, 0, total_img as usize);
    ptr::copy_nonoverlapping(raw.add(64), load_base, code_size as usize);

    if reloc_off != 0 && reloc_count != 0 {
        let relocs = raw.add(reloc_off as usize) as *const u32;
        let base = load_base as u64;
        for i in 0..reloc_count as usize {
            let off = *relocs.add(i) as usize;
            if off + 8 <= total_img as usize {
                let slot = load_base.add(off) as *mut u64;
                *slot = (*slot).wrapping_add(base);
            }
        }
    }

    (*mod_out).base = load_base as *mut c_void;
    (*mod_out).size = total_img as u32;
    if entry_off != 0 && (entry_off as u64) < total_img {
        (*mod_out).entry = load_base.add(entry_off as usize) as *mut c_void;
    }

    uefi_free(raw, sz as usize);
    0
}
pub unsafe fn desktop_init() {}
pub unsafe fn desktop_run() {}
pub unsafe fn sched_init() {}
pub unsafe fn disk_get_info(_index: i32) -> *const c_void {
    core::ptr::null()
}
pub unsafe fn lumie_reboot() {
    let st_ptr = crate::input::get_ld_st();
    if st_ptr.is_null() { return; }
    let rt = (*st_ptr).runtime_services;
    if rt.is_null() { return; }
    if let Some(reset) = (*rt).reset_system {
        reset(0, 0, 0, core::ptr::null_mut());
    }
}
#[allow(non_upper_case_globals)]
pub static mut g_nv_gpu_api: *mut c_void = core::ptr::null_mut();
