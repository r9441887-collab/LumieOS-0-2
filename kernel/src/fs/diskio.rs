#![no_std]

use core::ffi::c_void;

use crate::uefi::types::*;
use crate::uefi::protocols::block_io::efi_block_io_protocol;

pub type FatReadFn = unsafe fn(lba: u32, count: u32, buffer: *mut u8) -> i32;
pub type FatWriteFn = unsafe fn(lba: u32, count: u32, buffer: *const u8) -> i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiskIo {
    pub read_cb: Option<FatReadFn>,
    pub write_cb: Option<FatWriteFn>,
    pub use_ahci: bool,
    pub block_io: Option<*mut efi_block_io_protocol>,
}

impl DiskIo {
    pub const fn new() -> Self {
        DiskIo {
            read_cb: None,
            write_cb: None,
            use_ahci: false,
            block_io: None,
        }
    }
}
