use core::ptr;

/* SHA-256 */
#[inline(always)]
fn rr(x: u32, n: u32) -> u32 { (x >> n) | (x << (32 - n)) }
#[inline(always)]
fn ep0(x: u32) -> u32 { rr(x, 2) ^ rr(x, 13) ^ rr(x, 22) }
#[inline(always)]
fn ep1(x: u32) -> u32 { rr(x, 6) ^ rr(x, 11) ^ rr(x, 25) }
#[inline(always)]
fn sg0(x: u32) -> u32 { rr(x, 7) ^ rr(x, 18) ^ (x >> 3) }
#[inline(always)]
fn sg1(x: u32) -> u32 { rr(x, 17) ^ rr(x, 19) ^ (x >> 10) }

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34bcb0b5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[repr(C)]
struct Sha2Ctx {
    s: [u32; 8],
    cnt: u64,
    buf: [u8; 64],
}

fn sha2_init(c: &mut Sha2Ctx) {
    c.s = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    c.cnt = 0;
}

fn sha2_blk(s: &mut [u32; 8], b: &[u8]) {
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = (b[4 * i] as u32) << 24
            | (b[4 * i + 1] as u32) << 16
            | (b[4 * i + 2] as u32) << 8
            | b[4 * i + 3] as u32;
    }
    for i in 16..64 {
        w[i] = sg1(w[i - 2]).wrapping_add(w[i - 7]).wrapping_add(sg0(w[i - 15])).wrapping_add(w[i - 16]);
    }
    let (mut a, mut b0, mut c, mut d, mut e, mut f, mut g, mut h) = (s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]);
    for i in 0..64 {
        let t1 = h.wrapping_add(ep1(e)).wrapping_add((e & f) ^ ((!e) & g)).wrapping_add(K256[i]).wrapping_add(w[i]);
        let t2 = ep0(a).wrapping_add((a & b0) ^ (a & c) ^ (b0 & c));
        h = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b0; b0 = a; a = t1.wrapping_add(t2);
    }
    s[0] = s[0].wrapping_add(a);
    s[1] = s[1].wrapping_add(b0);
    s[2] = s[2].wrapping_add(c);
    s[3] = s[3].wrapping_add(d);
    s[4] = s[4].wrapping_add(e);
    s[5] = s[5].wrapping_add(f);
    s[6] = s[6].wrapping_add(g);
    s[7] = s[7].wrapping_add(h);
}

fn sha2_upd(c: &mut Sha2Ctx, d: *const u8, n: u32) {
    let mut p = d;
    let mut remaining = n as usize;
    let idx = (c.cnt & 63) as usize;
    c.cnt += n as u64;

    if remaining >= 64 - idx {
        unsafe {
            ptr::copy_nonoverlapping(p, c.buf.as_mut_ptr().add(idx), 64 - idx);
        }
        sha2_blk(&mut c.s, &c.buf);
        p = unsafe { p.add(64 - idx) };
        remaining -= 64 - idx;
        while remaining >= 64 {
            let block = unsafe { core::slice::from_raw_parts(p, 64) };
            sha2_blk(&mut c.s, block);
            p = unsafe { p.add(64) };
            remaining -= 64;
        }
        let idx2 = 0;
        unsafe {
            ptr::copy_nonoverlapping(p, c.buf.as_mut_ptr().add(idx2), remaining);
        }
    } else {
        unsafe {
            ptr::copy_nonoverlapping(p, c.buf.as_mut_ptr().add(idx), remaining);
        }
    }
}

fn sha2_done(c: &mut Sha2Ctx, d: *mut u8) {
    let bits = c.cnt << 3;
    let idx = (c.cnt & 63) as usize;
    c.buf[idx] = 0x80;
    if idx > 56 {
        for i in (idx + 1)..64 { c.buf[i] = 0; }
        sha2_blk(&mut c.s, &c.buf);
        c.buf.fill(0);
    } else {
        for i in (idx + 1)..56 { c.buf[i] = 0; }
    }
    for i in 0..8 {
        c.buf[56 + i] = (bits >> (56 - 8 * i)) as u8;
    }
    sha2_blk(&mut c.s, &c.buf);
    unsafe {
        for i in 0..8 {
            *d.add(4 * i) = (c.s[i] >> 24) as u8;
            *d.add(4 * i + 1) = (c.s[i] >> 16) as u8;
            *d.add(4 * i + 2) = (c.s[i] >> 8) as u8;
            *d.add(4 * i + 3) = c.s[i] as u8;
        }
    }
}

unsafe fn sha256(d: *const u8, n: u32, h: *mut u8) {
    let mut c: Sha2Ctx = core::mem::zeroed();
    sha2_init(&mut c);
    sha2_upd(&mut c, d, n);
    sha2_done(&mut c, h);
}

/* HMAC-SHA256 */
unsafe fn hmac_sha256(k: *const u8, kl: u32, d: *const u8, dl: u32, m: *mut u8) {
    let mut tk = [0u8; 32];
    let (k_ptr, k_len) = if kl > 64 {
        sha256(k, kl, tk.as_mut_ptr());
        (tk.as_ptr(), 32u32)
    } else {
        (k, kl)
    };
    let mut k_ip = [0u8; 64];
    let mut k_op = [0u8; 64];
    ptr::copy_nonoverlapping(k_ptr, k_ip.as_mut_ptr(), k_len as usize);
    ptr::copy_nonoverlapping(k_ptr, k_op.as_mut_ptr(), k_len as usize);
    for i in 0..64 {
        k_ip[i] ^= 0x36;
        k_op[i] ^= 0x5c;
    }
    let mut c: Sha2Ctx = core::mem::zeroed();
    sha2_init(&mut c);
    sha2_upd(&mut c, k_ip.as_ptr(), 64);
    sha2_upd(&mut c, d, dl);
    sha2_done(&mut c, m);

    sha2_init(&mut c);
    sha2_upd(&mut c, k_op.as_ptr(), 64);
    sha2_upd(&mut c, m, 32);
    sha2_done(&mut c, m);
}

/* AES-128-CBC */
const SB: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

fn aes_keyexp(k: &[u8], rk: &mut [u8]) {
    let w = unsafe { &mut *(rk.as_mut_ptr() as *mut [u32; 44]) };
    for i in 0..4 {
        w[i] = (k[4 * i] as u32) << 24 | (k[4 * i + 1] as u32) << 16 | (k[4 * i + 2] as u32) << 8 | k[4 * i + 3] as u32;
    }
    for i in 4..44 {
        let mut t = w[i - 1];
        if i % 4 == 0 {
            t = (t << 8) | (t >> 24);
            t = (SB[(t >> 24) as usize] as u32) << 24
                | (SB[((t >> 16) & 0xFF) as usize] as u32) << 16
                | (SB[((t >> 8) & 0xFF) as usize] as u32) << 8
                | SB[(t & 0xFF) as usize] as u32;
            t ^= 1u32 << ((i / 4) - 1);
        }
        w[i] = w[i - 4] ^ t;
    }
}

fn aes_enc(inp: &[u8], out: &mut [u8], rk: &[u8]) {
    let kp = unsafe { &*(rk.as_ptr() as *const [u32; 44]) };
    let mut s = [0u32; 4];
    for i in 0..4 {
        s[i] = ((inp[4 * i] as u32) << 24 | (inp[4 * i + 1] as u32) << 16 | (inp[4 * i + 2] as u32) << 8 | inp[4 * i + 3] as u32) ^ kp[i];
    }
    for r in 1..=10 {
        let mut t = [0u32; 4];
        for i in 0..4 {
            let x = s[(i + 0) % 4];
            t[i] = (SB[(x >> 24) as usize] as u32) << 24
                | (SB[((x >> 16) & 0xFF) as usize] as u32) << 16
                | (SB[((x >> 8) & 0xFF) as usize] as u32) << 8
                | SB[(x & 0xFF) as usize] as u32;
        }
        if r < 10 {
            let v = [
                t[0] ^ ((t[1] >> 16) | (t[1] << 24)) ^ ((t[2] >> 8) | (t[2] << 24)) ^ t[3],
                t[1] ^ ((t[2] >> 16) | (t[2] << 24)) ^ ((t[3] >> 8) | (t[3] << 24)) ^ t[0],
                t[2] ^ ((t[3] >> 16) | (t[3] << 24)) ^ ((t[0] >> 8) | (t[0] << 24)) ^ t[1],
                t[3] ^ ((t[0] >> 16) | (t[0] << 24)) ^ ((t[1] >> 8) | (t[1] << 24)) ^ t[2],
            ];
            t = v;
        }
        for i in 0..4 { s[i] = t[i] ^ kp[4 * r + i]; }
    }
    for i in 0..4 {
        out[4 * i] = (s[i] >> 24) as u8;
        out[4 * i + 1] = (s[i] >> 16) as u8;
        out[4 * i + 2] = (s[i] >> 8) as u8;
        out[4 * i + 3] = s[i] as u8;
    }
}

fn aes_cbc_enc(k: &[u8], iv: &[u8], inp: &[u8], out: &mut [u8]) {
    let mut rk = [0u8; 176];
    aes_keyexp(k, &mut rk);
    let mut c = [0u8; 16];
    c.copy_from_slice(iv);
    for i in (0..inp.len()).step_by(16) {
        for j in 0..16 { c[j] ^= inp[i + j]; }
        aes_enc(&c, &mut out[i..], &rk);
        c.copy_from_slice(&out[i..i + 16]);
    }
}

/* Public API */
pub unsafe fn tls_init() {
}

pub unsafe fn tls_verify_signature(data: *const u8, len: u32, sig: *const u8) -> i32 {
    if data.is_null() || sig.is_null() {
        return -1;
    }
    let mut hash: [u8; 32] = [0u8; 32];
    sha256(data, len, hash.as_mut_ptr());
    let mut verify: [u8; 32] = [0u8; 32];
    sha256(sig, 32, verify.as_mut_ptr());
    if hash == verify { 0 } else { -1 }
}
