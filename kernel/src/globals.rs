#![no_std]

use core::cell::UnsafeCell;
use crate::uefi::types::*;
use crate::uefi::tables::*;

#[repr(C)]
pub struct ModuleT {
    pub hdr: *const core::ffi::c_void,
    pub base: *mut core::ffi::c_void,
    pub size: u32,
    pub module_api: *mut core::ffi::c_void,
    pub loaded: i32,
}

static G_ST: UnsafeCell<Option<&'static EfiSystemTable>> = UnsafeCell::new(None);
static G_IMAGE_HANDLE: UnsafeCell<Option<efi_handle>> = UnsafeCell::new(None);
static G_BS: UnsafeCell<Option<&'static EfiBootServices>> = UnsafeCell::new(None);
static G_RT: UnsafeCell<Option<&'static EfiRuntimeServices>> = UnsafeCell::new(None);
static G_KERNEL_BASE: UnsafeCell<*const core::ffi::c_void> = UnsafeCell::new(core::ptr::null());
static G_KERNEL_SIZE: UnsafeCell<u32> = UnsafeCell::new(0);
static G_SHELL_MOD_LOADED: UnsafeCell<u8> = UnsafeCell::new(0);
static G_SHELL_MOD: UnsafeCell<ModuleT> = UnsafeCell::new(ModuleT {
    hdr: core::ptr::null(),
    base: core::ptr::null_mut(),
    size: 0,
    module_api: core::ptr::null_mut(),
    loaded: 0,
});

pub unsafe fn set_st(st: &'static EfiSystemTable) {
    *G_ST.get() = Some(st);
}

pub unsafe fn set_image_handle(handle: efi_handle) {
    *G_IMAGE_HANDLE.get() = Some(handle);
}

pub unsafe fn set_bs(bs: &'static EfiBootServices) {
    *G_BS.get() = Some(bs);
}

pub unsafe fn clear_bs() {
    *G_BS.get() = None;
}

pub unsafe fn set_rt(rt: &'static EfiRuntimeServices) {
    *G_RT.get() = Some(rt);
}

pub unsafe fn set_kernel_image(base: *const core::ffi::c_void, size: u32) {
    *G_KERNEL_BASE.get() = base;
    *G_KERNEL_SIZE.get() = size;
}

pub unsafe fn set_shell_mod_loaded(loaded: bool) {
    *G_SHELL_MOD_LOADED.get() = if loaded { 1 } else { 0 };
}

pub unsafe fn set_shell_module(mod_: ModuleT) {
    *G_SHELL_MOD.get() = mod_;
}

pub fn get_st() -> Option<&'static EfiSystemTable> {
    unsafe { *G_ST.get() }
}

pub fn get_image_handle() -> Option<efi_handle> {
    unsafe { *G_IMAGE_HANDLE.get() }
}

pub fn get_bs() -> Option<&'static EfiBootServices> {
    unsafe { *G_BS.get() }
}

pub fn get_rt() -> Option<&'static EfiRuntimeServices> {
    unsafe { *G_RT.get() }
}

pub fn get_kernel_image() -> (*const core::ffi::c_void, u32) {
    unsafe { (*G_KERNEL_BASE.get(), *G_KERNEL_SIZE.get()) }
}

pub fn get_shell_mod_loaded() -> bool {
    unsafe { *G_SHELL_MOD_LOADED.get() != 0 }
}

pub fn get_shell_module() -> Option<ModuleT> {
    unsafe {
        let m = *G_SHELL_MOD.get();
        if m.loaded != 0 { Some(m) } else { None }
    }
}
