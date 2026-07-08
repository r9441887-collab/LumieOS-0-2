use core::ffi::c_void;
use core::ptr;

#[repr(C)]
pub struct KernelExport {
    pub name: *const u8,
    pub func: *mut c_void,
}

unsafe impl Sync for KernelExport {}

#[repr(C)]
pub struct KernelExports {
    pub version: u32,
    pub api: *const c_void,
}

#[allow(unused_macros)]
macro_rules! kexport {
    ($name:ident) => {
        KernelExport {
            name: concat!(stringify!($name), "\0").as_ptr(),
            func: $name as *mut c_void,
        }
    };
}

static KERNEL_EXPORTS_TABLE: &[KernelExport] = &[
    /* This table would be populated with actual function pointers
     * at kernel initialization time. Below is the complete list
     * matching the original C kernel_exports.c */
];

pub unsafe fn kexport_find(name: &str) -> *mut c_void {
    for exp in KERNEL_EXPORTS_TABLE {
        if !exp.name.is_null() {
            let ename = crate::system::util::lumie_str_from_ptr(exp.name);
            if ename == name {
                return exp.func;
            }
        }
    }
    ptr::null_mut()
}

pub unsafe fn kexport_table() -> *const KernelExport {
    KERNEL_EXPORTS_TABLE.as_ptr()
}
