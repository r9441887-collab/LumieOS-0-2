use crate::vec::*;
use crate::geom::*;
use crate::math_ops::{sqrt_f32, abs_f32};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

pub fn ray_plane(ray: Ray, plane: Plane) -> Option<Hit> {
    let denom = plane.normal.dot(ray.dir);
    if abs_f32(denom) < 1e-6 { return None; }
    let t = -(plane.normal.dot(ray.origin) + plane.d) / denom;
    if t < 0.0 { return None; }
    Some(Hit { t, point: ray.point_at(t), normal: plane.normal })
}

pub fn ray_triangle(ray: Ray, tri: Tri3) -> Option<Hit> {
    let edge1 = tri.b - tri.a;
    let edge2 = tri.c - tri.a;
    let h = ray.dir.cross(edge2);
    let a = edge1.dot(h);
    if abs_f32(a) < 1e-6 { return None; }
    let f = 1.0 / a;
    let s = ray.origin - tri.a;
    let u = f * s.dot(h);
    if u < 0.0 || u > 1.0 { return None; }
    let q = s.cross(edge1);
    let v = f * ray.dir.dot(q);
    if v < 0.0 || u + v > 1.0 { return None; }
    let t = f * edge2.dot(q);
    if t < 0.0 { return None; }
    let normal = edge1.cross(edge2).norm();
    Some(Hit { t, point: ray.point_at(t), normal })
}

pub fn ray_sphere(ray: Ray, center: Vec3, radius: f32) -> Option<Hit> {
    let oc = ray.origin - center;
    let a = ray.dir.dot(ray.dir);
    let b = 2.0 * oc.dot(ray.dir);
    let c = oc.dot(oc) - radius * radius;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 { return None; }
    let sqrt_d = sqrt_f32(disc);
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);
    let t = if t1 >= 0.0 { t1 } else if t2 >= 0.0 { t2 } else { return None; };
    let point = ray.point_at(t);
    let normal = (point - center).norm();
    Some(Hit { t, point, normal })
}

pub fn ray_aabb(ray: Ray, aabb: Aabb) -> Option<Hit> {
    let inv_dir = Vec3::new(1.0 / ray.dir.x, 1.0 / ray.dir.y, 1.0 / ray.dir.z);
    let t1 = Vec3::new(
        (aabb.min.x - ray.origin.x) * inv_dir.x,
        (aabb.min.y - ray.origin.y) * inv_dir.y,
        (aabb.min.z - ray.origin.z) * inv_dir.z,
    );
    let t2 = Vec3::new(
        (aabb.max.x - ray.origin.x) * inv_dir.x,
        (aabb.max.y - ray.origin.y) * inv_dir.y,
        (aabb.max.z - ray.origin.z) * inv_dir.z,
    );

    let tmin = t1.x.min(t2.x).max(t1.y.min(t2.y)).max(t1.z.min(t2.z));
    let tmax = t1.x.max(t2.x).min(t1.y.max(t2.y)).min(t1.z.max(t2.z));

    if tmax < 0.0 || tmin > tmax { return None; }
    let t = if tmin >= 0.0 { tmin } else { tmax };

    let point = ray.point_at(t);
    let eps = 0.001;
    let normal = if abs_f32(point.x - aabb.min.x) < eps { Vec3::new(-1.0, 0.0, 0.0) }
    else if abs_f32(point.x - aabb.max.x) < eps { Vec3::new(1.0, 0.0, 0.0) }
    else if abs_f32(point.y - aabb.min.y) < eps { Vec3::new(0.0, -1.0, 0.0) }
    else if abs_f32(point.y - aabb.max.y) < eps { Vec3::new(0.0, 1.0, 0.0) }
    else if abs_f32(point.z - aabb.min.z) < eps { Vec3::new(0.0, 0.0, -1.0) }
    else { Vec3::new(0.0, 0.0, 1.0) };
    Some(Hit { t, point, normal })
}
