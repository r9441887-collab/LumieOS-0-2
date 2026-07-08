use crate::vec::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const ZERO: Rect = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
    #[inline] pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self { Self { x, y, w, h } }
    #[inline] pub fn from_corners(x0: f32, y0: f32, x1: f32, y1: f32) -> Self { Self { x: x0.min(x1), y: y0.min(y1), w: (x1 - x0).abs(), h: (y1 - y0).abs() } }
    #[inline] pub fn left(self) -> f32 { self.x }
    #[inline] pub fn right(self) -> f32 { self.x + self.w }
    #[inline] pub fn top(self) -> f32 { self.y }
    #[inline] pub fn bottom(self) -> f32 { self.y + self.h }
    #[inline] pub fn center(self) -> Vec2 { Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5) }
    #[inline] pub fn contains(self, p: Vec2) -> bool { p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h }
    #[inline] pub fn overlaps(self, other: Self) -> bool { self.x < other.x + other.w && self.x + self.w > other.x && self.y < other.y + other.h && self.y + self.h > other.y }
    #[inline] pub fn inflated(self, amount: f32) -> Self { Self::new(self.x - amount, self.y - amount, self.w + amount * 2.0, self.h + amount * 2.0) }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    #[inline] pub const fn new(origin: Vec3, dir: Vec3) -> Self { Self { origin, dir } }
    #[inline] pub fn point_at(self, t: f32) -> Vec3 { self.origin + self.dir * t }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    #[inline] pub const fn new(normal: Vec3, d: f32) -> Self { Self { normal, d } }
    pub fn from_points(a: Vec3, b: Vec3, c: Vec3) -> Self { let n = (b - a).cross(c - a).norm(); Self { normal: n, d: -a.dot(n) } }
    #[inline] pub fn distance_to(self, p: Vec3) -> f32 { self.normal.dot(p) + self.d }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    #[inline] pub const fn new(min: Vec3, max: Vec3) -> Self { Self { min, max } }
    #[inline] pub fn from_center(center: Vec3, half_size: Vec3) -> Self { Self::new(center - half_size, center + half_size) }
    #[inline] pub fn center(self) -> Vec3 { Vec3::new((self.min.x + self.max.x) * 0.5, (self.min.y + self.max.y) * 0.5, (self.min.z + self.max.z) * 0.5) }
    #[inline] pub fn size(self) -> Vec3 { self.max - self.min }
    #[inline] pub fn contains(self, p: Vec3) -> bool { p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y && p.z >= self.min.z && p.z <= self.max.z }
    #[inline] pub fn overlaps(self, other: Self) -> bool { self.min.x <= other.max.x && self.max.x >= other.min.x && self.min.y <= other.max.y && self.max.y >= other.min.y && self.min.z <= other.max.z && self.max.z >= other.min.z }
    pub fn union(self, other: Self) -> Self {
        Self::new(
            Vec3::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y), self.min.z.min(other.min.z)),
            Vec3::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y), self.max.z.max(other.max.z)),
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub center: Vec2,
    pub radius: f32,
}

impl Circle {
    #[inline] pub const fn new(center: Vec2, radius: f32) -> Self { Self { center, radius } }
    #[inline] pub fn contains(self, p: Vec2) -> bool { (p - self.center).len_sq() <= self.radius * self.radius }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Line2 {
    pub a: Vec2,
    pub b: Vec2,
}

impl Line2 {
    #[inline] pub const fn new(a: Vec2, b: Vec2) -> Self { Self { a, b } }
    #[inline] pub fn len(self) -> f32 { (self.b - self.a).len() }
    #[inline] pub fn dir(self) -> Vec2 { (self.b - self.a).norm() }
    #[inline] pub fn closest_point(self, p: Vec2) -> Vec2 {
        let ab = self.b - self.a;
        let ap = p - self.a;
        let t = (ap.dot(ab) / ab.len_sq()).clamp(0.0, 1.0);
        self.a + ab * t
    }
    #[inline] pub fn distance_to(self, p: Vec2) -> f32 { (p - self.closest_point(p)).len() }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Tri2 {
    pub a: Vec2,
    pub b: Vec2,
    pub c: Vec2,
}

impl Tri2 {
    #[inline] pub const fn new(a: Vec2, b: Vec2, c: Vec2) -> Self { Self { a, b, c } }

    pub fn contains(self, p: Vec2) -> bool {
        let ab = self.b - self.a;
        let ac = self.c - self.a;
        let ap = p - self.a;
        let d00 = ab.dot(ab);
        let d01 = ab.dot(ac);
        let d11 = ac.dot(ac);
        let d20 = ap.dot(ab);
        let d21 = ap.dot(ac);
        let denom = d00 * d11 - d01 * d01;
        if denom == 0.0 { return false; }
        let v = (d11 * d20 - d01 * d21) / denom;
        let w = (d00 * d21 - d01 * d20) / denom;
        let u = 1.0 - v - w;
        u >= 0.0 && v >= 0.0 && w >= 0.0
    }

    pub fn area(self) -> f32 {
        let cross = (self.b.x - self.a.x) * (self.c.y - self.a.y) - (self.b.y - self.a.y) * (self.c.x - self.a.x);
        (cross * 0.5).abs()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Tri3 {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
}

impl Tri3 {
    #[inline] pub const fn new(a: Vec3, b: Vec3, c: Vec3) -> Self { Self { a, b, c } }
    #[inline] pub fn normal(self) -> Vec3 { (self.b - self.a).cross(self.c - self.a).norm() }
    #[inline] pub fn area(self) -> f32 { (self.b - self.a).cross(self.c - self.a).len() * 0.5 }
}
