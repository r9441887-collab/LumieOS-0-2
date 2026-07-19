use core::ffi::c_void;

use crate::api::KernelApiV1;

pub struct Gpu {
    kapi: *const KernelApiV1,
}

unsafe impl Send for Gpu {}
unsafe impl Sync for Gpu {}

impl Gpu {
    pub unsafe fn new(kapi: *const KernelApiV1) -> Self {
        Gpu { kapi }
    }

    pub fn fill_rect(&self, x: u32, y: u32, w: u32, h: u32, color: u32) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).gpu_fill_rect {
                return f(x, y, w, h, color);
            }
            -1
        }
    }

    pub fn put_pixel(&self, x: u32, y: u32, color: u32) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).gpu_put_pixel {
                return f(x, y, color);
            }
            -1
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        unsafe {
            if let Some(f) = (*self.kapi).gpu_get_pixel {
                return f(x, y);
            }
            0
        }
    }

    pub fn is_active(&self) -> bool {
        unsafe {
            if let Some(f) = (*self.kapi).gpu_is_active {
                return f() != 0;
            }
            false
        }
    }

    pub fn flip(&self) {
        unsafe {
            if let Some(f) = (*self.kapi).gpu_flip {
                f();
            }
        }
    }

    pub fn vsync(&self) {
        unsafe {
            if let Some(f) = (*self.kapi).gpu_vsync {
                f();
            }
        }
    }

    pub fn draw_char(&self, x: u32, y: u32, fg: u32, bg: u32, c: u8) {
        self.fill_rect(x, y, 8, 16, bg);
        if c >= b' ' {
            self.put_pixel(x + 3, y + 3, fg);
        }
    }

    pub fn draw_rect_outline(&self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        self.fill_rect(x, y, w, 1, color);
        self.fill_rect(x, y + h - 1, w, 1, color);
        self.fill_rect(x, y, 1, h, color);
        self.fill_rect(x + w - 1, y, 1, h, color);
    }

    pub fn clear(&self, color: u32) {
        let w = 1024;
        let h = 768;
        self.fill_rect(0, 0, w, h, color);
    }

    pub fn width(&self) -> u32 {
        1024
    }

    pub fn height(&self) -> u32 {
        768
    }

    pub fn make_color(r: u8, g: u8, b: u8) -> u32 {
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}
