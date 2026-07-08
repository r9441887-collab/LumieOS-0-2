use core::ptr;
use crate::fs;

pub const MAX_USERS: usize = 16;
pub const USER_NAME_LEN: usize = 32;
pub const USER_ROLE_USER: i32 = 0;
pub const USER_ROLE_ADMIN: i32 = 1;
pub const USER_SESSION_FILE: &[u8] = b"/system/users.cfg\0";

#[repr(C)]
#[derive(Clone, Copy)]
struct UserEntry {
    name: [u8; USER_NAME_LEN],
    pass: [u8; USER_NAME_LEN],
    role: i32,
}

static mut G_USERS: [UserEntry; MAX_USERS] = [UserEntry {
    name: [0u8; USER_NAME_LEN],
    pass: [0u8; USER_NAME_LEN],
    role: 0,
}; MAX_USERS];
static mut G_USER_COUNT: i32 = 0;
static mut G_LOGGED_IN: i32 = -1;
static mut G_CURRENT_NAME: [u8; USER_NAME_LEN] = [0u8; USER_NAME_LEN];
static mut G_CURRENT_ROLE: i32 = 0;
static mut G_LOGGED_IN_FLAG: bool = false;

unsafe fn parse_users() -> i32 {
    let mut buf: [u8; 4096] = [0u8; 4096];
    let path_ptr = USER_SESSION_FILE.as_ptr() as *const u8;
    let sz = fs::get_file_size(path_ptr);
    if sz <= 0 || sz > 4096 {
        return -1;
    }
    let r = fs::read_file(path_ptr, buf.as_mut_ptr(), sz as u32);
    if r <= 0 {
        return -1;
    }
    let len = r as usize;
    buf[len] = 0;

    let mut count: i32 = 0;
    let mut pos: usize = 0;
    while pos < len && (count as usize) < MAX_USERS {
        while pos < len && (buf[pos] == b'\r' || buf[pos] == b'\n') {
            pos += 1;
        }
        if pos >= len || buf[pos] == 0 {
            break;
        }
        let start = pos;
        let mut c1: isize = -1;
        let mut c2: isize = -1;
        while pos < len && buf[pos] != b'\n' {
            if buf[pos] == b':' {
                if c1 < 0 {
                    c1 = pos as isize;
                } else if c2 < 0 {
                    c2 = pos as isize;
                }
            }
            pos += 1;
        }
        let line_end = pos;
        if pos < len && buf[pos] == b'\n' {
            pos += 1;
        }
        if c1 < 0 {
            continue;
        }
        let nlen = (c1 - start as isize) as usize;
        if nlen == 0 || nlen >= USER_NAME_LEN {
            continue;
        }
        let idx = count as usize;
        G_USERS[idx].name[..nlen].copy_from_slice(&buf[start..start + nlen]);
        G_USERS[idx].name[nlen] = 0;

        if c2 > c1 + 1 {
            let plen = (c2 - (c1 + 1)) as usize;
            let plen = if plen >= USER_NAME_LEN {
                USER_NAME_LEN - 1
            } else {
                plen
            };
            let ps = (c1 + 1) as usize;
            G_USERS[idx].pass[..plen].copy_from_slice(&buf[ps..ps + plen]);
            G_USERS[idx].pass[plen] = 0;
        } else {
            G_USERS[idx].pass[0] = 0;
        }

        G_USERS[idx].role = USER_ROLE_USER;
        if c2 > 0 && (c2 as usize + 1) < line_end && buf[(c2 + 1) as usize] == b'1' {
            G_USERS[idx].role = USER_ROLE_ADMIN;
        }
        count += 1;
    }
    G_USER_COUNT = count;
    count
}

unsafe fn save_users() -> i32 {
    let mut buf: [u8; 1024] = [0u8; 1024];
    let mut pos: usize = 0;
    for i in 0..(G_USER_COUNT as usize) {
        if pos >= 1000 {
            break;
        }
        let nlen = crate::system::util::lumie_strlen_raw(&G_USERS[i].name);
        let plen = crate::system::util::lumie_strlen_raw(&G_USERS[i].pass);
        if pos + nlen + plen + 10 > 1000 {
            break;
        }
        buf[pos..pos + nlen].copy_from_slice(&G_USERS[i].name[..nlen]);
        pos += nlen;
        buf[pos] = b':';
        pos += 1;
        buf[pos..pos + plen].copy_from_slice(&G_USERS[i].pass[..plen]);
        pos += plen;
        buf[pos] = b':';
        pos += 1;
        buf[pos] = b'0' + G_USERS[i].role as u8;
        pos += 1;
        buf[pos] = b'\n';
        pos += 1;
    }
    fs::write_file(USER_SESSION_FILE.as_ptr() as *const u8, buf.as_ptr(), pos as u32)
}

pub unsafe fn users_init() -> i32 {
    if !fs::exists(USER_SESSION_FILE.as_ptr() as *const u8) {
        let default_users = b"user::0\nadministrator::1\n";
        fs::write_file(
            USER_SESSION_FILE.as_ptr() as *const u8,
            default_users.as_ptr(),
            default_users.len() as u32,
        );
    }
    G_LOGGED_IN = -1;
    G_LOGGED_IN_FLAG = false;
    G_USER_COUNT = 0;
    parse_users()
}

pub unsafe fn users_login(name: *const u8, pass: *const u8) -> i32 {
    if name.is_null() {
        return -1;
    }
    let name_str = crate::system::util::lumie_str_from_ptr(name);
    let pass_str = if pass.is_null() {
        crate::system::util::lumie_str_from_ptr(ptr::null())
    } else {
        crate::system::util::lumie_str_from_ptr(pass)
    };
    for i in 0..(G_USER_COUNT as usize) {
        if crate::system::util::lumie_strcmp_raw(&G_USERS[i].name[..], name_str.as_bytes()) == 0 {
            if !pass_str.is_empty() {
                if crate::system::util::lumie_strcmp_raw(&G_USERS[i].pass[..], pass_str.as_bytes()) != 0 {
                    return -1;
                }
            }
            G_LOGGED_IN = i as i32;
            let nlen = crate::system::util::lumie_strlen_raw(&G_USERS[i].name);
            if nlen < USER_NAME_LEN {
                G_CURRENT_NAME[..nlen].copy_from_slice(&G_USERS[i].name[..nlen]);
                G_CURRENT_NAME[nlen] = 0;
            }
            G_CURRENT_ROLE = G_USERS[i].role;
            G_LOGGED_IN_FLAG = true;
            return i as i32;
        }
    }
    -1
}

pub unsafe fn users_logout() {
    G_LOGGED_IN = -1;
    G_LOGGED_IN_FLAG = false;
    G_CURRENT_NAME[0] = 0;
    G_CURRENT_ROLE = 0;
}

pub unsafe fn users_is_logged_in() -> bool {
    G_LOGGED_IN_FLAG
}

pub unsafe fn users_current_role() -> i32 {
    G_CURRENT_ROLE
}

pub unsafe fn users_current_name(buf: *mut u8) -> i32 {
    if buf.is_null() || !G_LOGGED_IN_FLAG {
        return -1;
    }
    let mut i = 0;
    while i < USER_NAME_LEN && G_CURRENT_NAME[i] != 0 {
        *buf.add(i) = G_CURRENT_NAME[i];
        i += 1;
    }
    *buf.add(i) = 0;
    i as i32
}

pub unsafe fn users_is_protected_path(path: *const u8) -> bool {
    if path.is_null() {
        return false;
    }
    let p = crate::system::util::lumie_str_from_ptr(path);
    p.starts_with("/system/")
        || p.starts_with("/drivers/")
        || p == "/system"
        || p == "/drivers"
        || p == "/EFI"
        || p.starts_with("/EFI/")
}

pub unsafe fn users_add(name: *const u8, pass: *const u8, role: i32) -> i32 {
    let name_str = crate::system::util::lumie_str_from_ptr(name);
    if name_str.is_empty() {
        return -1;
    }
    if (G_USER_COUNT as usize) >= MAX_USERS {
        return -1;
    }
    let nlen = name_str.len();
    if nlen >= USER_NAME_LEN {
        return -1;
    }
    for i in 0..(G_USER_COUNT as usize) {
        if crate::system::util::lumie_strcmp_raw(&G_USERS[i].name[..], name_str.as_bytes()) == 0 {
            return -1;
        }
    }
    let idx = G_USER_COUNT as usize;
    G_USERS[idx].name[..nlen].copy_from_slice(name_str.as_bytes());
    G_USERS[idx].name[nlen] = 0;
    if !pass.is_null() {
        let pass_str = crate::system::util::lumie_str_from_ptr(pass);
        let plen = pass_str.len();
        let plen = if plen >= USER_NAME_LEN {
            USER_NAME_LEN - 1
        } else {
            plen
        };
        G_USERS[idx].pass[..plen].copy_from_slice(pass_str.as_bytes());
        G_USERS[idx].pass[plen] = 0;
    } else {
        G_USERS[idx].pass[0] = 0;
    }
    G_USERS[idx].role = role;
    G_USER_COUNT += 1;
    save_users()
}

pub unsafe fn users_remove(name: *const u8) -> i32 {
    let name_str = crate::system::util::lumie_str_from_ptr(name);
    if name_str.is_empty() {
        return -1;
    }
    for i in 0..(G_USER_COUNT as usize) {
        if crate::system::util::lumie_strcmp_raw(&G_USERS[i].name[..], name_str.as_bytes()) == 0 {
            for j in i..(G_USER_COUNT as usize - 1) {
                G_USERS[j] = G_USERS[j + 1];
            }
            G_USER_COUNT -= 1;
            return save_users();
        }
    }
    -1
}

pub unsafe fn users_count() -> i32 {
    G_USER_COUNT
}

pub unsafe fn users_list(buf: *mut u8, max: i32) -> i32 {
    if buf.is_null() || max <= 0 {
        return 0;
    }
    let maxu = max as usize;
    let mut pos: usize = 0;
    for i in 0..(G_USER_COUNT as usize) {
        if pos >= maxu - 1 {
            break;
        }
        let nlen = crate::system::util::lumie_strlen_raw(&G_USERS[i].name);
        let space = maxu - 1 - pos;
        if nlen + 8 > space {
            break;
        }
        let slice = core::slice::from_raw_parts_mut(buf.add(pos), maxu - pos);
        slice[..nlen].copy_from_slice(&G_USERS[i].name[..nlen]);
        pos += nlen;
        buf.add(pos).write(b' ');
        pos += 1;
        if G_USERS[i].role == USER_ROLE_ADMIN {
            let admin = b"(admin)";
            let slice2 = core::slice::from_raw_parts_mut(buf.add(pos), maxu - pos);
            slice2[..admin.len()].copy_from_slice(admin);
            pos += admin.len();
        } else {
            let user = b"(user)";
            let slice2 = core::slice::from_raw_parts_mut(buf.add(pos), maxu - pos);
            slice2[..user.len()].copy_from_slice(user);
            pos += user.len();
        }
        buf.add(pos).write(b'\n');
        pos += 1;
    }
    if pos < maxu {
        buf.add(pos).write(0);
    }
    pos as i32
}
