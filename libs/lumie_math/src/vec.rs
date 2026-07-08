use core::ops::{Add, Sub, Mul, Div, Neg, Index, IndexMut};
use crate::math_ops::{sqrt_f32, sin_f32, cos_f32, tan_f32, acos_f32};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub const ONE: Vec2 = Vec2 { x: 1.0, y: 1.0 };
    pub const X: Vec2 = Vec2 { x: 1.0, y: 0.0 };
    pub const Y: Vec2 = Vec2 { x: 0.0, y: 1.0 };

    #[inline] pub const fn new(x: f32, y: f32) -> Self { Self { x, y } }
    #[inline] pub fn dot(self, b: Self) -> f32 { self.x * b.x + self.y * b.y }
    #[inline] pub fn len(self) -> f32 { sqrt_f32(self.x * self.x + self.y * self.y) }
    #[inline] pub fn len_sq(self) -> f32 { self.x * self.x + self.y * self.y }
    #[inline] pub fn norm(self) -> Self { let l = self.len(); if l > 0.0 { self / l } else { self } }
    #[inline] pub fn lerp(self, b: Self, t: f32) -> Self { self + (b - self) * t }
}

impl Add for Vec2 { type Output = Self; #[inline] fn add(self, b: Self) -> Self { Self::new(self.x + b.x, self.y + b.y) } }
impl Sub for Vec2 { type Output = Self; #[inline] fn sub(self, b: Self) -> Self { Self::new(self.x - b.x, self.y - b.y) } }
impl Mul<f32> for Vec2 { type Output = Self; #[inline] fn mul(self, s: f32) -> Self { Self::new(self.x * s, self.y * s) } }
impl Div<f32> for Vec2 { type Output = Self; #[inline] fn div(self, s: f32) -> Self { Self::new(self.x / s, self.y / s) } }
impl Neg for Vec2 { type Output = Self; #[inline] fn neg(self) -> Self { Self::new(-self.x, -self.y) } }

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Vec3 = Vec3 { x: 1.0, y: 1.0, z: 1.0 };
    pub const X: Vec3 = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
    pub const Y: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    pub const Z: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0 };

    #[inline] pub const fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    #[inline] pub fn dot(self, b: Self) -> f32 { self.x * b.x + self.y * b.y + self.z * b.z }
    #[inline] pub fn cross(self, b: Self) -> Self { Self::new(self.y * b.z - self.z * b.y, self.z * b.x - self.x * b.z, self.x * b.y - self.y * b.x) }
    #[inline] pub fn len(self) -> f32 { sqrt_f32(self.x * self.x + self.y * self.y + self.z * self.z) }
    #[inline] pub fn len_sq(self) -> f32 { self.x * self.x + self.y * self.y + self.z * self.z }
    #[inline] pub fn norm(self) -> Self { let l = self.len(); if l > 0.0 { self / l } else { self } }
    #[inline] pub fn lerp(self, b: Self, t: f32) -> Self { self + (b - self) * t }
    #[inline] pub fn reflect(self, n: Self) -> Self { self - n * 2.0 * self.dot(n) }
}

impl Add for Vec3 { type Output = Self; #[inline] fn add(self, b: Self) -> Self { Self::new(self.x + b.x, self.y + b.y, self.z + b.z) } }
impl Sub for Vec3 { type Output = Self; #[inline] fn sub(self, b: Self) -> Self { Self::new(self.x - b.x, self.y - b.y, self.z - b.z) } }
impl Mul<f32> for Vec3 { type Output = Self; #[inline] fn mul(self, s: f32) -> Self { Self::new(self.x * s, self.y * s, self.z * s) } }
impl Mul<Vec3> for Vec3 { type Output = Self; #[inline] fn mul(self, b: Self) -> Self { Self::new(self.x * b.x, self.y * b.y, self.z * b.z) } }
impl Div<f32> for Vec3 { type Output = Self; #[inline] fn div(self, s: f32) -> Self { Self::new(self.x / s, self.y / s, self.z / s) } }
impl Neg for Vec3 { type Output = Self; #[inline] fn neg(self) -> Self { Self::new(-self.x, -self.y, -self.z) } }

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub const ZERO: Vec4 = Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 0.0 };
    #[inline] pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self { Self { x, y, z, w } }
    #[inline] pub fn dot(self, b: Self) -> f32 { self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w }
    #[inline] pub fn len(self) -> f32 { sqrt_f32(self.dot(self)) }
    #[inline] pub fn lerp(self, b: Self, t: f32) -> Self { self + (b - self) * t }
}

impl Add for Vec4 { type Output = Self; #[inline] fn add(self, b: Self) -> Self { Self::new(self.x + b.x, self.y + b.y, self.z + b.z, self.w + b.w) } }
impl Sub for Vec4 { type Output = Self; #[inline] fn sub(self, b: Self) -> Self { Self::new(self.x - b.x, self.y - b.y, self.z - b.z, self.w - b.w) } }
impl Mul<f32> for Vec4 { type Output = Self; #[inline] fn mul(self, s: f32) -> Self { Self::new(self.x * s, self.y * s, self.z * s, self.w * s) } }
impl Div<f32> for Vec4 { type Output = Self; #[inline] fn div(self, s: f32) -> Self { Self::new(self.x / s, self.y / s, self.z / s, self.w / s) } }
impl Neg for Vec4 { type Output = Self; #[inline] fn neg(self) -> Self { Self::new(-self.x, -self.y, -self.z, -self.w) } }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Mat3 {
    pub m: [f32; 9],
}

impl Mat3 {
    #[inline] pub fn identity() -> Self { Self { m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] } }
    #[inline] pub fn zero() -> Self { Self { m: [0.0; 9] } }

    pub fn translate(x: f32, y: f32) -> Self { let mut m = Self::identity(); m.m[6] = x; m.m[7] = y; m }
    pub fn rotate(angle_deg: f32) -> Self {
        let rad = angle_deg * (core::f32::consts::PI / 180.0);
        let (s, c) = (sin_f32(rad), cos_f32(rad));
        Self { m: [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0] }
    }
    pub fn scale(x: f32, y: f32) -> Self { Self { m: [x, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 1.0] } }

    pub fn mul(self, b: Self) -> Self {
        let mut r = Self::zero();
        for col in 0..3 { for row in 0..3 { for k in 0..3 { r.m[col * 3 + row] += self.m[k * 3 + row] * b.m[col * 3 + k]; } } }
        r
    }

    pub fn transform_vec2(self, v: Vec2) -> Vec2 {
        Vec2::new(self.m[0] * v.x + self.m[3] * v.y + self.m[6], self.m[1] * v.x + self.m[4] * v.y + self.m[7])
    }

    pub fn inverse(self) -> Self {
        let d = self.m[0] * (self.m[4] * self.m[8] - self.m[5] * self.m[7])
            - self.m[3] * (self.m[1] * self.m[8] - self.m[2] * self.m[7])
            + self.m[6] * (self.m[1] * self.m[5] - self.m[2] * self.m[4]);
        if d == 0.0 { return Self::identity(); }
        let id = 1.0 / d;
        Self { m: [
            (self.m[4] * self.m[8] - self.m[5] * self.m[7]) * id,
            -(self.m[1] * self.m[8] - self.m[2] * self.m[7]) * id,
            (self.m[1] * self.m[5] - self.m[2] * self.m[4]) * id,
            -(self.m[3] * self.m[8] - self.m[5] * self.m[6]) * id,
            (self.m[0] * self.m[8] - self.m[2] * self.m[6]) * id,
            -(self.m[0] * self.m[5] - self.m[2] * self.m[3]) * id,
            (self.m[3] * self.m[7] - self.m[4] * self.m[6]) * id,
            -(self.m[0] * self.m[7] - self.m[1] * self.m[6]) * id,
            (self.m[0] * self.m[4] - self.m[1] * self.m[3]) * id,
        ]}
    }
}

impl Index<usize> for Mat3 { type Output = f32; #[inline] fn index(&self, i: usize) -> &f32 { &self.m[i] } }
impl IndexMut<usize> for Mat3 { #[inline] fn index_mut(&mut self, i: usize) -> &mut f32 { &mut self.m[i] } }
impl Mul for Mat3 { type Output = Self; #[inline] fn mul(self, b: Self) -> Self { self.mul(b) } }
impl Mul<Vec2> for Mat3 { type Output = Vec2; #[inline] fn mul(self, v: Vec2) -> Vec2 { self.transform_vec2(v) } }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    pub m: [f32; 16],
}

impl Mat4 {
    pub fn identity() -> Self {
        let mut m = Self::zero(); m.m[0] = 1.0; m.m[5] = 1.0; m.m[10] = 1.0; m.m[15] = 1.0; m
    }
    #[inline] pub fn zero() -> Self { Self { m: [0.0; 16] } }
    pub fn translate(x: f32, y: f32, z: f32) -> Self { let mut m = Self::identity(); m.m[12] = x; m.m[13] = y; m.m[14] = z; m }
    pub fn scale(x: f32, y: f32, z: f32) -> Self { Self { m: [x, 0.0, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 0.0, z, 0.0, 0.0, 0.0, 0.0, 1.0] } }

    pub fn rotate(angle_deg: f32, axis: Vec3) -> Self {
        let rad = angle_deg * (core::f32::consts::PI / 180.0);
        let (s, c) = (sin_f32(rad), cos_f32(rad));
        let nc = 1.0 - c;
        let n = axis.norm();
        let mut m = Self::zero();
        m.m[0] = c + n.x * n.x * nc;
        m.m[1] = n.y * n.x * nc + n.z * s;
        m.m[2] = n.z * n.x * nc - n.y * s;
        m.m[4] = n.x * n.y * nc - n.z * s;
        m.m[5] = c + n.y * n.y * nc;
        m.m[6] = n.z * n.y * nc + n.x * s;
        m.m[8] = n.x * n.z * nc + n.y * s;
        m.m[9] = n.y * n.z * nc - n.x * s;
        m.m[10] = c + n.z * n.z * nc;
        m.m[15] = 1.0;
        m
    }

    pub fn mul(self, b: Self) -> Self {
        let mut r = Self::zero();
        for col in 0..4 { for row in 0..4 { for k in 0..4 { r.m[col * 4 + row] += self.m[k * 4 + row] * b.m[col * 4 + k]; } } }
        r
    }

    pub fn transform_vec4(self, v: Vec4) -> Vec4 {
        Vec4::new(
            self.m[0] * v.x + self.m[4] * v.y + self.m[8] * v.z + self.m[12] * v.w,
            self.m[1] * v.x + self.m[5] * v.y + self.m[9] * v.z + self.m[13] * v.w,
            self.m[2] * v.x + self.m[6] * v.y + self.m[10] * v.z + self.m[14] * v.w,
            self.m[3] * v.x + self.m[7] * v.y + self.m[11] * v.z + self.m[15] * v.w,
        )
    }

    pub fn transform_vec3(self, v: Vec3) -> Vec3 {
        let r = self.transform_vec4(Vec4::new(v.x, v.y, v.z, 1.0));
        Vec3::new(r.x / r.w, r.y / r.w, r.z / r.w)
    }

    pub fn transpose(self) -> Self {
        let mut r = Self::zero();
        for i in 0..4 { for j in 0..4 { r.m[j * 4 + i] = self.m[i * 4 + j]; } }
        r
    }

    pub fn perspective(fov_deg: f32, aspect: f32, znear: f32, zfar: f32) -> Self {
        let f = 1.0 / tan_f32(fov_deg * core::f32::consts::PI / 360.0);
        let id = 1.0 / (znear - zfar);
        let mut m = Self::zero();
        m.m[0] = f / aspect; m.m[5] = f;
        m.m[10] = (zfar + znear) * id; m.m[11] = -1.0;
        m.m[14] = 2.0 * zfar * znear * id;
        m
    }

    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, znear: f32, zfar: f32) -> Self {
        let mut m = Self::zero();
        m.m[0] = 2.0 / (right - left); m.m[5] = 2.0 / (top - bottom);
        m.m[10] = 2.0 / (znear - zfar);
        m.m[12] = (left + right) / (left - right); m.m[13] = (bottom + top) / (bottom - top);
        m.m[14] = (znear + zfar) / (znear - zfar); m.m[15] = 1.0;
        m
    }

    pub fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let fwd = (center - eye).norm();
        let side = fwd.cross(up).norm();
        let u = side.cross(fwd);
        let mut m = Self::zero();
        m.m[0] = side.x; m.m[4] = side.y; m.m[8] = side.z;
        m.m[1] = u.x;   m.m[5] = u.y;   m.m[9] = u.z;
        m.m[2] = -fwd.x; m.m[6] = -fwd.y; m.m[10] = -fwd.z;
        m.m[15] = 1.0;
        m * Self::translate(-eye.x, -eye.y, -eye.z)
    }

    fn det3x3(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, g: f32, h: f32, i: f32) -> f32 {
        a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
    }

    pub fn inverse(self) -> Self {
        let d = Self::det3x3(self.m[0], self.m[4], self.m[8], self.m[1], self.m[5], self.m[9], self.m[2], self.m[6], self.m[10]);
        if d == 0.0 { return Self::identity(); }
        let invd = 1.0 / d;
        Self { m: [
            Self::det3x3(self.m[5], self.m[9], self.m[13], self.m[6], self.m[10], self.m[14], self.m[7], self.m[11], self.m[15]) * invd,
            -Self::det3x3(self.m[1], self.m[9], self.m[13], self.m[2], self.m[10], self.m[14], self.m[3], self.m[11], self.m[15]) * invd,
            Self::det3x3(self.m[1], self.m[5], self.m[13], self.m[2], self.m[6], self.m[14], self.m[3], self.m[7], self.m[15]) * invd,
            -Self::det3x3(self.m[1], self.m[5], self.m[9], self.m[2], self.m[6], self.m[10], self.m[3], self.m[7], self.m[11]) * invd,
            -Self::det3x3(self.m[4], self.m[8], self.m[12], self.m[6], self.m[10], self.m[14], self.m[7], self.m[11], self.m[15]) * invd,
            Self::det3x3(self.m[0], self.m[8], self.m[12], self.m[2], self.m[10], self.m[14], self.m[3], self.m[11], self.m[15]) * invd,
            -Self::det3x3(self.m[0], self.m[4], self.m[12], self.m[2], self.m[6], self.m[14], self.m[3], self.m[7], self.m[15]) * invd,
            Self::det3x3(self.m[0], self.m[4], self.m[8], self.m[2], self.m[6], self.m[10], self.m[3], self.m[7], self.m[11]) * invd,
            Self::det3x3(self.m[4], self.m[8], self.m[12], self.m[5], self.m[9], self.m[13], self.m[7], self.m[11], self.m[15]) * invd,
            -Self::det3x3(self.m[0], self.m[8], self.m[12], self.m[1], self.m[9], self.m[13], self.m[3], self.m[11], self.m[15]) * invd,
            Self::det3x3(self.m[0], self.m[4], self.m[12], self.m[1], self.m[5], self.m[13], self.m[3], self.m[7], self.m[15]) * invd,
            -Self::det3x3(self.m[0], self.m[4], self.m[8], self.m[1], self.m[5], self.m[9], self.m[3], self.m[7], self.m[11]) * invd,
            -Self::det3x3(self.m[4], self.m[8], self.m[12], self.m[5], self.m[9], self.m[13], self.m[6], self.m[10], self.m[14]) * invd,
            Self::det3x3(self.m[0], self.m[8], self.m[12], self.m[1], self.m[9], self.m[13], self.m[2], self.m[10], self.m[14]) * invd,
            -Self::det3x3(self.m[0], self.m[4], self.m[12], self.m[1], self.m[5], self.m[13], self.m[2], self.m[6], self.m[14]) * invd,
            Self::det3x3(self.m[0], self.m[4], self.m[8], self.m[1], self.m[5], self.m[9], self.m[2], self.m[6], self.m[10]) * invd,
        ]}
    }
}

impl Index<usize> for Mat4 { type Output = f32; #[inline] fn index(&self, i: usize) -> &f32 { &self.m[i] } }
impl IndexMut<usize> for Mat4 { #[inline] fn index_mut(&mut self, i: usize) -> &mut f32 { &mut self.m[i] } }
impl Mul for Mat4 { type Output = Self; #[inline] fn mul(self, b: Self) -> Self { self.mul(b) } }
impl Mul<Vec4> for Mat4 { type Output = Vec4; #[inline] fn mul(self, v: Vec4) -> Vec4 { self.transform_vec4(v) } }
impl Mul<Vec3> for Mat4 { type Output = Vec3; #[inline] fn mul(self, v: Vec3) -> Vec3 { self.transform_vec3(v) } }

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Quat {
    pub const IDENTITY: Quat = Quat { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
    #[inline] pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self { Self { x, y, z, w } }

    pub fn from_axis_angle(axis: Vec3, angle_deg: f32) -> Self {
        let rad = angle_deg * (core::f32::consts::PI / 180.0);
        let half = rad * 0.5;
        let s = sin_f32(half);
        let a = axis.norm();
        Self::new(a.x * s, a.y * s, a.z * s, cos_f32(half))
    }

    pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        let (sp, cp) = (sin_f32(pitch * 0.5), cos_f32(pitch * 0.5));
        let (sy, cy) = (sin_f32(yaw * 0.5), cos_f32(yaw * 0.5));
        let (sr, cr) = (sin_f32(roll * 0.5), cos_f32(roll * 0.5));
        Self::new(
            cy * sp * cr + sy * cp * sr,
            sy * cp * cr - cy * sp * sr,
            cy * cp * sr - sy * sp * cr,
            cy * cp * cr + sy * sp * sr,
        )
    }

    #[inline] pub fn len(self) -> f32 { sqrt_f32(self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w) }
    #[inline] pub fn norm(self) -> Self { let l = self.len(); if l > 0.0 { self * (1.0 / l) } else { Self::IDENTITY } }
    #[inline] pub fn conjugate(self) -> Self { Self::new(-self.x, -self.y, -self.z, self.w) }
    #[inline] pub fn inverse(self) -> Self { self.conjugate() / self.len_sq() }
    #[inline] pub fn len_sq(self) -> f32 { self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w }

    pub fn mul(self, b: Self) -> Self {
        Self::new(
            self.w * b.x + self.x * b.w + self.y * b.z - self.z * b.y,
            self.w * b.y - self.x * b.z + self.y * b.w + self.z * b.x,
            self.w * b.z + self.x * b.y - self.y * b.x + self.z * b.w,
            self.w * b.w - self.x * b.x - self.y * b.y - self.z * b.z,
        )
    }

    pub fn rotate_vec3(self, v: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z);
        let uv = qv.cross(v);
        let uuv = qv.cross(uv);
        v + uv * (2.0 * self.w) + uuv * 2.0
    }

    pub fn to_mat4(self) -> Mat4 {
        let q = self.norm();
        let (x, y, z, w) = (q.x, q.y, q.z, q.w);
        let (xx, yy, zz) = (x * x, y * y, z * z);
        let (xy, xz, yz) = (x * y, x * z, y * z);
        let (wx, wy, wz) = (w * x, w * y, w * z);
        let mut m = Mat4::zero();
        m.m[0] = 1.0 - 2.0 * (yy + zz);
        m.m[1] = 2.0 * (xy + wz);
        m.m[2] = 2.0 * (xz - wy);
        m.m[4] = 2.0 * (xy - wz);
        m.m[5] = 1.0 - 2.0 * (xx + zz);
        m.m[6] = 2.0 * (yz + wx);
        m.m[8] = 2.0 * (xz + wy);
        m.m[9] = 2.0 * (yz - wx);
        m.m[10] = 1.0 - 2.0 * (xx + yy);
        m.m[15] = 1.0;
        m
    }

    #[inline]
    pub fn lerp(self, b: Self, t: f32) -> Self {
        let cos = self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w;
        let (a, b) = if cos < 0.0 { (-t, b * -1.0) } else { (t, b) };
        self * (1.0 - a) + b * a
    }

    pub fn slerp(self, b: Self, t: f32) -> Self {
        let mut cos = self.x * b.x + self.y * b.y + self.z * b.z + self.w * b.w;
        let mut b = b;
        if cos < 0.0 { cos = -cos; b = -b; }
        if cos > 0.9995 { return self.lerp(b, t); }
        let a0 = acos_f32(cos);
        let a1 = a0 * t;
        let s0 = sin_f32(a1) / sin_f32(a0);
        let s1 = sin_f32(a1 - a0) / sin_f32(a0);
        self * s0 + b * s1
    }
}

impl Add for Quat { type Output = Self; #[inline] fn add(self, b: Self) -> Self { Self::new(self.x + b.x, self.y + b.y, self.z + b.z, self.w + b.w) } }
impl Sub for Quat { type Output = Self; #[inline] fn sub(self, b: Self) -> Self { Self::new(self.x - b.x, self.y - b.y, self.z - b.z, self.w - b.w) } }
impl Mul<f32> for Quat { type Output = Self; #[inline] fn mul(self, s: f32) -> Self { Self::new(self.x * s, self.y * s, self.z * s, self.w * s) } }
impl Div<f32> for Quat { type Output = Self; #[inline] fn div(self, s: f32) -> Self { Self::new(self.x / s, self.y / s, self.z / s, self.w / s) } }
impl Neg for Quat { type Output = Self; #[inline] fn neg(self) -> Self { Self::new(-self.x, -self.y, -self.z, -self.w) } }
impl Mul for Quat { type Output = Self; #[inline] fn mul(self, b: Self) -> Self { self.mul(b) } }
