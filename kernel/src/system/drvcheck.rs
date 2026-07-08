use crate::console::terminal;
use crate::fs;
use crate::system::module;

pub const DRVCHECK_MAX_WHITELIST: usize = 32;
pub const DRVCHECK_MAX_REPORT: i32 = 128;
pub const DRVCHECK_PATH_MAX: usize = 256;

const WHITELIST: &[&[u8]] = &[
    b"edit", b"krnl", b"sh", b"term", b"mouse", b"kbd", b"gop",
    b"fs", b"net", b"rtl", b"tls", b"util", b"extr",
    b"shell", b"desktop", b"taskmgr", b"kernel",
    b"nv_gpu", b"ahci", b"ps2kbd", b"ps2mouse", b"pit",
    b"sched", b"users", b"registry", b"disk_io", b"ramdisk",
    b"setup", b"install_pkg", b"xhci", b"lc", b"pcspkr",
];

#[repr(C)]
pub struct DrvCheckReport {
    pub path: [u8; DRVCHECK_PATH_MAX],
    pub name: [u8; 64],
    pub has_invalid_chars: i32,
    pub not_in_whitelist: i32,
    pub suspicious_content: i32,
    pub is_pe: i32,
    pub is_duplicate: i32,
}

fn is_invalid_char(c: u8) -> bool {
    match c {
        b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => false,
        b'_' | b'#' | b'$' | b'%' | b'@'
        | b'!' | b'*' | b'&' | b'^' | b'~'
        | b'`' | b'\'' | b'\"' | b'|'
        | b'\\' | b'/' | b':' | b'?' | b'<' | b'>'
        | b' ' | b'\t'
        | b'(' | b')' | b'[' | b']' | b'{' | b'}'
        | b',' | b';' | b'=' | b'+' => true,
        _ => false,
    }
}

fn has_suspicious_ext(name: &[u8]) -> bool {
    let len = name.len();
    if len < 5 {
        return false;
    }
    let ext = &name[len - 4..];
    if ext == b"exe\0" || ext == b"dll\0" || ext == b"com\0"
        || ext == b"bat\0" || ext == b"cmd\0" || ext == b"vbs\0" || ext == b"ps1\0"
    {
        let ext2 = &name[len - 4..len];
        if ext2 != b"ldrv" && ext2 != b".lsh" && ext2 != b".sys" && ext2 != b"lkrn" {
            return ext == b"exe\0" || ext == b"dll\0" || ext == b"com\0"
                || ext == b"bat\0" || ext == b"cmd\0" || ext == b"vbs\0" || ext == b"ps1\0";
        }
    }
    false
}

fn has_lumie_ext(name: &[u8]) -> bool {
    let len = name.len();
    if len < 5 {
        return false;
    }
    let ext = &name[len - 4..];
    ext == b"ldrv" || ext == b".lsh" || ext == b".sys" || ext == b"lkrn"
}

fn strip_ext(name: &[u8], buf: &mut [u8]) {
    let len = name.len();
    let mut out_len = len;
    for i in (0..len).rev() {
        if name[i] == b'.' {
            out_len = i;
            break;
        }
    }
    buf[..out_len].copy_from_slice(&name[..out_len]);
    buf[out_len] = 0;
}

fn check_pe_content(buf: &[u8], rep: &mut DrvCheckReport) -> i32 {
    if buf.len() < 128 {
        return 0;
    }
    if buf.len() > 4 && buf[0] == b'M' && buf[1] == b'Z' {
        rep.is_pe = 1;
        rep.suspicious_content = 1;
        return 1;
    }
    let mut bad_count: i32 = 0;
    let check_len = if buf.len() > 4096 { 4096 } else { buf.len() };
    for i in 0..check_len - 1 {
        if buf[i] == 0 {
            bad_count += 1;
        }
    }
    if bad_count > 100 {
        rep.suspicious_content = 1;
        return 1;
    }
    0
}

pub unsafe fn drvcheck_validate_name(name: *const u8) -> i32 {
    if name.is_null() {
        return -1;
    }
    let mut i = 0;
    while *name.add(i) != 0 {
        if is_invalid_char(*name.add(i)) {
            return -1;
        }
        i += 1;
    }
    0
}

pub unsafe fn drvcheck_whitelist_check(name: *const u8) -> i32 {
    if name.is_null() {
        return 0;
    }
    let name_str = crate::system::util::lumie_str_from_ptr(name);
    let mut clean: [u8; 64] = [0u8; 64];
    strip_ext(name_str.as_bytes(), &mut clean);
    for &w in WHITELIST {
        let wlen = w.len();
        if clean[..wlen] == *w && clean[wlen] == 0 {
            return 1;
        }
    }
    0
}

pub unsafe fn drvcheck_scan_file(fat_path: *const u8, rep: *mut DrvCheckReport) -> i32 {
    if fat_path.is_null() || rep.is_null() {
        return -1;
    }
    let rep_ref = &mut *rep;
    core::ptr::write_bytes(rep_ref, 0, 1);

    let path_str = crate::system::util::lumie_str_from_ptr(fat_path);
    let path_bytes = path_str.as_bytes();
    let plen = path_bytes.len();
    if plen < DRVCHECK_PATH_MAX {
        rep_ref.path[..plen].copy_from_slice(path_bytes);
        rep_ref.path[plen] = 0;
    }

    let mut fname: &[u8] = path_bytes;
    for i in (0..path_bytes.len()).rev() {
        if path_bytes[i] == b'/' || path_bytes[i] == b'\\' {
            fname = &path_bytes[i + 1..];
            break;
        }
    }
    let flen = fname.len();
    if flen < 64 {
        rep_ref.name[..flen].copy_from_slice(fname);
        rep_ref.name[flen] = 0;
    }

    if drvcheck_validate_name(rep_ref.name.as_ptr()) != 0 {
        rep_ref.has_invalid_chars = 1;
    }

    if drvcheck_whitelist_check(rep_ref.name.as_ptr()) == 0 {
        rep_ref.not_in_whitelist = 1;
    }

    let path_ptr = path_str.as_ptr() as *const u8;
    let fsz = fs::get_file_size(path_ptr);
    if fsz > 0 {
        let buf = crate::mm::alloc(fsz as u64);
        if !buf.is_null() {
            let rd = fs::read_file(path_ptr, buf, fsz as u32);
            if rd > 0 {
                let data = core::slice::from_raw_parts(buf, rd as usize);
                check_pe_content(data, rep_ref);
                if !has_lumie_ext(fname) && !has_suspicious_ext(fname) && rep_ref.suspicious_content == 0 {
                    if rd >= 64 {
                        let magic = *(buf as *const u32);
                        if magic == module::MOD_MAGIC_LSH
                            || magic == module::MOD_MAGIC_LDRV
                            || magic == module::MOD_MAGIC_LKRN
                            || magic == module::MOD_MAGIC_SYS
                        {
                            rep_ref.suspicious_content = 1;
                        }
                    }
                }
            }
            crate::mm::free(buf);
        }
    }

    if rep_ref.has_invalid_chars == 0 && rep_ref.not_in_whitelist == 0 && rep_ref.suspicious_content == 0 {
        0
    } else {
        -2
    }
}

pub unsafe fn drvcheck_scan_drivers(reports: *mut DrvCheckReport, max_reports: i32) -> i32 {
    if reports.is_null() || max_reports <= 0 {
        return 0;
    }
    let mut count: i32 = 0;
    let scan_dirs: &[*const u8] = &[
        b"/drivers\0" as *const u8,
        b"/system\0" as *const u8,
        b"/\0" as *const u8,
    ];

    for &dir in scan_dirs {
        if !fs::exists(dir) {
            continue;
        }
        let mut entries: [crate::fs::LumieDirEnt; 256] = [crate::fs::LumieDirEnt {
            name: [0u8; 256],
            is_dir: 0,
            size: 0,
        }; 256];
        let n = fs::list_dir(dir, entries.as_mut_ptr(), 256);
        if n < 0 {
            continue;
        }
        for i in 0..n {
            if (count as i32) >= max_reports {
                break;
            }
            let ename = &entries[i as usize];
            if ename.is_dir != 0 {
                continue;
            }
            let name_bytes = &ename.name;
            let name_len = crate::system::util::lumie_strlen_raw(name_bytes);
            let name_slice = &name_bytes[..name_len];
            if !has_lumie_ext(name_slice) && !has_suspicious_ext(name_slice) {
                continue;
            }
            let mut full: [u8; DRVCHECK_PATH_MAX] = [0u8; DRVCHECK_PATH_MAX];
            let dir_str = crate::system::util::lumie_str_from_ptr(dir);
            let dlen = dir_str.len();
            full[..dlen].copy_from_slice(dir_str.as_bytes());
            let mut flen = dlen;
            if flen > 0 && full[flen - 1] != b'/' {
                full[flen] = b'/';
                flen += 1;
            }
            full[flen..flen + name_len].copy_from_slice(name_slice);
            flen += name_len;
            full[flen] = 0;

            let ret = drvcheck_scan_file(full.as_ptr(), reports.add(count as usize));
            if ret != 0 {
                count += 1;
            }
        }
    }
    count
}

pub unsafe fn drvcheck_delete_suspicious(reports: *const DrvCheckReport, count: i32) -> i32 {
    let mut deleted: i32 = 0;
    for i in 0..count {
        let rep = &*reports.add(i as usize);
        if rep.has_invalid_chars != 0 || rep.suspicious_content != 0 {
            let path = crate::system::util::lumie_str_from_raw_ptr(&rep.path);
            if fs::delete(path.as_ptr() as *const u8) == 0 {
                deleted += 1;
            }
        }
    }
    deleted
}

pub unsafe fn drvcheck_run_scan() {
    let mut reports: [DrvCheckReport; DRVCHECK_MAX_REPORT as usize] = core::mem::zeroed();
    let count = drvcheck_scan_drivers(reports.as_mut_ptr(), DRVCHECK_MAX_REPORT);

    terminal::term_set_fg(0xFFFF00);
    terminal::term_write(b"=== Driver Validation Scan ===\n\0" as *const u8);
    terminal::term_write(b"Scanned \0" as *const u8);
    let mut nbuf: [u8; 16] = [0u8; 16];
    crate::system::util::lumie_itoa(count as i64, nbuf.as_mut_ptr(), 10);
    terminal::term_write(nbuf.as_ptr());
    terminal::term_writeln(b" suspicious files found.\0" as *const u8);
    terminal::term_set_fg(0xFFFFFF);

    if count == 0 {
        terminal::term_set_fg(0x00FF00);
        terminal::term_writeln(b"All drivers pass validation.\0" as *const u8);
        terminal::term_set_fg(0xFFFFFF);
        return;
    }

    for i in 0..count {
        let rep = &reports[i as usize];
        terminal::term_set_fg(0xFF4444);
        terminal::term_write(b"  [\0" as *const u8);
        terminal::term_write(rep.path.as_ptr());
        terminal::term_write(b"]\0" as *const u8);
        if rep.has_invalid_chars != 0 {
            terminal::term_write(b" BAD_NAME\0" as *const u8);
        }
        if rep.not_in_whitelist != 0 {
            terminal::term_write(b" UNKNOWN\0" as *const u8);
        }
        if rep.suspicious_content != 0 {
            terminal::term_write(b" SUSPECT\0" as *const u8);
        }
        if rep.is_pe != 0 {
            terminal::term_write(b" PE\0" as *const u8);
        }
        terminal::term_putchar(b'\n');
    }
    terminal::term_set_fg(0xFFFFFF);

    let deleted = drvcheck_delete_suspicious(reports.as_ptr(), count);
    if deleted > 0 {
        let mut dbuf: [u8; 16] = [0u8; 16];
        crate::system::util::lumie_itoa(deleted as i64, dbuf.as_mut_ptr(), 10);
        terminal::term_set_fg(0x00FF00);
        terminal::term_write(b"Deleted \0" as *const u8);
        terminal::term_write(dbuf.as_ptr());
        terminal::term_writeln(b" suspicious files.\0" as *const u8);
        terminal::term_set_fg(0xFFFFFF);
    }
}
