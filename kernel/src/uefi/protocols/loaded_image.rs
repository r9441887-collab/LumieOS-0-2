#![no_std]

use crate::uefi::types::*;
use crate::uefi::tables::EfiSystemTable;

#[repr(C)]
pub struct EfiLoadedImageProtocol {
    pub revision: u32,
    pub parent_handle: efi_handle,
    pub system_table: *mut EfiSystemTable,
    pub device_handle: efi_handle,
    pub file_path: *mut core::ffi::c_void,
    pub reserved: *mut core::ffi::c_void,
    pub load_options_size: u32,
    pub load_options: *mut core::ffi::c_void,
    pub image_base: *mut core::ffi::c_void,
    pub image_size: u64,
    pub image_code_type: u64,
    pub image_data_type: u64,
    pub unload: Option<unsafe extern "efiapi" fn(efi_handle) -> efi_status>,
}
