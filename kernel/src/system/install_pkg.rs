use core::ffi::c_void;
use core::ptr;
use crate::fs;
use crate::mm;

const PKG_MAGIC: u32 = 0x4B47504C;
const FLAG_DIR: u8 = 1;
const FLAG_LZ1: u8 = 2;

#[repr(C)]
pub struct InstallPkg {
    pub present: u8,
    pub data: *mut u8,
    pub data_size: u32,
    pub file_count: u32,
    pub entries_off: u32,
}

/// LZ1 block decompression.
/// Custom format: control byte (8 flags MSB), literals (0) or 3-byte references (1: u16 LE offset + u8 length-3).
unsafe fn lz1_decompress(src: *const u8, src_len: u32, dst: *mut u8, dst_len: u32) -> i32 {
    let mut ip: usize = 0;
    let mut op: usize = 0;
    let src_end = src_len as usize;
    let dst_end = dst_len as usize;

    while ip < src_end && op < dst_end {
        let mut ctrl = *src.add(ip) as u32;
        ip += 1;
        for _ in 0..8 {
            if ip >= src_end || op >= dst_end { break; }
            if ctrl & 0x80 != 0 {
                if ip + 3 > src_end { return -1; }
                let offset = *src.add(ip) as usize | ((*src.add(ip + 1) as usize) << 8);
                let length = *src.add(ip + 2) as usize + 3;
                ip += 3;
                if offset == 0 || offset > op { return -1; }
                if op + length > dst_end { return -1; }
                let mut match_pos = op - offset;
                for _ in 0..length {
                    *dst.add(op) = *dst.add(match_pos);
                    op += 1;
                    match_pos += 1;
                }
            } else {
                *dst.add(op) = *src.add(ip);
                ip += 1;
                op += 1;
            }
            ctrl <<= 1;
        }
    }

    if op != dst_len as usize { -2 } else { 0 }
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

    let buf = mm::alloc(file_size as u64);
    if buf.is_null() {
        return -1;
    }
    let ret = fs::read_file(path_ptr, buf, file_size);
    if ret < 16 {
        mm::free(buf);
        return -2;
    }
    let len = (if (ret as u32) < file_size { ret } else { file_size as i32 }) as u32;

    let magic = *(buf as *const u32);
    if magic != PKG_MAGIC {
        mm::free(buf);
        return -3;
    }

    // version at +4, file_count at +8, entries_off at +12
    let entries_off = *(buf.add(12) as *const u32);
    let file_count = *(buf.add(8) as *const u32);

    pkg_ref.data = buf;
    pkg_ref.data_size = len;
    pkg_ref.file_count = file_count;
    pkg_ref.entries_off = entries_off;
    pkg_ref.present = 1;
    0
}

pub unsafe fn install_pkg_close(pkg: *mut c_void) -> i32 {
    let pkg_ref = &mut *(pkg as *mut InstallPkg);
    if !pkg_ref.data.is_null() {
        mm::free(pkg_ref.data);
        pkg_ref.data = ptr::null_mut();
        pkg_ref.present = 0;
    }
    0
}

pub unsafe fn install_pkg_extract_all(pkg: *mut c_void, _progress: *mut c_void) -> i32 {
    let pkg_ref = &mut *(pkg as *mut InstallPkg);
    if pkg_ref.present == 0 || pkg_ref.data.is_null() {
        return -1;
    }

    let data = pkg_ref.data;
    let file_count = pkg_ref.file_count;
    let entries_off = pkg_ref.entries_off as usize;

    for i in 0..file_count as usize {
        let entry = data.add(entries_off + i * 32);
        let path_off = *(entry as *const u32);
        let data_off = *(entry.add(4) as *const u32);
        let store_sz = *(entry.add(8) as *const u32);
        let flags = *(entry.add(12) as *const u8);

        let path = data.add(path_off as usize);

        if flags & FLAG_DIR != 0 {
            continue;
        }

        // Map LPKG path to kernel destination path
        let mut dest_path: [u8; 128] = [0u8; 128];
        let path_slice = core::slice::from_raw_parts(path, 128);
        let path_len = crate::system::util::lumie_strlen_raw(path_slice);
        if path_len > 127 { continue; }
        ptr::copy_nonoverlapping(path, dest_path.as_mut_ptr(), path_len);
        dest_path[path_len] = 0;

        // Write file (decompress if needed)
        if flags & FLAG_LZ1 != 0 {
            let orig_sz = *(entry.add(16) as *const u32);
            let tmp = mm::alloc(orig_sz as u64);
            if tmp.is_null() { return -1; }
            if lz1_decompress(data.add(data_off as usize), store_sz, tmp, orig_sz) != 0 {
                mm::free(tmp);
                return -2;
            }
            fs::write_file(dest_path.as_ptr(), tmp, orig_sz);
            mm::free(tmp);
        } else {
            fs::write_file(dest_path.as_ptr(), data.add(data_off as usize), store_sz);
        }
    }

    0
}
