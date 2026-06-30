#![no_std]

extern "C" {
    fn xhci_init();
    fn xhci_mouse_present() -> i32;
    fn xhci_poll_mouse(dx: *mut i32, dy: *mut i32, btns: *mut u8) -> i32;
}

pub unsafe fn init() {
    xhci_init();
}

pub fn mouse_present() -> i32 {
    unsafe { xhci_mouse_present() }
}

pub unsafe fn poll_mouse(dx: *mut i32, dy: *mut i32, btns: *mut u8) -> i32 {
    xhci_poll_mouse(dx, dy, btns)
}
