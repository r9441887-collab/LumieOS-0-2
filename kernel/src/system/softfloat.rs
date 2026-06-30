/* IEEE 754 single-precision soft-float helpers */

union F32Bits {
    u: u32,
    f: f32,
}

#[inline(always)]
fn f32_sign(f: u32) -> u32 { f >> 31 }

#[inline(always)]
fn f32_exp(f: u32) -> u32 { (f >> 23) & 0xFF }

#[inline(always)]
fn f32_mant(f: u32) -> u32 { f & 0x7FFFFF }

#[inline(always)]
fn f32_is_nan(f: u32) -> bool { f32_exp(f) == 0xFF && f32_mant(f) != 0 }

#[inline(always)]
fn f32_is_inf(f: u32) -> bool { f32_exp(f) == 0xFF && f32_mant(f) == 0 }

#[inline(always)]
fn f32_is_zero(f: u32) -> bool { f32_exp(f) == 0 && f32_mant(f) == 0 }

fn f32_round_pack(mut sig: u32, mut exp: i32, sign: u32) -> u32 {
    sig = sig.wrapping_add(0x40);
    if sig & 0x1000000 != 0 {
        sig >>= 1;
        exp += 1;
    }
    if exp >= 0xFF {
        return (sign << 31) | 0x7F800000;
    }
    if exp <= 0 {
        let rsh = 1 - exp;
        if rsh > 31 {
            return sign << 31;
        }
        sig = (sig + (1u32 << (rsh - 1) as u32)) >> rsh;
        exp = 0;
    }
    (sign << 31) | ((exp as u32) << 23) | (sig & 0x7FFFFF)
}

pub fn f32_add(a: f32, b: f32) -> f32 {
    let ua = unsafe { F32Bits { f: a }.u };
    let ub = unsafe { F32Bits { f: b }.u };
    if f32_is_nan(ua) { return a; }
    if f32_is_nan(ub) { return b; }

    let sa = f32_sign(ua);
    let sb = f32_sign(ub);
    let mut ea = f32_exp(ua) as i32;
    let mut eb = f32_exp(ub) as i32;
    let mut ma = f32_mant(ua);
    let mut mb = f32_mant(ub);

    if ea == 0 && ma == 0 {
        if sb == 0 { return b; }
        if sa != 0 { return a; }
    }
    if eb == 0 && mb == 0 { return a; }

    ma |= 0x800000;
    mb |= 0x800000;
    let mut sign = sa;

    if ea < eb || (ea == eb && ma < mb) {
        core::mem::swap(&mut ma, &mut mb);
        core::mem::swap(&mut ea, &mut eb);
        core::mem::swap(&mut sign, &mut sb);
        sign = sa;
    }

    let rsh = ea - eb;
    if rsh > 25 { mb = 0; } else { mb >>= rsh as u32; }

    let sig;
    if sa == sb {
        sig = ma + mb;
        if sig & 0x1000000 != 0 { sig >>= 1; ea += 1; }
    } else {
        if ma < mb {
            core::mem::swap(&mut ma, &mut mb);
            sign = sa;
        }
        sig = ma - mb;
        if sig == 0 { return 0.0; }
        while sig & 0x800000 == 0 { sig <<= 1; ea -= 1; }
    }

    let r = f32_round_pack(sig, ea, sign);
    unsafe { F32Bits { u: r }.f }
}

pub fn f32_sub(a: f32, b: f32) -> f32 {
    let ub = unsafe { F32Bits { f: b }.u ^ 0x80000000 };
    f32_add(a, unsafe { F32Bits { u: ub }.f })
}

pub fn f32_mul(a: f32, b: f32) -> f32 {
    let ua = unsafe { F32Bits { f: a }.u };
    let ub = unsafe { F32Bits { f: b }.u };
    if f32_is_nan(ua) { return a; }
    if f32_is_nan(ub) { return b; }

    let sign = f32_sign(ua) ^ f32_sign(ub);
    let ea = f32_exp(ua) as i32;
    let eb = f32_exp(ub) as i32;
    let ma = f32_mant(ua) | 0x800000;
    let mb = f32_mant(ub) | 0x800000;

    if f32_is_inf(ua) || f32_is_inf(ub) {
        return unsafe { F32Bits { u: (sign << 31) | 0x7F800000 }.f };
    }
    if f32_is_zero(ua) || f32_is_zero(ub) {
        return unsafe { F32Bits { u: sign << 31 }.f };
    }

    let mul = (ma as u64) * (mb as u64);
    let mut sig = (mul >> 23) as u32;
    let mut exp = ea + eb - 127;
    if sig & 0x1000000 != 0 { sig >>= 1; exp += 1; }

    let r = f32_round_pack(sig, exp, sign);
    unsafe { F32Bits { u: r }.f }
}

pub fn f32_div(a: f32, b: f32) -> f32 {
    let ua = unsafe { F32Bits { f: a }.u };
    let ub = unsafe { F32Bits { f: b }.u };
    if f32_is_nan(ua) { return a; }
    if f32_is_nan(ub) { return b; }

    let sign = f32_sign(ua) ^ f32_sign(ub);
    let ea = f32_exp(ua) as i32;
    let eb = f32_exp(ub) as i32;
    let ma = f32_mant(ua) | 0x800000;
    let mb = f32_mant(ub) | 0x800000;

    if f32_is_inf(ua) && f32_is_inf(ub) {
        return unsafe { F32Bits { u: (sign << 31) | 0x7FC00000 }.f };
    }
    if f32_is_zero(ub) {
        return unsafe { F32Bits { u: (sign << 31) | 0x7F800000 }.f };
    }
    if f32_is_zero(ua) {
        return unsafe { F32Bits { u: sign << 31 }.f };
    }
    if f32_is_inf(ua) {
        return unsafe { F32Bits { u: (sign << 31) | 0x7F800000 }.f };
    }

    let dividend = (ma as u64) << 31;
    let mut sig = (dividend / mb as u64) as u32;
    let mut exp = ea - eb + 127;
    if sig & 0x1000000 != 0 { sig >>= 1; exp += 1; }

    let r = f32_round_pack(sig, exp, sign);
    unsafe { F32Bits { u: r }.f }
}

pub fn f32_neg(a: f32) -> f32 {
    unsafe { F32Bits { u: F32Bits { f: a }.u ^ 0x80000000 }.f }
}

pub fn f32_eq(a: f32, b: f32) -> i32 {
    let ua = unsafe { F32Bits { f: a }.u };
    let ub = unsafe { F32Bits { f: b }.u };
    if f32_is_nan(ua) || f32_is_nan(ub) { return 1; }
    if ua == ub { 0 } else { 1 }
}

pub fn f32_lt(a: f32, b: f32) -> i32 {
    let ua = unsafe { F32Bits { f: a }.u };
    let ub = unsafe { F32Bits { f: b }.u };
    if f32_is_nan(ua) || f32_is_nan(ub) { return 1; }
    let sa = f32_sign(ua);
    let sb = f32_sign(ub);
    if sa != 0 && sb == 0 { return -1; }
    if sa == 0 && sb != 0 { return 1; }
    if sa != 0 { (ua > ub) as i32 * -1 + (ua == ub) as i32 * 0 } else { (ua < ub) as i32 * -1 + 0 }
}

pub fn f32_le(a: f32, b: f32) -> i32 {
    let ua = unsafe { F32Bits { f: a }.u };
    let ub = unsafe { F32Bits { f: b }.u };
    if f32_is_nan(ua) || f32_is_nan(ub) { return 1; }
    let sa = f32_sign(ua);
    let sb = f32_sign(ub);
    if sa != 0 && sb == 0 { return -1; }
    if sa == 0 && sb != 0 { return 1; }
    let less = if sa != 0 { ua > ub } else { ua < ub };
    if less { -1 } else if ua == ub { 0 } else { 1 }
}

pub fn f32_gt(a: f32, b: f32) -> i32 {
    let ua = unsafe { F32Bits { f: a }.u };
    let ub = unsafe { F32Bits { f: b }.u };
    if f32_is_nan(ua) || f32_is_nan(ub) { return -1; }
    let sa = f32_sign(ua);
    let sb = f32_sign(ub);
    if sa != 0 && sb == 0 { return -1; }
    if sa == 0 && sb != 0 { return 1; }
    let greater = if sa != 0 { ua < ub } else { ua > ub };
    if greater { 1 } else { 0 }
}

pub fn f32_ge(a: f32, b: f32) -> i32 {
    let ua = unsafe { F32Bits { f: a }.u };
    let ub = unsafe { F32Bits { f: b }.u };
    if f32_is_nan(ua) || f32_is_nan(ub) { return -1; }
    let sa = f32_sign(ua);
    let sb = f32_sign(ub);
    if sa != 0 && sb == 0 { return -1; }
    if sa == 0 && sb != 0 { return 1; }
    let greater = if sa != 0 { ua < ub } else { ua > ub };
    if greater { 1 } else if ua == ub { 0 } else { -1 }
}

pub fn f32_ne(a: f32, b: f32) -> i32 {
    f32_eq(a, b)
}

pub fn f32_from_i32(i: i32) -> f32 {
    if i == 0 { return 0.0; }
    let (sign, mut u): (u32, u32) = if i < 0 { (1, (-(i as i64)) as u32) } else { (0, i as u32) };
    let mut exp: i32 = 127 + 23;
    while u & 0x80000000 == 0 { u <<= 1; exp -= 1; }
    unsafe { F32Bits { u: (sign << 31) | ((exp as u32) << 23) | ((u >> 8) & 0x7FFFFF) }.f }
}

pub fn f32_from_u32(u: u32) -> f32 {
    if u == 0 { return 0.0; }
    let mut u2 = u;
    let mut exp: i32 = 127 + 23;
    while u2 & 0x80000000 == 0 { u2 <<= 1; exp -= 1; }
    unsafe { F32Bits { u: ((exp as u32) << 23) | ((u2 >> 8) & 0x7FFFFF) }.f }
}

pub fn f32_to_i32(a: f32) -> i32 {
    let u = unsafe { F32Bits { f: a }.u };
    if f32_is_nan(u) || f32_is_zero(u) { return 0; }
    let sign = f32_sign(u);
    let exp = f32_exp(u) as i32 - 127;
    let mant = f32_mant(u) | 0x800000;
    if exp > 30 {
        return if sign != 0 { 0x80000000i32 } else { 0x7FFFFFFFi32 };
    }
    if exp < 0 { return 0; }
    let val = (mant << 6) >> (23 - exp) as u32;
    if sign != 0 { -(val as i32) } else { val as i32 }
}

pub fn f32_to_u32(a: f32) -> u32 {
    let u = unsafe { F32Bits { f: a }.u };
    if f32_is_nan(u) || f32_sign(u) != 0 { return 0; }
    let exp = f32_exp(u) as i32 - 127;
    let mant = f32_mant(u) | 0x800000;
    if exp > 31 { return 0xFFFFFFFF; }
    if exp < 0 { return 0; }
    (mant << 6) >> (23 - exp) as u32
}

/* Software sqrt */
pub fn sqrtf(x: f32) -> f32 {
    if x <= 0.0 { return 0.0; }
    let mut guess = x;
    for _ in 0..20 {
        let next = (guess + x / guess) * 0.5;
        if guess == next { break; }
        guess = next;
    }
    guess
}

pub fn fabsf(x: f32) -> f32 {
    unsafe { F32Bits { u: F32Bits { f: x }.u & 0x7FFFFFFF }.f }
}

/* Minimal sin/cos/tan via Taylor series */
fn sin_poly(x: f32) -> f32 {
    let x2 = x * x;
    let mut s = x;
    let mut term = x;
    term *= x2 / (2.0 * 3.0);   s += term;
    term *= x2 / (4.0 * 5.0);   s -= term;
    term *= x2 / (6.0 * 7.0);   s += term;
    term *= x2 / (8.0 * 9.0);   s -= term;
    term *= x2 / (10.0 * 11.0); s += term;
    s
}

pub fn sinf(mut x: f32) -> f32 {
    let pi = 3.14159265358979323846_f32;
    let two_pi = 2.0 * pi;
    if x < 0.0 {
        let n = ((-x) / two_pi) as i32 + 1;
        x += n as f32 * two_pi;
    }
    if x > two_pi {
        let n = (x / two_pi) as i32;
        x -= n as f32 * two_pi;
    }
    let mut negate = false;
    if x > pi { x -= two_pi; }
    if x > pi / 2.0 { x = pi - x; }
    else if x < -pi / 2.0 { x = -pi - x; negate = true; }
    let r = sin_poly(x);
    if negate { -r } else { r }
}

pub fn cosf(x: f32) -> f32 {
    let pi = 3.14159265358979323846_f32;
    sinf(x + pi / 2.0)
}

pub fn tanf(x: f32) -> f32 {
    sinf(x) / cosf(x)
}
