
use crate::uefi::types::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiPixelBitmask {
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub reserved_mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiGopModeInfo {
    pub version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: u32,
    pub pixel_information: EfiPixelBitmask,
    pub pixels_per_scan_line: u32,
}

#[repr(C)]
pub struct EfiGopMode {
    pub max_mode: u32,
    pub mode: u32,
    pub info: *mut EfiGopModeInfo,
    pub size_of_info: u64,
    pub frame_buffer_base: u64,
    pub frame_buffer_size: u64,
}

pub type EfiGopQueryMode = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u32, *mut u64, *mut *mut EfiGopModeInfo) -> efi_status>;
pub type EfiGopSetMode = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u32) -> efi_status>;
pub type EfiGopBlt = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, u32, u32, u32, u32, u32, u32, u32, u32) -> efi_status>;

#[repr(C)]
pub struct EfiGopProtocol {
    pub query_mode: EfiGopQueryMode,
    pub set_mode: EfiGopSetMode,
    pub blt: EfiGopBlt,
    pub mode: *mut EfiGopMode,
}

pub const EFI_GOP_PIXEL_RGBX_8BPP: u32 = 0;
pub const EFI_GOP_PIXEL_BGRX_8BPP: u32 = 1;
