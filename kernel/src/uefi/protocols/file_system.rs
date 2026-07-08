
use crate::uefi::types::*;
use crate::uefi::guid::EfiGuid;

pub type EfiFileOpen = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol, *mut *mut EfiFileProtocol, *mut char16, u64, u64) -> efi_status>;
pub type EfiFileClose = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol) -> efi_status>;
pub type EfiFileDelete = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol) -> efi_status>;
pub type EfiFileRead = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol, *mut u64, *mut core::ffi::c_void) -> efi_status>;
pub type EfiFileWrite = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol, *mut u64, *mut core::ffi::c_void) -> efi_status>;
pub type EfiFileGetPosition = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol, *mut u64) -> efi_status>;
pub type EfiFileSetPosition = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol, u64) -> efi_status>;
pub type EfiFileGetInfo = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol, *const EfiGuid, *mut core::ffi::c_void, *mut u64) -> efi_status>;
pub type EfiFileSetInfo = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol, *const EfiGuid, *mut core::ffi::c_void, u64) -> efi_status>;
pub type EfiFileFlush = Option<unsafe extern "efiapi" fn(*mut EfiFileProtocol) -> efi_status>;

#[repr(C)]
pub struct EfiFileProtocol {
    pub revision: u64,
    pub open: EfiFileOpen,
    pub close: EfiFileClose,
    pub delete: EfiFileDelete,
    pub read: EfiFileRead,
    pub write: EfiFileWrite,
    pub get_position: EfiFileGetPosition,
    pub set_position: EfiFileSetPosition,
    pub get_info: EfiFileGetInfo,
    pub set_info: EfiFileSetInfo,
    pub flush: EfiFileFlush,
}

pub type EfiOpenVolume = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> efi_status>;

#[repr(C)]
pub struct EfiSimpleFileSystemProtocol {
    pub revision: u64,
    pub open_volume: EfiOpenVolume,
}

pub const EFI_FILE_MODE_READ: u64 = 1;
pub const EFI_FILE_MODE_WRITE: u64 = 2;
pub const EFI_FILE_MODE_CREATE: u64 = 0x8000000000000000;
