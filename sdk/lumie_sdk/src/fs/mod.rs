use core::ffi::c_void;

use crate::api::{KernelApiV1, LumieDirEnt};

pub struct FileSystem {
    kapi: *const KernelApiV1,
}

unsafe impl Send for FileSystem {}
unsafe impl Sync for FileSystem {}

impl FileSystem {
    pub unsafe fn new(kapi: *const KernelApiV1) -> Self {
        FileSystem { kapi }
    }

    pub fn read(&self, path: &str, buf: &mut [u8]) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).fs_read {
                return f(path.as_ptr(), buf.as_mut_ptr() as *mut c_void, buf.len() as u32);
            }
            -1
        }
    }

    pub fn write(&self, path: &str, data: &[u8]) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).fs_write {
                return f(path.as_ptr(), data.as_ptr() as *const c_void, data.len() as u32);
            }
            -1
        }
    }

    pub fn exists(&self, path: &str) -> bool {
        unsafe {
            if let Some(f) = (*self.kapi).fs_exists {
                return f(path.as_ptr()) != 0;
            }
            false
        }
    }

    pub fn list_dir(&self, path: &str, entries: &mut [LumieDirEnt]) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).fs_list {
                return f(
                    path.as_ptr(),
                    entries.as_mut_ptr() as *mut c_void,
                    entries.len() as i32,
                );
            }
            -1
        }
    }

    pub fn mkdir(&self, path: &str) -> i32 {
        unsafe {
            if let Some(f) = (*self.kapi).fs_mkdir {
                return f(path.as_ptr());
            }
            -1
        }
    }

    pub fn read_to_string(&self, path: &str, max_size: usize) -> (i32, [u8; 4096]) {
        let mut buf = [0u8; 4096];
        let read_size = core::cmp::min(max_size, 4096);
        let ret = self.read(path, &mut buf[..read_size]);
        (ret, buf)
    }

    pub fn write_str(&self, path: &str, s: &str) -> i32 {
        self.write(path, s.as_bytes())
    }

    pub fn read_file_size(&self, path: &str) -> i32 {
        let mut buf = [0u8; 1];
        self.read(path, &mut buf)
    }

    pub fn list_dir_count(&self, path: &str, entries: &mut [LumieDirEnt]) -> i32 {
        self.list_dir(path, entries)
    }
}
