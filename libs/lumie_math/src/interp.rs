// ─── Scalar interpolation ────────────────────────────────────────
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

#[inline]
pub fn inverse_lerp(a: f32, b: f32, v: f32) -> f32 { (v - a) / (b - a) }

#[inline]
pub fn remap(v: f32, from_a: f32, from_b: f32, to_a: f32, to_b: f32) -> f32 {
    lerp(to_a, to_b, inverse_lerp(from_a, from_b, v))
}

#[inline]
pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 { if v < lo { lo } else if v > hi { hi } else { v } }

#[inline]
pub fn saturate(v: f32) -> f32 { if v < 0.0 { 0.0 } else if v > 1.0 { 1.0 } else { v } }

#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = saturate((x - edge0) / (edge1 - edge0));
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn smootherstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = saturate((x - edge0) / (edge1 - edge0));
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

pub fn bezier(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    a * mt3 + b * 3.0 * mt2 * t + c * 3.0 * mt * t2 + d * t3
}

pub fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    0.5 * ((2.0 * p1) + (-p0 + p2) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t)
}

// ─── Color blending ──────────────────────────────────────────────
#[inline]
pub fn blend_pixel_over(src: u32, dst: u32) -> u32 {
    let sa = (src >> 24) & 0xFF;
    if sa == 0xFF { return src; }
    if sa == 0 { return dst; }
    let sr = (src >> 16) & 0xFF;
    let sg = (src >> 8) & 0xFF;
    let sb = src & 0xFF;
    let da = (dst >> 24) & 0xFF;
    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8) & 0xFF;
    let db = dst & 0xFF;
    let out_a = sa + da * (255 - sa) / 255;
    let out_r = (sr * sa + dr * (255 - sa)) / 255;
    let out_g = (sg * sa + dg * (255 - sa)) / 255;
    let out_b = (sb * sa + db * (255 - sa)) / 255;
    ((out_a as u32) << 24) | ((out_r as u32) << 16) | ((out_g as u32) << 8) | out_b as u32
}

#[inline]
pub fn blend_pixel_add(src: u32, dst: u32) -> u32 {
    let sr = (src >> 16) & 0xFF;
    let sg = (src >> 8) & 0xFF;
    let sb = src & 0xFF;
    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8) & 0xFF;
    let db = dst & 0xFF;
    let out_r = (sr + dr).min(255);
    let out_g = (sg + dg).min(255);
    let out_b = (sb + db).min(255);
    0xFF000000 | ((out_r as u32) << 16) | ((out_g as u32) << 8) | out_b as u32
}

// ─── Color construction ──────────────────────────────────────────
#[inline]
pub const fn argb(a: u8, r: u8, g: u8, b: u8) -> u32 {
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

#[inline]
pub const fn rgb(r: u8, g: u8, b: u8) -> u32 { 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | b as u32 }
