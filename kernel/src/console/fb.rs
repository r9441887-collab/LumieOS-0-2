#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FbInfo {
    pub base: u64,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u32,
    pub pixel_format: u32,
}

pub const PIXEL_FORMAT_RGBX_8BPP: u32 = 0;
pub const PIXEL_FORMAT_BGRX_8BPP: u32 = 1;
