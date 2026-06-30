use crate::types::*;

const DIGITS: &[u8; 16] = b"0123456789abcdef";

pub unsafe fn lumie_itoa(num: i64, buf: *mut u8, base: i32) -> isize {
    if num == 0 {
        *buf = b'0';
        *buf.add(1) = 0;
        return 1;
    }

    let mut tmp: [u8; 65] = [0u8; 65];
    let mut ti: usize = 0;
    let neg: bool;
    let unum: u64;

    if num < 0 {
        neg = true;
        if base == 10 {
            if num == i64::MIN {
                let s = b"-9223372036854775808\0";
                let mut i = 0;
                while s[i] != 0 {
                    *buf.add(i) = s[i];
                    i += 1;
                }
                *buf.add(i) = 0;
                return i as isize;
            }
            unum = (-num) as u64;
        } else {
            unum = (num as u64).wrapping_neg();
        }
    } else {
        neg = false;
        unum = num as u64;
    }

    let mut x = unum;
    while x > 0 {
        tmp[ti] = DIGITS[(x % base as u64) as usize];
        ti += 1;
        x /= base as u64;
    }

    let mut j: usize = 0;
    if neg {
        *buf.add(j) = b'-';
        j += 1;
    }
    while ti > 0 {
        ti -= 1;
        *buf.add(j) = tmp[ti];
        j += 1;
    }
    *buf.add(j) = 0;
    j as isize
}

pub unsafe fn lumie_atoi(s: *const u8) -> i64 {
    let mut p: usize = 0;
    while *s.add(p) == b' ' {
        p += 1;
    }
    let neg = if *s.add(p) == b'-' {
        p += 1;
        true
    } else {
        if *s.add(p) == b'+' {
            p += 1;
        }
        false
    };
    let mut result: i64 = 0;
    while *s.add(p) >= b'0' && *s.add(p) <= b'9' {
        result = result.wrapping_mul(10).wrapping_add((*s.add(p) - b'0') as i64);
        p += 1;
    }
    if neg { -result } else { result }
}

pub struct LumieFmt<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> LumieFmt<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        LumieFmt { buf, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn as_str(&self) -> &[u8] {
        &self.buf[..self.pos]
    }

    fn write_byte(&mut self, b: u8) {
        if self.pos < self.buf.len() {
            self.buf[self.pos] = b;
            self.pos += 1;
        }
    }

    pub fn write_str(&mut self, s: &[u8]) {
        for &b in s {
            self.write_byte(b);
        }
    }

    pub fn write_i64(&mut self, v: i64) {
        let mut tmp: [u8; 32] = [0u8; 32];
        unsafe {
            lumie_itoa(v, tmp.as_mut_ptr(), 10);
        }
        let mut i = 0;
        while i < 32 && tmp[i] != 0 {
            self.write_byte(tmp[i]);
            i += 1;
        }
    }

    pub fn write_u32_hex(&mut self, v: u32) {
        let mut tmp: [u8; 32] = [0u8; 32];
        unsafe {
            lumie_itoa(v as i64, tmp.as_mut_ptr(), 16);
        }
        let mut i = 0;
        while i < 32 && tmp[i] != 0 {
            self.write_byte(tmp[i]);
            i += 1;
        }
    }

    pub fn write_u32(&mut self, v: u32) {
        let mut tmp: [u8; 32] = [0u8; 32];
        unsafe {
            lumie_itoa(v as i64, tmp.as_mut_ptr(), 10);
        }
        let mut i = 0;
        while i < 32 && tmp[i] != 0 {
            self.write_byte(tmp[i]);
            i += 1;
        }
    }

    pub fn write_char(&mut self, c: u8) {
        self.write_byte(c);
    }

    pub fn finish(self) -> usize {
        if self.pos < self.buf.len() {
            self.buf[self.pos] = 0;
        }
        self.pos
    }
}

pub unsafe fn lumie_snprintf(buf: *mut u8, sz: size_t, fmt: *const u8) -> isize {
    let max = if sz > 0 { sz - 1 } else { 0 };
    let mut pos: usize = 0;
    let mut fi: usize = 0;

    while *fmt.add(fi) != 0 && pos < max {
        if *fmt.add(fi) != b'%' {
            *buf.add(pos) = *fmt.add(fi);
            pos += 1;
            fi += 1;
            continue;
        }
        fi += 1;
        if *fmt.add(fi) == 0 {
            break;
        }
        match *fmt.add(fi) {
            b'%' => {
                *buf.add(pos) = b'%';
                pos += 1;
            }
            _ => {
                *buf.add(pos) = b'%';
                pos += 1;
                if pos < max {
                    *buf.add(pos) = *fmt.add(fi);
                    pos += 1;
                }
            }
        }
        fi += 1;
    }
    if pos < sz {
        *buf.add(pos) = 0;
    }
    pos as isize
}
