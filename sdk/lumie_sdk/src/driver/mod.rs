use core::ffi::c_void;
use core::ptr;

use crate::api::{KernelApiV1, ModuleEntryFn};

pub struct DriverExport {
    pub exports: [usize; 4],
}

impl DriverExport {
    pub fn new() -> Self {
        DriverExport { exports: [0; 4] }
    }

    pub fn set(&mut self, index: usize, func: usize) {
        if index < 4 {
            self.exports[index] = func;
        }
    }

    pub fn as_ptr(&self) -> *mut c_void {
        &self.exports as *const [usize; 4] as *mut c_void
    }
}

pub trait DriverModule {
    fn init(&mut self, kapi: *const KernelApiV1) -> i32;
    fn exports(&self) -> &DriverExport;
}

pub unsafe fn driver_entry_init<T: DriverModule>(
    kapi: *const c_void,
    module_api: *mut *mut c_void,
    module: &mut T,
) -> i32 {
    let kapi = kapi as *const KernelApiV1;
    let ret = module.init(kapi);
    if ret != 0 {
        return ret;
    }
    if !module_api.is_null() {
        *module_api = module.exports().as_ptr();
    }
    0
}

pub struct AppContext {
    pub kapi: *const KernelApiV1,
    pub terminal: crate::term::Terminal,
    pub kbd: crate::term::Kbd,
    pub memory: crate::mem::Memory,
    pub filesystem: crate::fs::FileSystem,
    pub gpu: crate::gpu::Gpu,
    pub scheduler: crate::sched::Scheduler,
    pub system: crate::sys::System,
}

unsafe impl Send for AppContext {}
unsafe impl Sync for AppContext {}

impl AppContext {
    pub unsafe fn from_raw(kapi: *const c_void) -> Self {
        let kapi = kapi as *const KernelApiV1;
        AppContext {
            kapi,
            terminal: crate::term::Terminal::new(kapi),
            kbd: crate::term::Kbd::new(kapi),
            memory: crate::mem::Memory::new(kapi),
            filesystem: crate::fs::FileSystem::new(kapi),
            gpu: crate::gpu::Gpu::new(kapi),
            scheduler: crate::sched::Scheduler::new(kapi),
            system: crate::sys::System::new(kapi),
        }
    }

    pub fn term(&self) -> &crate::term::Terminal {
        &self.terminal
    }

    pub fn kbd(&self) -> &crate::term::Kbd {
        &self.kbd
    }

    pub fn mem(&self) -> &crate::mem::Memory {
        &self.memory
    }

    pub fn fs(&self) -> &crate::fs::FileSystem {
        &self.filesystem
    }

    pub fn gpu(&self) -> &crate::gpu::Gpu {
        &self.gpu
    }

    pub fn sched(&self) -> &crate::sched::Scheduler {
        &self.scheduler
    }

    pub fn sys(&self) -> &crate::sys::System {
        &self.system
    }
}

#[macro_export]
macro_rules! lumie_app_entry {
    ($name:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn entry(
            kapi: *const core::ffi::c_void,
            _module_api: *mut *mut core::ffi::c_void,
        ) -> i32 {
            let ctx = $crate::driver::AppContext::from_raw(kapi);
            $name(&ctx);
            0
        }
    };
}

#[macro_export]
macro_rules! lumie_driver_entry {
    ($name:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn entry(
            kapi: *const core::ffi::c_void,
            module_api: *mut *mut core::ffi::c_void,
        ) -> i32 {
            let mut driver = $name::new();
            $crate::driver::driver_entry_init(kapi, module_api, &mut driver)
        }
    };
}
