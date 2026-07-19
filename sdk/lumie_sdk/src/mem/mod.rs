use core::ffi::c_void;
use core::ptr;

use crate::api::KernelApiV1;

pub struct Memory {
    kapi: *const KernelApiV1,
}

unsafe impl Send for Memory {}
unsafe impl Sync for Memory {}

impl Memory {
    pub unsafe fn new(kapi: *const KernelApiV1) -> Self {
        Memory { kapi }
    }

    pub fn alloc(&self, size: u64) -> *mut c_void {
        unsafe {
            if let Some(f) = (*self.kapi).kmalloc {
                return f(size);
            }
            ptr::null_mut()
        }
    }

    pub fn free(&self, ptr: *mut c_void) {
        unsafe {
            if let Some(f) = (*self.kapi).kfree {
                f(ptr);
            }
        }
    }

    pub fn calloc(&self, count: u64, size: u64) -> *mut c_void {
        unsafe {
            if let Some(f) = (*self.kapi).kcalloc {
                return f(count, size);
            }
            ptr::null_mut()
        }
    }

    pub fn memset(&self, dst: *mut c_void, val: i32, size: u64) {
        unsafe {
            if let Some(f) = (*self.kapi).kmemset {
                f(dst, val, size);
            }
        }
    }

    pub fn memcpy(&self, dst: *mut c_void, src: *const c_void, size: u64) {
        unsafe {
            if let Some(f) = (*self.kapi).kmemcpy {
                f(dst, src, size);
            }
        }
    }

    pub fn total(&self) -> u64 {
        unsafe {
            if let Some(f) = (*self.kapi).mem_total {
                return f();
            }
            0
        }
    }

    pub fn free_mem(&self) -> u64 {
        unsafe {
            if let Some(f) = (*self.kapi).mem_free {
                return f();
            }
            0
        }
    }

    pub fn used(&self) -> u64 {
        unsafe {
            if let Some(f) = (*self.kapi).mem_used {
                return f();
            }
            0
        }
    }

    pub fn alloc_array<T>(&self, count: usize) -> *mut T {
        let size = (core::mem::size_of::<T>() * count) as u64;
        self.alloc(size) as *mut T
    }

    pub fn free_slice<T>(&self, ptr: *mut T, _count: usize) {
        self.free(ptr as *mut c_void);
    }
}
