#![no_std]
mod ps2;
mod packet;
mod init;
use core::ffi::c_void;
static mut EXPORTS: [usize; 4] = [0; 4];
#[no_mangle]
pub unsafe extern "C" fn entry(_kapi: *const c_void, module_api: *mut *mut c_void) -> i32 {
    let ret = init::mouse_init();
    if ret != 0 { return ret; }
    EXPORTS[0] = packet::poll as *const () as usize;
    EXPORTS[1] = packet::get_pos as *const () as usize;
    EXPORTS[2] = init::is_ready as *const () as usize;
    EXPORTS[3] = 0;
    if !module_api.is_null() {
        *module_api = &mut EXPORTS as *mut [usize; 4] as *mut c_void;
    }
    0
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
