
pub type FatReadFn = unsafe fn(lba: u32, count: u32, buffer: *mut u8) -> i32;
pub type FatWriteFn = unsafe fn(lba: u32, count: u32, buffer: *const u8) -> i32;
pub type AllocFn = unsafe fn(size: usize) -> *mut u8;
pub type FreeFn = unsafe fn(ptr: *mut u8, size: usize);
pub type TimeFn = unsafe fn() -> u32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiskIo {
    pub read_cb: Option<FatReadFn>,
    pub write_cb: Option<FatWriteFn>,
    pub alloc_cb: Option<AllocFn>,
    pub free_cb: Option<FreeFn>,
    pub time_cb: Option<TimeFn>,
}
