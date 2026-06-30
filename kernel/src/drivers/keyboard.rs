#![no_std]

use crate::uefi::tables::EfiSystemTable;
use crate::uefi::types::efi_handle;

extern "C" {
    fn kbd_init(st: *mut EfiSystemTable);
    fn kbd_switch_to_ps2();
    fn kbd_getchar() -> i32;
    fn kbd_kbhit() -> i32;
}

pub unsafe fn init(st: *mut EfiSystemTable) {
    kbd_init(st);
}

pub fn switch_to_ps2() {
    unsafe { kbd_switch_to_ps2(); }
}

pub fn getchar() -> i32 {
    unsafe { kbd_getchar() }
}

pub fn kbhit() -> i32 {
    unsafe { kbd_kbhit() }
}
