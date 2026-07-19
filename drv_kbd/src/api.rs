#![no_std]
use core::ffi::c_void;

type PrintfFn = unsafe fn(*const u8);

pub struct Kapi {
    pub printf: Option<PrintfFn>,
}

impl Kapi {
    pub fn from_raw(kapi: *const c_void) -> Self {
        if kapi.is_null() {
            return Kapi { printf: None };
        }
        unsafe {
            let vtable = kapi as *const usize;
            let printf_ptr = vtable.add(31);
            Kapi {
                printf: core::mem::transmute(*printf_ptr),
            }
        }
    }

    pub unsafe fn log(&self, msg: &[u8]) {
        if let Some(printf) = self.printf {
            let mut buf: [u8; 256] = [0u8; 256];
            let len = msg.len().min(254);
            buf[..len].copy_from_slice(&msg[..len]);
            buf[len] = 0;
            printf(buf.as_ptr());
        }
    }
}
