#![no_std]

use crate::uefi::types::*;

pub const EFI_VARIABLE_NON_VOLATILE: u32 = 0x00000001;
pub const EFI_VARIABLE_BOOTSERVICE_ACCESS: u32 = 0x00000002;
pub const EFI_VARIABLE_RUNTIME_ACCESS: u32 = 0x00000004;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiLoadOption {
    pub attributes: u32,
    pub file_path_list_length: u16,
}

pub const LOAD_OPTION_ACTIVE: u32 = 0x00000001;
pub const LOAD_OPTION_CATEGORY_APP: u32 = 0x00000800;
