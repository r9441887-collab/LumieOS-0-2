#![no_std]

use core::ptr;
use crate::uefi::types::*;
use crate::uefi::guid::EFI_GOP_GUID;
use crate::uefi::tables::EfiSystemTable;
use crate::uefi::protocols::gop::{EfiGopProtocol, EfiGopModeInfo};
use super::fb::FbInfo;
use super::nv_gpu::{self, G_NV_GPU_API};

pub static mut GOP_PROTO: Option<&'static EfiGopProtocol> = None;

pub static mut FB_INFO: FbInfo = FbInfo {
    base: 0,
    size: 0,
    width: 0,
    height: 0,
    pitch: 0,
    bpp: 0,
    pixel_format: 0,
};

pub static mut NV_ACTIVE: i32 = 0;

pub unsafe fn init(_image_handle: EfiHandle, st: &EfiSystemTable) -> efi_status {
    let bs = (*st).boot_services;
    let locate_protocol = ((*bs).locate_protocol).unwrap();
    let mut gop_ptr: *mut core::ffi::c_void = ptr::null_mut();

    let status = locate_protocol(
        &EFI_GOP_GUID as *const _ as *const crate::uefi::guid::EfiGuid,
        ptr::null_mut(),
        &mut gop_ptr,
    );
    if (status as i64) < 0 {
        return status;
    }
    if gop_ptr.is_null() {
        return efi_err(1);
    }

    let gop = &mut *(gop_ptr as *mut EfiGopProtocol);
    let mode = &*gop.mode;

    let set_mode = gop.set_mode.unwrap();
    let s = set_mode(gop as *mut _ as *mut core::ffi::c_void, mode.mode);
    if (s as i64) < 0 {
        return s;
    }

    let query_mode = gop.query_mode.unwrap();
    let mut info_size: u64 = 0;
    let mut info: *mut EfiGopModeInfo = ptr::null_mut();
    let qs = query_mode(
        gop as *mut _ as *mut core::ffi::c_void,
        mode.mode,
        &mut info_size,
        &mut info,
    );

    let fb_info = if (qs as i64) < 0 || info.is_null() {
        mode.info
    } else {
        info
    };

    if fb_info.is_null() {
        return efi_err(1);
    }

    GOP_PROTO = Some(&*gop);
    FB_INFO.base = mode.frame_buffer_base;
    FB_INFO.size = mode.frame_buffer_size;
    FB_INFO.width = (*fb_info).horizontal_resolution;
    FB_INFO.height = (*fb_info).vertical_resolution;
    {
        let pitch64 = (*fb_info).pixels_per_scan_line as u64 * 4;
        FB_INFO.pitch = if pitch64 > 0xFFFFFFFF {
            0xFFFFFFFF
        } else {
            pitch64 as u32
        };
    }
    FB_INFO.bpp = 32;
    FB_INFO.pixel_format = (*fb_info).pixel_format;

    EFI_SUCCESS
}

pub unsafe fn gop_make_color(r: u8, g: u8, b: u8) -> u32 {
    if FB_INFO.pixel_format == 0 {
        r as u32 | (g as u32) << 8 | (b as u32) << 16
    } else {
        (r as u32) << 16 | (g as u32) << 8 | b as u32
    }
}

pub unsafe fn put_pixel(x: u32, y: u32, color: u32) {
    if x >= FB_INFO.width || y >= FB_INFO.height {
        return;
    }
    let ptr = FB_INFO.base as *mut u32;
    let pitch_px = FB_INFO.pitch / 4;
    ptr::write_volatile(
        ptr.offset((y as u64 * pitch_px as u64 + x as u64) as isize),
        color,
    );
}

pub unsafe fn get_pixel(x: u32, y: u32) -> u32 {
    if x >= FB_INFO.width || y >= FB_INFO.height {
        return 0;
    }
    let ptr = FB_INFO.base as *mut u32;
    let pitch_px = FB_INFO.pitch / 4;
    ptr::read_volatile(
        ptr.offset((y as u64 * pitch_px as u64 + x as u64) as isize) as *const u32,
    )
}

pub unsafe fn fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    if x >= FB_INFO.width || y >= FB_INFO.height {
        return;
    }
    let w = if w > FB_INFO.width - x {
        FB_INFO.width - x
    } else {
        w
    };
    let h = if h > FB_INFO.height - y {
        FB_INFO.height - y
    } else {
        h
    };

    if NV_ACTIVE != 0 && w * h >= nv_gpu::NV_GPU_FILL_THRESHOLD {
        if let Some(api) = G_NV_GPU_API {
            if let Some(fill_rect_fn) = api.fill_rect {
                fill_rect_fn(x, y, w, h, color);
                return;
            }
        }
        nv_gpu::nv_gpu_fill_rect(x, y, w, h, color);
        return;
    }

    let base = FB_INFO.base as *mut u32;
    let pitch_px = FB_INFO.pitch / 4;
    for j in 0..h {
        let line = base.offset(((y + j) as u64 * pitch_px as u64 + x as u64) as isize);
        for i in 0..w {
            ptr::write_volatile(line.offset(i as isize), color);
        }
    }
}

pub unsafe fn draw_char(x: u32, y: u32, fg: u32, bg: u32, c: u8) {
    let c = if c < 32 || c > 126 { b'.' } else { c };
    if x + 8 > FB_INFO.width || y + 16 > FB_INFO.height {
        return;
    }
    let base = FB_INFO.base as *mut u32;
    let pitch_px = FB_INFO.pitch / 4;
    let idx = (c - 32) as usize;
    for row in 0..16 {
        let bits = super::font::FONT_8X16[idx][row];
        let line = base.offset(((y + row) as u64 * pitch_px as u64 + x as u64) as isize);
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
}

pub unsafe fn draw_string(mut x: u32, y: u32, fg: u32, bg: u32, s: &str) {
    for &c in s.as_bytes() {
        draw_char(x, y, fg, bg, c);
        x += 8;
    }
}

pub fn get_fb() -> &'static FbInfo {
    unsafe { &FB_INFO }
}

pub fn get_width() -> u32 {
    unsafe { FB_INFO.width }
}

pub fn get_height() -> u32 {
    unsafe { FB_INFO.height }
}

pub unsafe fn nv_init() -> i32 {
    if let Some(api) = G_NV_GPU_API {
        if let Some(is_active) = api.is_active {
            if is_active() != 0 {
                if let Some(set_fb) = api.set_fb {
                    set_fb(FB_INFO.base, FB_INFO.width, FB_INFO.height, FB_INFO.pitch);
                }
                NV_ACTIVE = 1;
                return 1;
            }
        }
    }
    let ret = nv_gpu::nv_gpu_init(FB_INFO.base, FB_INFO.width, FB_INFO.height, FB_INFO.pitch);
    NV_ACTIVE = ret;
    ret
}

pub fn nv_active() -> bool {
    unsafe { NV_ACTIVE != 0 }
}

pub unsafe fn nv_fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    if let Some(api) = G_NV_GPU_API {
        if let Some(fill_rect_fn) = api.fill_rect {
            fill_rect_fn(x, y, w, h, color);
            return;
        }
    }
    nv_gpu::nv_gpu_fill_rect(x, y, w, h, color);
}
