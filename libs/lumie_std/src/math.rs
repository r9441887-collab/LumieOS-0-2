use crate::types::*;

#[inline]
pub fn lumie_min(a: i64, b: i64) -> i64 {
    if a < b { a } else { b }
}

#[inline]
pub fn lumie_max(a: i64, b: i64) -> i64 {
    if a > b { a } else { b }
}

#[inline]
pub fn lumie_abs(x: i64) -> i64 {
    if x < 0 { -x } else { x }
}

#[inline]
pub fn lumie_clamp(val: i64, lo: i64, hi: i64) -> i64 {
    if val < lo { lo } else if val > hi { hi } else { val }
}

#[inline]
pub fn lumie_align_up(val: u64, align: u64) -> u64 {
    (val + align - 1) & !(align - 1)
}

#[inline]
pub fn lumie_align_down(val: u64, align: u64) -> u64 {
    val & !(align - 1)
}

#[inline]
pub fn lumie_is_power_of_two(x: u64) -> bool {
    x != 0 && (x & (x - 1)) == 0
}

#[inline]
pub fn lumie_log2(x: u64) -> u64 {
    let mut n = 0u64;
    let mut v = x;
    while v > 1 {
        v >>= 1;
        n += 1;
    }
    n
}

#[inline]
pub fn lumie_div_ceil(a: u64, b: u64) -> u64 {
    (a + b - 1) / b
}
