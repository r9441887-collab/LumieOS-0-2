use core::ffi::c_void;

use crate::api::KernelApiV1;

pub struct System {
    kapi: *const KernelApiV1,
}

unsafe impl Send for System {}
unsafe impl Sync for System {}

impl System {
    pub unsafe fn new(kapi: *const KernelApiV1) -> Self {
        System { kapi }
    }

    pub fn stall(&self, us: u64) {
        unsafe {
            if let Some(f) = (*self.kapi).stall {
                f(us);
            }
        }
    }

    pub fn sleep_ms(&self, ms: u64) {
        self.stall(ms * 1000);
    }

    pub fn reboot(&self) {
        unsafe {
            if let Some(f) = (*self.kapi).reboot {
                f();
            }
        }
        loop {}
    }

    pub fn shutdown(&self) {
        unsafe {
            if let Some(f) = (*self.kapi).shutdown {
                f();
            }
        }
        loop {}
    }

    pub fn get_time(&self, buf: &mut [u8]) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).get_time {
                return f(buf.as_mut_ptr(), buf.len() as i32);
            }
            -1
        }
    }

    pub fn pci_scan(&self, index: i32) -> (u16, u16, u8) {
        unsafe {
            let mut vendor: u16 = 0;
            let mut device: u16 = 0;
            let mut class: u8 = 0;
            if let Some(f) = (*self.kapi).pci_scan {
                f(index, &mut vendor, &mut device, &mut class);
            }
            (vendor, device, class)
        }
    }

    pub fn pci_vendor_str(&self, vendor: u16) -> *const u8 {
        unsafe {
            if let Some(f) = (*self.kapi).pci_vendor_str {
                return f(vendor);
            }
            core::ptr::null()
        }
    }

    pub fn pci_device_str(&self, vendor: u16, device: u16) -> *const u8 {
        unsafe {
            if let Some(f) = (*self.kapi).pci_device_str {
                return f(vendor, device);
            }
            core::ptr::null()
        }
    }

    pub fn disk_count(&self) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).disk_count {
                return f();
            }
            0
        }
    }

    pub fn disk_name(&self, id: i32) -> *const u8 {
        unsafe {
            if let Some(f) = (*self.kapi).disk_name {
                return f(id);
            }
            core::ptr::null()
        }
    }

    pub fn disk_sectors(&self, id: i32) -> u64 {
        unsafe {
            if let Some(f) = (*self.kapi).disk_sectors {
                return f(id);
            }
            0
        }
    }

    pub fn disk_read(&self, id: i32, sector: u64, count: u32, buf: &mut [u8]) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).disk_read {
                return f(id, sector, count, buf.as_mut_ptr() as *mut c_void);
            }
            -1
        }
    }

    pub fn disk_write(&self, id: i32, sector: u64, count: u32, data: &[u8]) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).disk_write {
                return f(id, sector, count, data.as_ptr() as *const c_void);
            }
            -1
        }
    }

    pub fn mod_load(&self, path: &str) -> i32 {
        unsafe {
            let mut api: *mut c_void = core::ptr::null_mut();
            if let Some(f) = (*self.kapi).mod_load {
                return f(path.as_ptr(), &mut api);
            }
            -1
        }
    }
}
