
use crate::uefi::types::*;

pub type EfiOutReset = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, boolean)>;
pub type EfiOutOutputString = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *mut char16) -> efi_status>;
pub type EfiOutSetAttribute = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u64) -> efi_status>;
pub type EfiOutClearScreen = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void) -> efi_status>;
pub type EfiOutSetCursorPosition = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u64, u64) -> efi_status>;
pub type EfiOutEnableCursor = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, boolean) -> efi_status>;

#[repr(C)]
pub struct EfiSimpleTextOutputProtocol {
    pub reset: EfiOutReset,
    pub output_string: EfiOutOutputString,
    pub test_string: *mut core::ffi::c_void,
    pub set_attribute: EfiOutSetAttribute,
    pub clear_screen: EfiOutClearScreen,
    pub set_cursor_position: EfiOutSetCursorPosition,
    pub enable_cursor: EfiOutEnableCursor,
    pub mode: *mut core::ffi::c_void,
}

#[inline]
pub const fn efi_text_attr(fg: u64, bg: u64) -> u64 {
    fg | (bg << 4)
}

pub const EFI_BLACK: u64 = 0;
pub const EFI_BLUE: u64 = 1;
pub const EFI_GREEN: u64 = 2;
pub const EFI_CYAN: u64 = 3;
pub const EFI_RED: u64 = 4;
pub const EFI_MAGENTA: u64 = 5;
pub const EFI_BROWN: u64 = 6;
pub const EFI_LIGHTGRAY: u64 = 7;
pub const EFI_BRIGHT: u64 = 8;
