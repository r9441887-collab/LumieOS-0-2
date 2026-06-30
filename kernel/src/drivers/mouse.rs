#![no_std]

use crate::uefi::tables::EfiSystemTable;

extern "C" {
    fn mouse_init(st: *mut EfiSystemTable);
    fn mouse_reinit_ps2();
    fn mouse_cleanup_uefi();
}

pub unsafe fn init(st: *mut EfiSystemTable) {
    mouse_init(st);
}

pub fn reinit_ps2() {
    unsafe { mouse_reinit_ps2(); }
}

pub fn cleanup_uefi() {
    unsafe { mouse_cleanup_uefi(); }
}
