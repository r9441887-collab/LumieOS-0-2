
pub unsafe fn init() {
    crate::ffi::init_xhci();
}

pub fn mouse_present() -> i32 {
    crate::ffi::xhci_mouse_available()
}

pub unsafe fn poll_mouse(dx: *mut i32, dy: *mut i32, btns: *mut u8) -> i32 {
    crate::ffi::poll_xhci_mouse(dx, dy, btns)
}
