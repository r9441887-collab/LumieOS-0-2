
use crate::uefi::types::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiInputKey {
    pub scan_code: u16,
    pub unicode_char: char16,
}

pub type EfiInputReset = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, boolean)>;
pub type EfiInputReadKey = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *mut EfiInputKey) -> efi_status>;

#[repr(C)]
pub struct EfiSimpleTextInputProtocol {
    pub reset: EfiInputReset,
    pub read_key_stroke: EfiInputReadKey,
    pub wait_for_key: efi_event,
}
