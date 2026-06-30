#![no_std]

use core::ffi::c_void;

extern "C" {
    fn ps2mouse_init();
    fn ps2mouse_is_ready() -> i32;
    fn ps2mouse_poll(dx: *mut i32, dy: *mut i32, btns: *mut u8) -> i32;
}

pub unsafe fn init() {
    ps2mouse_init();
}

pub fn is_ready() -> i32 {
    unsafe { ps2mouse_is_ready() }
}

pub unsafe fn poll(dx: *mut i32, dy: *mut i32, btns: *mut u8) -> i32 {
    ps2mouse_poll(dx, dy, btns)
}
