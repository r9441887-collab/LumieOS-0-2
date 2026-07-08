use core::ffi::c_void;
use core::mem;
use core::ptr;
use crate::fs;

pub const INSTALL_PKG_VER: u32 = 1;
pub const INSTALL_PKG_MAX_FILES: usize = 64;

#[repr(C)]
pub struct InstallPkgEntry {
    pub name: [u8; 64],
    pub offset: u32,
    pub size: u32,
}

#[repr(C)]
pub struct InstallPkg {
    pub present: u8,
    pub total_files: u32,
    pub data_offset: u32,
    pub data: *mut u8,
    pub data_size: u32,
    pub files: [InstallPkgEntry; INSTALL_PKG_MAX_FILES],
}

pub unsafe fn install_pkg_open(path: *const u8, pkg: *mut c_void) -> i32 {
    let pkg_ref = &mut *(pkg as *mut InstallPkg);
    ptr::write_bytes(pkg_ref, 0, 1);

    let path_str = crate::system::util::lumie_str_from_ptr(path);
    let path_ptr = path_str.as_ptr() as *const u8;
    let fsz = fs::get_file_size(path_ptr);
    if fsz < 16 {
        return -1;
    }
    let file_size = fsz as u32;

    let buf = crate::mm::alloc(file_size as u64);
    if buf.is_null() {
        return -1;
    }
    let ret = fs::read_file(path_ptr, buf, file_size);
    if ret < 16 {
        crate::mm::free(buf);
        return -2;
    }
    let len = (if (ret as u32) < file_size { ret } else { file_size as i32 }) as u32;

    let magic = [
        *buf.add(0), *buf.add(1), *buf.add(2), *buf.add(3),
        *buf.add(4), *buf.add(5), *buf.add(6), *buf.add(7),
    ];
    if &magic != b"LUMIEPKG" {
        crate::mm::free(buf);
        return -3;
    }

    let version = *(buf.add(8) as *const u32);
    if version != INSTALL_PKG_VER {
        crate::mm::free(buf);
        return -4;
    }

    let total_files = *(buf.add(12) as *const u32);
    if total_files > INSTALL_PKG_MAX_FILES as u32 {
        crate::mm::free(buf);
        return -5;
    }

    pkg_ref.data = buf;
    pkg_ref.data_size = len;
    pkg_ref.total_files = total_files;
    pkg_ref.data_offset = 16 + total_files * mem::size_of::<InstallPkgEntry>() as u32;

    for i in 0..total_files as usize {
        let entry_off = 16 + i * mem::size_of::<InstallPkgEntry>();
        let entry = &mut pkg_ref.files[i];
        entry.name[..63].copy_from_slice(&core::slice::from_raw_parts(buf.add(entry_off), 63));
        entry.name[63] = 0;
        entry.offset = *(buf.add(entry_off + 64) as *const u32);
        entry.size = *(buf.add(entry_off + 68) as *const u32);
    }

    pkg_ref.present = 1;
    0
}

pub unsafe fn install_pkg_close(pkg: *mut c_void) -> i32 {
    let pkg_ref = &mut *(pkg as *mut InstallPkg);
    if !pkg_ref.data.is_null() {
        crate::mm::free(pkg_ref.data);
        pkg_ref.data = ptr::null_mut();
        pkg_ref.present = 0;
    }
    0
}

pub unsafe fn install_pkg_find(pkg: *mut c_void, name: *const u8, size: *mut u32) -> i32 {
    let pkg_ref = &mut *(pkg as *mut InstallPkg);
    if pkg_ref.present == 0 {
        return -1;
    }
    let name_str = crate::system::util::lumie_str_from_ptr(name);
    for i in 0..pkg_ref.total_files as usize {
        let ename = crate::system::util::lumie_str_from_raw_ptr(&pkg_ref.files[i].name);
        if ename == name_str {
            if !size.is_null() {
                *size = pkg_ref.files[i].size;
            }
            return i as i32;
        }
    }
    -1
}

pub unsafe fn install_pkg_extract(pkg: *mut c_void, file_name: *const u8, buf: *mut u8, max_size: u32) -> i32 {
    let pkg_ref = &mut *(pkg as *mut InstallPkg);
    if pkg_ref.present == 0 || pkg_ref.data.is_null() {
        return -1;
    }
    let idx = install_pkg_find(pkg, file_name, ptr::null_mut());
    if idx < 0 {
        return -2;
    }
    let ent = &pkg_ref.files[idx as usize];
    if ent.size > max_size {
        return -3;
    }
    let src_off = pkg_ref.data_offset + ent.offset;
    if (src_off + ent.size) > pkg_ref.data_size {
        return -4;
    }
    ptr::copy_nonoverlapping(pkg_ref.data.add(src_off as usize), buf, ent.size as usize);
    ent.size as i32
}

pub unsafe fn install_pkg_extract_all(pkg: *mut c_void, _progress: *mut c_void) -> i32 {
    let pkg_ref = &mut *(pkg as *mut InstallPkg);
    if pkg_ref.present == 0 || pkg_ref.data.is_null() {
        return -1;
    }

    for i in 0..pkg_ref.total_files as usize {
        let ent = &pkg_ref.files[i];

        let mut dest_path: [u8; 128] = [0u8; 128];
        let ename = crate::system::util::lumie_str_from_raw_ptr(&ent.name);
        if ename == "kernel.lkrn" {
            dest_path[..15].copy_from_slice(b"/system/kernel.lkrn");
        } else if ename == "shell.lsh" || ename == "sh" {
            dest_path[..16].copy_from_slice(b"/system/shell.lsh");
        } else {
            dest_path[..9].copy_from_slice(b"/drivers/");
            let mut pos = 9;
            let name_len = crate::system::util::lumie_strlen_raw(&ent.name);
            dest_path[pos..pos + name_len].copy_from_slice(&ent.name[..name_len]);
            pos += name_len;
            let ext = b".ldrv";
            dest_path[pos..pos + 5].copy_from_slice(ext);
            pos += 5;
            dest_path[pos] = 0;
        }

        let src_off = pkg_ref.data_offset + ent.offset;
        if (src_off + ent.size) > pkg_ref.data_size {
            continue;
        }

        if !fs::exists(b"/system\0" as *const u8) {
            fs::mkdir(b"/system\0" as *const u8);
        }
        if !fs::exists(b"/drivers\0" as *const u8) {
            fs::mkdir(b"/drivers\0" as *const u8);
        }
        fs::write_file(
            dest_path.as_ptr() as *const u8,
            pkg_ref.data.add(src_off as usize),
            ent.size,
        );
    }

    0
}
