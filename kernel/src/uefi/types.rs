#![allow(non_camel_case_types)]
pub type uintn = core::primitive::u64;
pub type s8 = core::primitive::i8;
pub type s16 = core::primitive::i16;
pub type efi_status = core::primitive::u64;
pub type efi_handle = *mut core::ffi::c_void;
pub type EfiHandle = *mut core::ffi::c_void;
pub type efi_event = *mut core::ffi::c_void;
pub type char16 = core::primitive::u16;
pub type boolean = core::primitive::u8;

pub const NULL: *mut core::ffi::c_void = core::ptr::null_mut();
pub const TRUE: boolean = 1;
pub const FALSE: boolean = 0;

pub const EFI_SUCCESS: efi_status = 0;

#[inline]
pub const fn efi_error(s: efi_status) -> bool {
    (s as i64) < 0
}

#[inline]
pub const fn efi_err(x: efi_status) -> efi_status {
    x | (1u64 << 63)
}

pub const EFI_INVALID_PARAMETER: efi_status = efi_err(2);
pub const EFI_NOT_FOUND: efi_status = efi_err(14);

#[inline]
pub const fn bool_from_boolean(b: boolean) -> bool {
    b != 0
}

#[inline]
pub const fn boolean_from_bool(b: bool) -> boolean {
    b as boolean
}

#[inline]
pub const fn signature_16(a: u8, b: u8) -> u16 {
    (a as u16) | ((b as u16) << 8)
}

#[inline]
pub const fn signature_32(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (signature_16(a, b) as u32) | ((signature_16(c, d) as u32) << 16)
}

#[inline]
pub const fn signature_64(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) -> u64 {
    (signature_32(a, b, c, d) as u64) | ((signature_32(e, f, g, h) as u64) << 32)
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct EfiStatus(pub efi_status);

impl EfiStatus {
    pub const fn new(val: efi_status) -> Self {
        Self(val)
    }

    pub const fn is_error(self) -> bool {
        (self.0 as i64) < 0
    }

    pub const fn is_success(self) -> bool {
        self.0 == 0
    }

    pub const fn value(self) -> efi_status {
        self.0
    }
}
