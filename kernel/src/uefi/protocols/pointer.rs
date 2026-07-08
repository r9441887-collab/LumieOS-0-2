
use crate::uefi::types::*;
use crate::uefi::protocols::input::EfiInputReset;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiSimplePointerMode {
    pub input_report_wait_timeout: u64,
    pub sample_count: u64,
    pub maximum_positive_x: u32,
    pub maximum_positive_y: u32,
    pub maximum_positive_z: u32,
    pub minimum_negative_x: u32,
    pub minimum_negative_y: u32,
    pub minimum_negative_z: u32,
}

pub type EfiPointerGetState = Option<unsafe extern "efiapi" fn(*mut EfiSimplePointerProtocol, *mut EfiSimplePointerState) -> efi_status>;

#[repr(C)]
pub struct EfiSimplePointerProtocol {
    pub reset: EfiInputReset,
    pub get_state: EfiPointerGetState,
    pub wait_for_input: efi_event,
    pub mode: *mut EfiSimplePointerMode,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiSimplePointerState {
    pub relative_movement_x: i64,
    pub relative_movement_y: i64,
    pub relative_movement_z: i64,
    pub attributes: u32,
    pub buttons: u32,
}

pub const EFI_SIMPLE_POINTER_LEFT_BUTTON: u32 = 0x01;
pub const EFI_SIMPLE_POINTER_RIGHT_BUTTON: u32 = 0x02;
pub const EFI_SIMPLE_POINTER_MIDDLE_BUTTON: u32 = 0x04;

pub type EfiAbsPointerGetState = Option<unsafe extern "efiapi" fn(*mut EfiAbsolutePointerProtocol, *mut core::ffi::c_void) -> efi_status>;

#[repr(C)]
pub struct EfiAbsolutePointerProtocol {
    pub reset: EfiInputReset,
    pub get_state: EfiAbsPointerGetState,
    pub wait_for_input: efi_event,
    pub mode: *mut core::ffi::c_void,
}
