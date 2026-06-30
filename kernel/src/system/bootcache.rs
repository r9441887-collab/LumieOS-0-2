use core::ptr;
use crate::fs;

pub const BOOTCACHE_PATH: &[u8] = b"/system/boot.cache\0";
pub const BOOTCACHE_LINE_MAX: usize = 512;
pub const BOOTCACHE_MAX_LINES: usize = 64;

static mut G_BOOTCACHE_BUF: [u8; BOOTCACHE_MAX_LINES * BOOTCACHE_LINE_MAX] = [0u8; BOOTCACHE_MAX_LINES * BOOTCACHE_LINE_MAX];
static mut G_BOOTCACHE_LEN: usize = 0;
static mut G_BOOTCACHE_INIT: bool = false;

unsafe fn bootcache_load_internal() {
    if G_BOOTCACHE_INIT {
        return;
    }
    if fs::exists(BOOTCACHE_PATH.as_ptr() as *const u8) {
        let sz = fs::read_file(
            BOOTCACHE_PATH.as_ptr() as *const u8,
            G_BOOTCACHE_BUF.as_mut_ptr(),
            (G_BOOTCACHE_BUF.len() - 1) as u32,
        );
        if sz > 0 {
            G_BOOTCACHE_LEN = sz as usize;
            G_BOOTCACHE_BUF[G_BOOTCACHE_LEN] = 0;
        }
    }
    G_BOOTCACHE_INIT = true;
}

pub unsafe fn bootcache_init() {
    bootcache_load_internal();
}

pub unsafe fn bootcache_save(key: *const u8) -> i32 {
    if key.is_null() {
        return -1;
    }
    bootcache_load_internal();

    let cmd = crate::system::util::lumie_str_from_ptr(key);
    if cmd.is_empty() {
        return -1;
    }

    let cmd_len = cmd.len();
    let need = G_BOOTCACHE_LEN + cmd_len + 2;
    if need > G_BOOTCACHE_BUF.len() {
        return -2;
    }

    let p = G_BOOTCACHE_BUF.as_mut_ptr().add(G_BOOTCACHE_LEN);
    ptr::copy_nonoverlapping(cmd.as_bytes().as_ptr(), p, cmd_len);
    let p = p.add(cmd_len);
    p.write(b'\n');
    p.add(1).write(0);
    G_BOOTCACHE_LEN += cmd_len + 1;

    fs::write_file(
        BOOTCACHE_PATH.as_ptr() as *const u8,
        G_BOOTCACHE_BUF.as_ptr(),
        G_BOOTCACHE_LEN as u32,
    )
}

pub unsafe fn bootcache_load(lines: *mut [u8; 256], max: i32) -> i32 {
    if lines.is_null() || max <= 0 {
        return 0;
    }
    bootcache_load_internal();

    if G_BOOTCACHE_LEN == 0 {
        return 0;
    }

    let mut count: i32 = 0;
    let mut line_start: usize = 0;
    let maxu = max as usize;
    for pos in 0..G_BOOTCACHE_LEN {
        if G_BOOTCACHE_BUF[pos] == b'\n' {
            let len = pos - line_start;
            if len > 0 {
                let len = if len >= 256 { 255 } else { len };
                let line = &mut *lines.add(count as usize);
                line[..len].copy_from_slice(&G_BOOTCACHE_BUF[line_start..line_start + len]);
                line[len] = 0;
                count += 1;
                if count as usize >= maxu {
                    return count;
                }
            }
            line_start = pos + 1;
        }
    }

    if line_start < G_BOOTCACHE_LEN && (count as usize) < maxu {
        let len = G_BOOTCACHE_LEN - line_start;
        let len = if len >= 256 { 255 } else { len };
        let line = &mut *lines.add(count as usize);
        line[..len].copy_from_slice(&G_BOOTCACHE_BUF[line_start..line_start + len]);
        line[len] = 0;
        count += 1;
    }

    count
}

pub unsafe fn bootcache_clear() -> i32 {
    if !fs::exists(BOOTCACHE_PATH.as_ptr() as *const u8) {
        return 0;
    }
    if fs::delete(BOOTCACHE_PATH.as_ptr() as *const u8) == 0 {
        G_BOOTCACHE_LEN = 0;
        G_BOOTCACHE_INIT = false;
        0
    } else {
        -1
    }
}

pub unsafe fn bootcache_count() -> i32 {
    bootcache_load_internal();
    if G_BOOTCACHE_LEN == 0 {
        return 0;
    }
    let mut count: i32 = 0;
    for i in 0..G_BOOTCACHE_LEN {
        if G_BOOTCACHE_BUF[i] == b'\n' {
            count += 1;
        }
    }
    if G_BOOTCACHE_LEN > 0 && G_BOOTCACHE_BUF[G_BOOTCACHE_LEN - 1] != b'\n' {
        count += 1;
    }
    count
}
