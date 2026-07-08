use core::f32::consts::PI;

#[inline]
pub fn sqrt_f32(x: f32) -> f32 {
    if x <= 0.0 { return 0.0; }
    let bits = x.to_bits();
    let mut r = f32::from_bits((bits >> 1).wrapping_add(0x1FC00000));
    r = (r + x / r) * 0.5;
    r = (r + x / r) * 0.5;
    r
}

#[inline]
pub fn sin_f32(x: f32) -> f32 {
    let mut x = x % (2.0 * PI);
    if x > PI { x -= 2.0 * PI; }
    if x < -PI { x += 2.0 * PI; }
    let s = x * x;
    let mut r = x;
    let mut t = x;
    t *= -s / (2.0 * 3.0); r += t;
    t *= -s / (4.0 * 5.0); r += t;
    t *= -s / (6.0 * 7.0); r += t;
    t *= -s / (8.0 * 9.0); r += t;
    t *= -s / (10.0 * 11.0); r += t;
    r
}

#[inline]
pub fn cos_f32(x: f32) -> f32 {
    sin_f32(x + PI / 2.0)
}

#[inline]
pub fn tan_f32(x: f32) -> f32 {
    sin_f32(x) / cos_f32(x)
}

#[inline]
pub fn atan2_f32(y: f32, x: f32) -> f32 {
    if x == 0.0 {
        return if y > 0.0 { PI / 2.0 } else if y < 0.0 { -PI / 2.0 } else { 0.0 };
    }
    let a = (y / x).abs();
    let mut r = a - a * a * a * (1.0 / 3.0) + a * a * a * a * a * (1.0 / 5.0);
    r -= a * a * a * a * a * a * a * (1.0 / 7.0);
    if x < 0.0 { r = PI - r; }
    if y < 0.0 { r = -r; }
    r
}

#[inline]
pub fn acos_f32(x: f32) -> f32 {
    atan2_f32(sqrt_f32(1.0 - x * x), x)
}

pub fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}
