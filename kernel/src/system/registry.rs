use core::ptr;
use crate::fs;

pub const REG_MAX_KEYS: usize = 64;
pub const REG_KEY_LEN: usize = 64;
pub const REG_VAL_LEN: usize = 256;
pub const REG_FILE: &[u8] = b"/system/registry.cfg\0";

#[repr(C)]
struct RegEntry {
    key: [u8; REG_KEY_LEN],
    val: [u8; REG_VAL_LEN],
}

static mut G_REGS: [RegEntry; REG_MAX_KEYS] = [RegEntry {
    key: [0u8; REG_KEY_LEN],
    val: [0u8; REG_VAL_LEN],
}; REG_MAX_KEYS];
static mut G_REG_COUNT: usize = 0;

unsafe fn parse_registry() -> i32 {
    let mut buf: [u8; 8192] = [0u8; 8192];
    let path_ptr = REG_FILE.as_ptr() as *const u8;
    let sz = fs::get_file_size(path_ptr);
    if sz <= 0 || sz > 8192 {
        return -1;
    }
    let r = fs::read_file(path_ptr, buf.as_mut_ptr(), sz as u32);
    if r <= 0 {
        return -1;
    }
    let len = r as usize;
    buf[len] = 0;

    let mut count: usize = 0;
    let mut pos: usize = 0;
    while pos < len && count < REG_MAX_KEYS {
        while pos < len && (buf[pos] == b'\r' || buf[pos] == b'\n') {
            pos += 1;
        }
        if pos >= len || buf[pos] == 0 {
            break;
        }
        let ks = pos;
        let mut kend: isize = -1;
        while pos < len && buf[pos] != b'\n' {
            if buf[pos] == b'=' && kend < 0 {
                kend = pos as isize;
            }
            pos += 1;
        }
        if pos < len && buf[pos] == b'\n' {
            buf[pos] = 0;
            pos += 1;
        }
        if kend < 0 {
            continue;
        }
        let klen = (kend - ks as isize) as usize;
        let klen = if klen >= REG_KEY_LEN {
            REG_KEY_LEN - 1
        } else {
            klen
        };
        G_REGS[count].key[..klen].copy_from_slice(&buf[ks..ks + klen]);
        G_REGS[count].key[klen] = 0;

        let vs = (kend + 1) as usize;
        let mut vlen = crate::system::util::lumie_strlen_raw(&buf[vs..]);
        while vlen > 0 && (buf[vs + vlen - 1] == b'\r' || buf[vs + vlen - 1] == b'\n') {
            vlen -= 1;
        }
        let vlen = if vlen >= REG_VAL_LEN {
            REG_VAL_LEN - 1
        } else {
            vlen
        };
        G_REGS[count].val[..vlen].copy_from_slice(&buf[vs..vs + vlen]);
        G_REGS[count].val[vlen] = 0;
        count += 1;
    }
    G_REG_COUNT = count;
    count as i32
}

unsafe fn save_registry() -> i32 {
    let mut buf: [u8; 8192] = [0u8; 8192];
    let mut pos: usize = 0;
    for i in 0..G_REG_COUNT {
        if pos >= 8100 {
            break;
        }
        let klen = crate::system::util::lumie_strlen_raw(&G_REGS[i].key);
        let vlen = crate::system::util::lumie_strlen_raw(&G_REGS[i].val);
        if pos + klen + vlen + 4 > 8100 {
            break;
        }
        buf[pos..pos + klen].copy_from_slice(&G_REGS[i].key[..klen]);
        pos += klen;
        buf[pos] = b'=';
        pos += 1;
        buf[pos..pos + vlen].copy_from_slice(&G_REGS[i].val[..vlen]);
        pos += vlen;
        buf[pos] = b'\n';
        pos += 1;
    }
    fs::write_file(REG_FILE.as_ptr() as *const u8, buf.as_ptr(), pos as u32)
}

pub unsafe fn reg_init() -> i32 {
    G_REG_COUNT = 0;
    if !fs::exists(REG_FILE.as_ptr() as *const u8) {
        let def = b"Start=/system/shell.lsh\n";
        fs::write_file(REG_FILE.as_ptr() as *const u8, def.as_ptr(), def.len() as u32);
    }
    parse_registry()
}

pub unsafe fn reg_get(key: *const u8, val: *mut u8, max_len: u32) -> i32 {
    if key.is_null() || val.is_null() || max_len == 0 {
        return -1;
    }
    let key_str = crate::system::util::lumie_str_from_ptr(key);
    for i in 0..G_REG_COUNT {
        if crate::system::util::lumie_strcmp_raw(&G_REGS[i].key, key_str) == 0 {
            let vlen = crate::system::util::lumie_strlen_raw(&G_REGS[i].val);
            let vlen = if vlen >= max_len as usize {
                (max_len - 1) as usize
            } else {
                vlen
            };
            let slice = core::slice::from_raw_parts_mut(val, max_len as usize);
            slice[..vlen].copy_from_slice(&G_REGS[i].val[..vlen]);
            slice[vlen] = 0;
            return vlen as i32;
        }
    }
    -1
}

pub unsafe fn reg_set(key: *const u8, val: *const u8) -> i32 {
    if key.is_null() {
        return -1;
    }
    let key_str = crate::system::util::lumie_str_from_ptr(key);
    for i in 0..G_REG_COUNT {
        if crate::system::util::lumie_strcmp_raw(&G_REGS[i].key, key_str) == 0 {
            if !val.is_null() {
                let val_str = crate::system::util::lumie_str_from_ptr(val);
                let vlen = val_str.len();
                let vlen = if vlen >= REG_VAL_LEN {
                    REG_VAL_LEN - 1
                } else {
                    vlen
                };
                G_REGS[i].val[..vlen].copy_from_slice(val_str.as_bytes());
                G_REGS[i].val[vlen] = 0;
            } else {
                G_REGS[i].val[0] = 0;
            }
            return save_registry();
        }
    }
    if G_REG_COUNT >= REG_MAX_KEYS {
        return -1;
    }
    let klen = key_str.len();
    let klen = if klen >= REG_KEY_LEN {
        REG_KEY_LEN - 1
    } else {
        klen
    };
    G_REGS[G_REG_COUNT].key[..klen].copy_from_slice(key_str.as_bytes());
    G_REGS[G_REG_COUNT].key[klen] = 0;
    if !val.is_null() {
        let val_str = crate::system::util::lumie_str_from_ptr(val);
        let vlen = val_str.len();
        let vlen = if vlen >= REG_VAL_LEN {
            REG_VAL_LEN - 1
        } else {
            vlen
        };
        G_REGS[G_REG_COUNT].val[..vlen].copy_from_slice(val_str.as_bytes());
        G_REGS[G_REG_COUNT].val[vlen] = 0;
    } else {
        G_REGS[G_REG_COUNT].val[0] = 0;
    }
    G_REG_COUNT += 1;
    save_registry()
}

pub unsafe fn reg_del(key: *const u8) -> i32 {
    let key_str = crate::system::util::lumie_str_from_ptr(key);
    if key_str.is_empty() {
        return -1;
    }
    for i in 0..G_REG_COUNT {
        if crate::system::util::lumie_strcmp_raw(&G_REGS[i].key, key_str) == 0 {
            for j in i..G_REG_COUNT - 1 {
                G_REGS[j] = G_REGS[j + 1];
            }
            G_REG_COUNT -= 1;
            return save_registry();
        }
    }
    -1
}

pub unsafe fn reg_list(buf: *mut u8, max_len: u32) -> i32 {
    if buf.is_null() || max_len == 0 {
        return 0;
    }
    let maxu = max_len as usize;
    let mut pos: usize = 0;
    for i in 0..G_REG_COUNT {
        if pos >= maxu - 1 {
            break;
        }
        let klen = crate::system::util::lumie_strlen_raw(&G_REGS[i].key);
        let vlen = crate::system::util::lumie_strlen_raw(&G_REGS[i].val);
        let space = maxu - 1 - pos;
        if klen + vlen + 5 > space {
            break;
        }
        let slice = core::slice::from_raw_parts_mut(buf.add(pos), maxu - pos);
        slice[..klen].copy_from_slice(&G_REGS[i].key[..klen]);
        pos += klen;
        buf.add(pos).write(b' ');
        pos += 1;
        buf.add(pos).write(b'=');
        pos += 1;
        buf.add(pos).write(b' ');
        pos += 1;
        let slice2 = core::slice::from_raw_parts_mut(buf.add(pos), maxu - pos);
        slice2[..vlen].copy_from_slice(&G_REGS[i].val[..vlen]);
        pos += vlen;
        buf.add(pos).write(b'\n');
        pos += 1;
    }
    if pos < maxu {
        buf.add(pos).write(0);
    }
    pos as i32
}

pub unsafe fn reg_get_start(buf: *mut u8) -> i32 {
    let key = b"Start\0";
    reg_get(key.as_ptr(), buf, 256)
}
