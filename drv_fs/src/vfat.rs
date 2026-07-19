use core::ffi::c_void;
type ReadFn = unsafe fn(*const u8, *mut c_void, u32) -> i32;
type WriteFn = unsafe fn(*const u8, *const c_void, u32) -> i32;
type ExistsFn = unsafe fn(*const u8) -> i32;
type MkdirFn = unsafe fn(*const u8) -> i32;
pub struct FsOps {
    pub read: ReadFn,
    pub write: WriteFn,
    pub exists: ExistsFn,
    pub mkdir: MkdirFn,
}
static mut OPS: Option<FsOps> = None;
pub fn set_ops(ops: FsOps) { unsafe { OPS = Some(ops); } }
pub fn get_ops() -> Option<&'static FsOps> { unsafe { OPS.as_ref() } }
pub fn read(path: &[u8], buf: &mut [u8]) -> i32 {
    unsafe {
        if let Some(ref ops) = OPS {
            let mut p = [0u8; 256];
            let n = path.len().min(254);
            p[..n].copy_from_slice(&path[..n]);
            p[n] = 0;
            (ops.read)(p.as_ptr(), buf.as_mut_ptr() as *mut c_void, buf.len() as u32)
        } else { -1 }
    }
}
pub fn write(path: &[u8], data: &[u8]) -> i32 {
    unsafe {
        if let Some(ref ops) = OPS {
            let mut p = [0u8; 256];
            let n = path.len().min(254);
            p[..n].copy_from_slice(&path[..n]);
            p[n] = 0;
            (ops.write)(p.as_ptr(), data.as_ptr() as *const c_void, data.len() as u32)
        } else { -1 }
    }
}
pub fn exists(path: &[u8]) -> bool {
    unsafe {
        if let Some(ref ops) = OPS {
            let mut p = [0u8; 256];
            let n = path.len().min(254);
            p[..n].copy_from_slice(&path[..n]);
            p[n] = 0;
            (ops.exists)(p.as_ptr()) == 1
        } else { false }
    }
}
