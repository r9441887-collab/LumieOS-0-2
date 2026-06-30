use crate::types::*;

#[inline]
pub unsafe fn lumie_strlen(str: *const u8) -> size_t {
    let mut len: size_t = 0;
    while *str.add(len) != 0 {
        len += 1;
    }
    len
}

#[inline]
pub unsafe fn lumie_strcmp(s1: *const u8, s2: *const u8) -> i32 {
    let mut i: size_t = 0;
    while *s1.add(i) != 0 && *s1.add(i) == *s2.add(i) {
        i += 1;
    }
    (*s1.add(i) as i32) - (*s2.add(i) as i32)
}

#[inline]
pub unsafe fn lumie_strncmp(s1: *const u8, s2: *const u8, n: size_t) -> i32 {
    let mut i: size_t = 0;
    while i < n && *s1.add(i) != 0 && *s1.add(i) == *s2.add(i) {
        i += 1;
    }
    if i == n {
        return 0;
    }
    (*s1.add(i) as i32) - (*s2.add(i) as i32)
}

#[inline]
pub unsafe fn lumie_strcpy(dest: *mut u8, src: *const u8) -> *mut u8 {
    let mut i: size_t = 0;
    loop {
        let c = *src.add(i);
        *dest.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    dest
}

#[inline]
pub unsafe fn lumie_strcat(dest: *mut u8, src: *const u8) -> *mut u8 {
    let mut i: size_t = 0;
    while *dest.add(i) != 0 {
        i += 1;
    }
    let mut j: size_t = 0;
    loop {
        let c = *src.add(j);
        *dest.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
        j += 1;
    }
    dest
}

#[inline]
pub unsafe fn lumie_strchr(str: *const u8, ch: i32) -> *mut u8 {
    let mut i: size_t = 0;
    while *str.add(i) != 0 {
        if *str.add(i) == ch as u8 {
            return str.add(i) as *mut u8;
        }
        i += 1;
    }
    core::ptr::null_mut()
}

#[inline]
pub unsafe fn lumie_strstr(haystack: *const u8, needle: *const u8) -> *mut u8 {
    if *needle == 0 {
        return haystack as *mut u8;
    }
    let mut hi: size_t = 0;
    while *haystack.add(hi) != 0 {
        let mut h: size_t = hi;
        let mut ni: size_t = 0;
        while *haystack.add(h) != 0 && *needle.add(ni) != 0 && *haystack.add(h) == *needle.add(ni) {
            h += 1;
            ni += 1;
        }
        if *needle.add(ni) == 0 {
            return haystack.add(hi) as *mut u8;
        }
        hi += 1;
    }
    core::ptr::null_mut()
}

#[inline]
pub unsafe fn lumie_memset(ptr: *mut u8, val: i32, num: size_t) -> *mut u8 {
    for i in 0..num {
        *ptr.add(i) = val as u8;
    }
    ptr
}

#[inline]
pub unsafe fn lumie_memcpy(dest: *mut u8, src: *const u8, num: size_t) -> *mut u8 {
    for i in 0..num {
        *dest.add(i) = *src.add(i);
    }
    dest
}

#[inline]
pub unsafe fn lumie_memmove(dest: *mut u8, src: *const u8, num: size_t) -> *mut u8 {
    if dest < src as *mut u8 {
        for i in 0..num {
            *dest.add(i) = *src.add(i);
        }
    } else {
        let mut i = num;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}

#[inline]
pub unsafe fn lumie_memcmp(p1: *const u8, p2: *const u8, num: size_t) -> i32 {
    for i in 0..num {
        let a = *p1.add(i);
        let b = *p2.add(i);
        if a != b {
            return a as i32 - b as i32;
        }
    }
    0
}
