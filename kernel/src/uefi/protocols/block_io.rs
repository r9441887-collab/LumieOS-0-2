
use crate::uefi::types::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiBlockIoMedia {
    pub media_id: u32,
    pub removable_media: u8,
    pub media_present: u8,
    pub logical_partition: u8,
    pub read_only: u8,
    pub write_caching: u8,
    pub pad: [u8; 3],
    pub block_size: u64,
    pub last_block: u64,
    pub lowest_aligned_lba: u64,
    pub logical_blocks_per_physical_block: u32,
    pub optimal_transfer_length_granularity: u32,
}

#[repr(C)]
pub struct EfiBlockIoProtocol {
    pub revision: u64,
    pub media: *mut EfiBlockIoMedia,
    pub reset: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u8) -> efi_status>,
    pub read_blocks: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u32, u64, u64, *mut core::ffi::c_void) -> efi_status>,
    pub write_blocks: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u32, u64, u64, *mut core::ffi::c_void) -> efi_status>,
    pub flush_blocks: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void) -> efi_status>,
}
