use core::ptr;

pub unsafe fn lumie_strcmp(a: *const u8, b: *const u8) -> i32 {
    let mut i = 0;
    loop {
        let ca = *a.add(i);
        let cb = *b.add(i);
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

pub fn lumie_strcmp_raw(a: &[u8], b: &[u8]) -> i32 {
    let mut i = 0;
    loop {
        let ca = a[i];
        let cb = if i < b.len() { b[i] } else { 0 };
        if ca != cb {
            return ca as i32 - cb as i32;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

pub fn lumie_str_from_ptr(s: *const u8) -> &'static str {
    if s.is_null() {
        return "";
    }
    let mut len = 0;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
        core::str::from_utf8(core::slice::from_raw_parts(s, len)).unwrap_or("")
    }
}

pub fn lumie_str_from_raw_ptr(slice: &[u8]) -> &str {
    let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    core::str::from_utf8(&slice[..len]).unwrap_or("")
}

pub unsafe fn lumie_strlen(s: *const u8) -> u32 {
    if s.is_null() {
        return 0;
    }
    let mut len = 0;
    while *s.add(len as usize) != 0 {
        len += 1;
    }
    len
}

pub fn lumie_strlen_raw(buf: &[u8]) -> usize {
    buf.iter().position(|&c| c == 0).unwrap_or(buf.len())
}

pub unsafe fn lumie_strcpy(dst: *mut u8, src: *const u8) {
    let mut i = 0;
    loop {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
}

pub fn lumie_strlen_str(s: &str) -> u32 {
    s.len() as u32
}

/* Integer to ASCII */
pub unsafe fn lumie_itoa(mut num: i64, buf: *mut u8, base: i32) {
    let digits = b"0123456789abcdef";
    let mut tmp: [u8; 65] = [0u8; 65];
    let mut i: usize = 0;
    let neg: bool;
    let unum: u64;

    if num == 0 {
        *buf = b'0';
        *buf.add(1) = 0;
        return;
    }

    if num < 0 {
        neg = true;
        if base == 10 {
            if num == i64::MIN {
                let min_str = b"-9223372036854775808";
                let mut j = 0;
                while j < 20 {
                    *buf.add(j) = min_str[j];
                    j += 1;
                }
                *buf.add(j) = 0;
                return;
            }
            unum = (-num) as u64;
        } else {
            unum = (-(num as u64)) as u64;
        }
    } else {
        neg = false;
        unum = num as u64;
    }

    let base_u64 = base as u64;
    let mut v = unum;
    while v > 0 {
        tmp[i] = digits[(v % base_u64) as usize];
        i += 1;
        v /= base_u64;
    }

    let mut j: usize = 0;
    if neg {
        *buf.add(j) = b'-';
        j += 1;
    }
    while i > 0 {
        i -= 1;
        *buf.add(j) = tmp[i];
        j += 1;
    }
    *buf.add(j) = 0;
}

pub unsafe fn lumie_memset(ptr: *mut u8, val: i32, num: u32) {
    for i in 0..num as usize {
        *ptr.add(i) = val as u8;
    }
}

pub unsafe fn lumie_memcpy(dest: *mut u8, src: *const u8, num: u32) {
    for i in 0..num as usize {
        *dest.add(i) = *src.add(i);
    }
}
