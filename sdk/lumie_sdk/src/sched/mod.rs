use core::ffi::c_void;

use crate::api::KernelApiV1;

pub struct Scheduler {
    kapi: *const KernelApiV1,
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}

impl Scheduler {
    pub unsafe fn new(kapi: *const KernelApiV1) -> Self {
        Scheduler { kapi }
    }

    pub fn count(&self) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).sched_count {
                return f();
            }
            0
        }
    }

    pub fn name(&self, id: i32) -> *const u8 {
        unsafe {
            if let Some(f) = (*self.kapi).sched_name {
                return f(id);
            }
            core::ptr::null()
        }
    }

    pub fn state(&self, id: i32) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).sched_state {
                return f(id);
            }
            -1
        }
    }
}
