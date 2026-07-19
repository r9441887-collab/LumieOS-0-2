#![no_std]
mod vfat;
use core::ffi::c_void;
static mut EXPORTS: [usize; 4] = [0; 4];
#[no_mangle]
pub unsafe extern "C" fn entry(kapi: *const c_void, module_api: *mut *mut c_void) -> i32 {
    if kapi.is_null() { return -1; }
    let vtable = kapi as *const usize;
    let read_fn = core::mem::transmute::<usize, unsafe fn(*const u8, *mut c_void, u32) -> i32>(*vtable.add(8));
    let write_fn = core::mem::transmute::<usize, unsafe fn(*const u8, *const c_void, u32) -> i32>(*vtable.add(9));
    let exists_fn = core::mem::transmute::<usize, unsafe fn(*const u8) -> i32>(*vtable.add(10));
    let mkdir_fn = core::mem::transmute::<usize, unsafe fn(*const u8) -> i32>(*vtable.add(11));
    vfat::set_ops(vfat::FsOps { read: read_fn, write: write_fn, exists: exists_fn, mkdir: mkdir_fn });
    EXPORTS[0] = vfat::read as *const () as usize;
    EXPORTS[1] = vfat::write as *const () as usize;
    EXPORTS[2] = vfat::exists as *const () as usize;
    EXPORTS[3] = 0;
    if !module_api.is_null() {
        *module_api = &mut EXPORTS as *mut [usize; 4] as *mut c_void;
    }
    0
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
