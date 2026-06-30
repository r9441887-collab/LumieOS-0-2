use core::ffi::c_void;
use core::mem;
use core::ptr;
use crate::globals;
use crate::fs;

pub const MODULE_MAGIC: u32 = 0x4D4F444C;
pub const MOD_MAGIC_SYS: u32 = 0x01535953;
pub const MOD_MAGIC_LKRN: u32 = 0x4E524B4C;
pub const MOD_MAGIC_LDRV: u32 = 0x5652444C;
pub const MOD_MAGIC_LSH: u32 = 0x48534C4C;

const REL_ADDR64: u16 = 1;
const REL_ADDR32: u16 = 2;
const REL_REL32: u16 = 4;

#[repr(C)]
pub struct ModuleHeader {
    pub magic: u32,
    pub entry: u32,
    pub text_size: u32,
    pub bss_size: u32,
    pub reloc_off: u32,
    pub reloc_count: u32,
    pub import_off: u32,
    pub import_count: u32,
    pub export_off: u32,
    pub export_count: u32,
    pub name: [u8; 32],
}

#[repr(C)]
pub struct LshHeader {
    pub magic: u32,
    pub entry: u32,
    pub code_size: u32,
    pub bss_size: u32,
    pub reloc_off: u32,
    pub reloc_count: u32,
    pub import_off: u32,
    pub import_count: u32,
    pub strtab_off: u32,
    pub strtab_size: u32,
    pub name: [u8; 24],
}

#[repr(C)]
pub struct LshImport {
    pub thunk_offset: u32,
    pub reloc_type: u16,
    pub name_offset: u16,
}

#[repr(C)]
pub struct ModuleImport {
    pub ordinal: u16,
    pub reserved: u16,
    pub thunk_offset: u32,
}

#[repr(C)]
pub struct ModuleT {
    pub hdr: *const c_void,
    pub base: *mut c_void,
    pub size: u32,
    pub module_api: *mut c_void,
    pub loaded: i32,
}

pub type ModuleEntryFn = unsafe extern "C" fn(*const c_void, *mut *mut c_void) -> i32;

unsafe fn lsh_load(data: *const u8, data_size: u32, kapi: *const c_void, mod_: &mut ModuleT) -> i32 {
    if data.is_null() || data_size < mem::size_of::<LshHeader>() as u32 {
        return -1;
    }
    let hdr = &*(data as *const LshHeader);
    if hdr.magic != MOD_MAGIC_LSH && hdr.magic != MOD_MAGIC_LDRV && hdr.magic != MOD_MAGIC_LKRN {
        return -2;
    }
    let code_sz = hdr.code_size;
    let bss_sz = hdr.bss_size;
    if code_sz == 0 {
        return -3;
    }
    let mod_size = code_sz + bss_sz;
    let mod_base = crate::mm::alloc(mod_size as u64);
    if mod_base.is_null() {
        return -4;
    }
    ptr::write_bytes(mod_base, 0, mod_size as usize);
    let load_base = mod_base as u64;

    ptr::copy_nonoverlapping(data.add(mem::size_of::<LshHeader>()), mod_base, code_sz as usize);

    if hdr.reloc_count > 0 && hdr.reloc_off < data_size {
        let relocs = data.add(hdr.reloc_off as usize) as *const u64;
        for i in 0..hdr.reloc_count {
            let off = *relocs.add(i as usize);
            if (off + 8) <= mod_size as u64 {
                let ptr = mod_base.add(off as usize) as *mut u64;
                *ptr = (*ptr).wrapping_add(load_base);
            }
        }
    }

    if hdr.import_count > 0 && hdr.import_off < data_size && hdr.strtab_off < data_size {
        let imp = data.add(hdr.import_off as usize) as *const LshImport;
        let strtab = data.add(hdr.strtab_off as usize) as *const u8;
        let strtab_sz = hdr.strtab_size;
        for i in 0..hdr.import_count {
            let entry = &*imp.add(i as usize);
            let t_off = entry.thunk_offset;
            let r_type = entry.reloc_type;
            let n_off = entry.name_offset;
            if (t_off + 8) > mod_size {
                continue;
            }
            if n_off >= strtab_sz {
                continue;
            }
            let name = crate::system::util::lumie_str_from_ptr(strtab.add(n_off as usize));
            let target = crate::system::kernel_exports::kexport_find(name);
            if target.is_null() {
                continue;
            }
            let target_addr = target as u64;
            let patch_addr = load_base.wrapping_add(t_off as u64);
            match r_type {
                REL_ADDR64 => {
                    *(mod_base.add(t_off as usize) as *mut u64) = target_addr;
                }
                REL_ADDR32 => {
                    *(mod_base.add(t_off as usize) as *mut u32) = target_addr as u32;
                }
                REL_REL32 => {
                    let next_pc = patch_addr.wrapping_add(4);
                    *(mod_base.add(t_off as usize) as *mut u32) = target_addr.wrapping_sub(next_pc) as u32;
                }
                _ => {}
            }
        }
    }

    let mut entry_rva = hdr.entry;
    if entry_rva >= mod_size {
        entry_rva = 0;
    }

    mod_.base = mod_base;
    mod_.size = mod_size;
    mod_.loaded = 1;
    mod_.hdr = hdr as *const LshHeader as *const c_void;

    if entry_rva != 0 {
        let entry_fn: ModuleEntryFn = mem::transmute(mod_base.add(entry_rva as usize));
        let mut module_api: *mut c_void = ptr::null_mut();
        entry_fn(kapi, &mut module_api);
        mod_.module_api = module_api;
    }
    0
}

unsafe fn sys_load_format(buf: *const u8, buf_size: u32, kapi: *const c_void, mod_: &mut ModuleT) -> i32 {
    if buf_size < mem::size_of::<ModuleHeader>() as u32 {
        return -1;
    }
    let hdr = &*(buf as *const ModuleHeader);
    if hdr.magic != MOD_MAGIC_SYS {
        return -2;
    }
    let mod_size = hdr.text_size + hdr.bss_size;
    if mod_size == 0 {
        return -3;
    }
    let mod_base = crate::mm::alloc(mod_size as u64);
    if mod_base.is_null() {
        return -4;
    }
    ptr::write_bytes(mod_base, 0, mod_size as usize);
    ptr::copy_nonoverlapping(
        buf.add(mem::size_of::<ModuleHeader>()),
        mod_base,
        hdr.text_size as usize,
    );

    let load_base = mod_base as u64;

    if hdr.reloc_off != 0 && hdr.reloc_count != 0 {
        let relocs = buf.add(hdr.reloc_off as usize) as *const u32;
        for i in 0..hdr.reloc_count {
            let off = *relocs.add(i as usize);
            if (off + 8) <= mod_size {
                let ptr = mod_base.add(off as usize) as *mut u64;
                *ptr = (*ptr).wrapping_add(load_base);
            }
        }
    }

    if !kapi.is_null() && hdr.import_off != 0 && hdr.import_count != 0 {
        let imp = buf.add(hdr.import_off as usize) as *const ModuleImport;
        let kapi_funcs = kapi as *const u64;
        for i in 0..hdr.import_count {
            let entry = &*imp.add(i as usize);
            let off = entry.thunk_offset;
            let ord = entry.ordinal as usize;
            if (off + 8) <= mod_size && (ord * 8) < mem::size_of::<crate::api::KernelApi>() {
                *(mod_base.add(off as usize) as *mut u64) = *kapi_funcs.add(ord);
            }
        }
    }

    mod_.base = mod_base;
    mod_.size = mod_size;
    mod_.loaded = 1;

    if hdr.entry != 0 && hdr.entry < mod_size {
        let entry_fn: ModuleEntryFn = mem::transmute(mod_base.add(hdr.entry as usize));
        let mut module_api: *mut c_void = ptr::null_mut();
        entry_fn(kapi, &mut module_api);
        mod_.module_api = module_api;
    }
    0
}

pub unsafe fn module_load(fat_path: *const u8, kapi: *const c_void, mod_: *mut ModuleT) -> i32 {
    if fat_path.is_null() || mod_.is_null() {
        return -1;
    }
    let mod_ref = &mut *mod_;
    ptr::write_bytes(mod_ref, 0, 1);

    let path = crate::system::util::lumie_str_from_ptr(fat_path);
    let path_ptr = path.as_ptr() as *const u8;
    let fsz = fs::get_file_size(path_ptr);
    if fsz < 64 {
        return -2;
    }
    let mut buf = crate::mm::alloc(fsz as u64);
    if buf.is_null() {
        return -3;
    }
    let ret = fs::read_file(path_ptr, buf, fsz as u32);
    if ret < 64 {
        crate::mm::free(buf);
        return -4;
    }
    let buf_size = ret as u32;
    let lh = &*(buf as *const LshHeader);

    if lh.magic == MOD_MAGIC_LSH || lh.magic == MOD_MAGIC_LDRV || lh.magic == MOD_MAGIC_LKRN {
        let ret2 = lsh_load(buf, buf_size, kapi, mod_ref);
        crate::mm::free(buf);
        return ret2;
    }
    if lh.magic == MOD_MAGIC_SYS {
        let ret2 = sys_load_format(buf, buf_size, kapi, mod_ref);
        crate::mm::free(buf);
        return ret2;
    }

    crate::mm::free(buf);
    -5
}

pub unsafe fn module_unload(mod_: *mut ModuleT) {
    if !mod_.is_null() {
        let m = &mut *mod_;
        if !m.base.is_null() {
            crate::mm::free(m.base);
            m.base = ptr::null_mut();
            m.size = 0;
            m.hdr = ptr::null();
            m.module_api = ptr::null_mut();
            m.loaded = 0;
        }
    }
}

pub unsafe fn module_check(data: *const c_void, size: u32) -> i32 {
    if data.is_null() || size < 64 {
        return 0;
    }
    let lh = &*(data as *const LshHeader);
    if lh.magic == MOD_MAGIC_SYS
        || lh.magic == MOD_MAGIC_LDRV
        || lh.magic == MOD_MAGIC_LSH
        || lh.magic == MOD_MAGIC_LKRN
    {
        return 1;
    }
    0
}
