use core::ffi::c_void;
use core::mem;
use core::ptr;
use crate::fs;

pub const SYS_MAGIC: u32 = 0x01535953;

#[repr(C)]
pub struct SysHeader {
    pub magic: u32,
    pub entry: u32,
    pub text_size: u32,
    pub bss_size: u32,
    pub reloc_off: u32,
    pub reloc_count: u32,
    pub exports_off: u32,
    pub exports_count: u32,
    pub name: [u8; 32],
}

#[repr(C)]
pub struct SysBootInfo {
    pub version: u32,
    pub alloc: Option<unsafe fn(u32) -> *mut c_void>,
    pub free: Option<unsafe fn(*mut c_void)>,
    pub log: Option<unsafe fn(*const u8)>,
    pub log_hex: Option<unsafe fn(u64)>,
    pub gop_fb_base: u64,
    pub gop_width: u32,
    pub gop_height: u32,
    pub gop_pitch: u32,
}

pub type SysEntryFn = Option<unsafe fn(*const SysBootInfo, *mut *mut c_void) -> i32>;

#[repr(C)]
pub struct SysModule {
    pub base: *mut c_void,
    pub size: u32,
    pub entry: SysEntryFn,
}

pub unsafe fn sys_load(path: *const u8, _boot_info: *mut c_void, mod_out: *mut c_void) -> i32 {
    if path.is_null() || mod_out.is_null() {
        return -1;
    }
    let mod_ref = &mut *(mod_out as *mut SysModule);
    ptr::write_bytes(mod_ref, 0, 1);

    let path_str = crate::system::util::lumie_str_from_ptr(path);
    let path_ptr = path_str.as_ptr() as *const u8;
    let fsz = fs::get_file_size(path_ptr);
    if fsz < mem::size_of::<SysHeader>() as i32 {
        return -2;
    }

    let buf = crate::mm::alloc(fsz as u64);
    if buf.is_null() {
        return -3;
    }
    let ret = fs::read_file(path_ptr, buf, fsz as u32);
    if ret < mem::size_of::<SysHeader>() as i32 {
        crate::mm::free(buf);
        return -4;
    }
    let hdr = &*(buf as *const SysHeader);
    if hdr.magic != SYS_MAGIC {
        crate::mm::free(buf);
        return -5;
    }
    let mod_size = hdr.text_size + hdr.bss_size;
    if mod_size == 0 {
        crate::mm::free(buf);
        return -6;
    }
    let mod_base = crate::mm::alloc(mod_size as u64);
    if mod_base.is_null() {
        crate::mm::free(buf);
        return -7;
    }
    ptr::write_bytes(mod_base, 0, mod_size as usize);
    ptr::copy_nonoverlapping(
        buf.add(mem::size_of::<SysHeader>()),
        mod_base,
        hdr.text_size as usize,
    );

    if hdr.reloc_off != 0 && hdr.reloc_count != 0 {
        let load_base = mod_base as u64;
        let relocs = buf.add(hdr.reloc_off as usize) as *const u32;
        for i in 0..hdr.reloc_count {
            let off = *relocs.add(i as usize);
            if (off + 8) <= mod_size {
                let ptr = mod_base.add(off as usize) as *mut u64;
                *ptr = (*ptr).wrapping_add(load_base);
            }
        }
    }

    mod_ref.base = mod_base as *mut c_void;
    mod_ref.size = mod_size;
    mod_ref.entry = Some(mem::transmute(mod_base.add(hdr.entry as usize)));

    crate::mm::free(buf);
    0
}

pub unsafe fn sys_free(mod_: *mut SysModule) {
    if !mod_.is_null() {
        let m = &mut *mod_;
        if !m.base.is_null() {
            crate::mm::free(m.base as *mut u8);
            m.base = ptr::null_mut();
            m.size = 0;
            m.entry = None;
        }
    }
}
