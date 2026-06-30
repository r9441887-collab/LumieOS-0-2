#![no_std]

extern "C" {
    fn pcspkr_init();
}

pub unsafe fn init() {
    pcspkr_init();
}
