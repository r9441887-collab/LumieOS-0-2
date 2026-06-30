#![no_std]

use crate::uefi::types::*;
use crate::uefi::protocols::input::EfiInputReset;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiInputKeyEx {
    pub scan_code: u16,
    pub unicode_char: char16,
    pub shift_state: u32,
}

pub type EfiInputExReadKey = Option<unsafe extern "efiapi" fn(*mut EfiSimpleTextInputExProtocol, *mut EfiInputKeyEx) -> efi_status>;
pub type EfiInputExSetState = Option<unsafe extern "efiapi" fn(*mut EfiSimpleTextInputExProtocol, *mut core::ffi::c_void) -> efi_status>;
pub type EfiInputExRegisterKey = Option<unsafe extern "efiapi" fn(*mut EfiSimpleTextInputExProtocol, *mut core::ffi::c_void, *mut core::ffi::c_void, *mut core::ffi::c_void) -> efi_status>;
pub type EfiInputExUnregisterKey = Option<unsafe extern "efiapi" fn(*mut EfiSimpleTextInputExProtocol, *mut core::ffi::c_void) -> efi_status>;

#[repr(C)]
pub struct EfiSimpleTextInputExProtocol {
    pub reset: EfiInputReset,
    pub read_key_stroke_ex: EfiInputExReadKey,
    pub wait_for_key_ex: efi_event,
    pub set_state: EfiInputExSetState,
    pub register_key_notify: EfiInputExRegisterKey,
    pub unregister_key_notify: EfiInputExUnregisterKey,
}

pub const EFI_SHIFT_STATE_VALID: u32 = 0x80000000;
pub const EFI_LEFT_SHIFT_PRESSED: u32 = 0x00000001;
pub const EFI_RIGHT_SHIFT_PRESSED: u32 = 0x00000002;
pub const EFI_LEFT_CONTROL_PRESSED: u32 = 0x00000004;
pub const EFI_RIGHT_CONTROL_PRESSED: u32 = 0x00000008;
pub const EFI_LEFT_ALT_PRESSED: u32 = 0x00000010;
pub const EFI_RIGHT_ALT_PRESSED: u32 = 0x00000020;
pub const EFI_LEFT_LOGO_PRESSED: u32 = 0x00000040;
pub const EFI_RIGHT_LOGO_PRESSED: u32 = 0x00000080;
