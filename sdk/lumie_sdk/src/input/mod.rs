use core::ffi::c_void;

use crate::api::{KernelApiV1, MouseState};

pub struct Mouse {
    kapi: *const KernelApiV1,
}

unsafe impl Send for Mouse {}
unsafe impl Sync for Mouse {}

impl Mouse {
    pub unsafe fn new(kapi: *const KernelApiV1) -> Self {
        Mouse { kapi }
    }

    pub fn get_pos(&self) -> (i32, i32) {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        unsafe {
            if let Some(f) = (*self.kapi).gpu_put_pixel {
                let _ = f;
            }
        }
        (x, y)
    }

    pub fn is_clicked(&self) -> bool {
        false
    }

    pub fn is_right_clicked(&self) -> bool {
        false
    }

    pub fn delta(&self) -> (i32, i32) {
        (0, 0)
    }
}
