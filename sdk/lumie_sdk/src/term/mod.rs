use core::ffi::c_void;
use core::ptr;

use crate::api::KernelApiV1;

pub struct Terminal {
    kapi: *const KernelApiV1,
}

unsafe impl Send for Terminal {}
unsafe impl Sync for Terminal {}

impl Terminal {
    pub unsafe fn new(kapi: *const KernelApiV1) -> Self {
        Terminal { kapi }
    }

    pub fn clear(&self, bg: u32) {
        unsafe {
            if let Some(f) = (*self.kapi).term_clear {
                f(bg);
            }
        }
    }

    pub fn set_fg(&self, color: u32) {
        unsafe {
            if let Some(f) = (*self.kapi).term_set_fg {
                f(color);
            }
        }
    }

    pub fn set_bg(&self, color: u32) {
        unsafe {
            if let Some(f) = (*self.kapi).term_set_bg {
                f(color);
            }
        }
    }

    pub fn set_pos(&self, x: i32, y: i32) {
        unsafe {
            if let Some(f) = (*self.kapi).term_set_pos {
                f(x, y);
            }
        }
    }

    pub fn write(&self, s: &str) {
        unsafe {
            if let Some(f) = (*self.kapi).term_write {
                f(s.as_ptr());
            }
        }
    }

    pub fn writeln(&self, s: &str) {
        unsafe {
            if let Some(f) = (*self.kapi).term_writeln {
                f(s.as_ptr());
            }
        }
    }

    pub fn putchar(&self, c: u8) {
        unsafe {
            if let Some(f) = (*self.kapi).term_putchar {
                f(c);
            }
        }
    }

    pub fn width(&self) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).term_get_width {
                return f();
            }
            0
        }
    }

    pub fn height(&self) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).term_get_height {
                return f();
            }
            0
        }
    }

    pub fn printf(&self, s: &str) {
        unsafe {
            if let Some(f) = (*self.kapi).printf {
                f(s.as_ptr());
            }
        }
    }

    pub fn write_fmt(&self, args: &str) {
        self.write(args);
    }
}

pub struct Kbd {
    kapi: *const KernelApiV1,
}

unsafe impl Send for Kbd {}
unsafe impl Sync for Kbd {}

impl Kbd {
    pub unsafe fn new(kapi: *const KernelApiV1) -> Self {
        Kbd { kapi }
    }

    pub fn getchar(&self) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).kbd_getchar {
                return f();
            }
            -1
        }
    }

    pub fn kbhit(&self) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).kbd_kbhit {
                return f();
            }
            0
        }
    }

    pub fn flush(&self) {
        while self.kbhit() != 0 {
            self.getchar();
        }
    }

    pub fn read_key_blocking(&self) -> i32 {
        loop {
            let c = self.getchar();
            if c != -1 {
                return c;
            }
        }
    }
}
