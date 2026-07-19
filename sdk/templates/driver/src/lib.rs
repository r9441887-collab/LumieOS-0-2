#![no_std]

extern crate lumie_sdk;

use core::ffi::c_void;
use lumie_sdk::{DriverExport, DriverModule, KernelApiV1};

pub struct MyDriver {
    exports: DriverExport,
    ready: bool,
}

impl MyDriver {
    pub fn new() -> Self {
        MyDriver {
            exports: DriverExport::new(),
            ready: false,
        }
    }
}

impl DriverModule for MyDriver {
    fn init(&mut self, _kapi: *const KernelApiV1) -> i32 {
        self.ready = true;
        0
    }

    fn exports(&self) -> &DriverExport {
        &self.exports
    }
}

#[no_mangle]
pub unsafe extern "C" fn entry(
    kapi: *const c_void,
    module_api: *mut *mut c_void,
) -> i32 {
    let mut driver = MyDriver::new();
    let kapi = kapi as *const KernelApiV1;
    let ret = driver.init(kapi);
    if ret != 0 {
        return ret;
    }
    if !module_api.is_null() {
        *module_api = driver.exports.as_ptr();
    }
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
