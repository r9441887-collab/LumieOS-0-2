use crate::system::softfloat::{sqrtf, sinf, cosf, tanf};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mat3 {
    pub m: [f32; 9],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mat4 {
    pub m: [f32; 16],
}

/* === Vec2 === */
pub fn vec2_new(x: f32, y: f32) -> Vec2 { Vec2 { x, y } }
pub fn vec2_add(a: Vec2, b: Vec2) -> Vec2 { Vec2 { x: a.x + b.x, y: a.y + b.y } }
pub fn vec2_sub(a: Vec2, b: Vec2) -> Vec2 { Vec2 { x: a.x - b.x, y: a.y - b.y } }
pub fn vec2_mul(a: Vec2, s: f32) -> Vec2 { Vec2 { x: a.x * s, y: a.y * s } }
pub fn vec2_div(a: Vec2, s: f32) -> Vec2 { Vec2 { x: a.x / s, y: a.y / s } }
pub fn vec2_dot(a: Vec2, b: Vec2) -> f32 { a.x * b.x + a.y * b.y }
pub fn vec2_len(a: Vec2) -> f32 { sqrtf(a.x * a.x + a.y * a.y) }
pub fn vec2_norm(a: Vec2) -> Vec2 {
    let l = vec2_len(a);
    if l > 0.0 { vec2_div(a, l) } else { a }
}

/* === Mat3 (column-major) === */
pub fn mat3_identity() -> Mat3 {
    Mat3 {
        m: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
    }
}

pub fn mat3_translate(x: f32, y: f32) -> Mat3 {
    let mut m = mat3_identity();
    m.m[6] = x;
    m.m[7] = y;
    m
}

pub fn mat3_rotate(angle_deg: f32) -> Mat3 {
    let rad = angle_deg * (3.14159265_f32 / 180.0_f32);
    let c = cosf(rad);
    let s = sinf(rad);
    Mat3 {
        m: [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0],
    }
}

pub fn mat3_scale(x: f32, y: f32) -> Mat3 {
    Mat3 {
        m: [x, 0.0, 0.0, 0.0, y, 0.0, 0.0, 0.0, 1.0],
    }
}

pub fn mat3_mul(a: Mat3, b: Mat3) -> Mat3 {
    let mut r = Mat3 { m: [0.0; 9] };
    for col in 0..3 {
        for row in 0..3 {
            for k in 0..3 {
                r.m[col * 3 + row] += a.m[k * 3 + row] * b.m[col * 3 + k];
            }
        }
    }
    r
}

pub fn mat3_transform_vec2(m: Mat3, v: Vec2) -> Vec2 {
    Vec2 {
        x: m.m[0] * v.x + m.m[3] * v.y + m.m[6],
        y: m.m[1] * v.x + m.m[4] * v.y + m.m[7],
    }
}

pub fn mat3_inverse(m: Mat3) -> Mat3 {
    let det = m.m[0] * (m.m[4] * m.m[8] - m.m[5] * m.m[7])
        - m.m[3] * (m.m[1] * m.m[8] - m.m[2] * m.m[7])
        + m.m[6] * (m.m[1] * m.m[5] - m.m[2] * m.m[4]);
    if det == 0.0 { return mat3_identity(); }
    let id = 1.0 / det;
    Mat3 {
        m: [
            (m.m[4] * m.m[8] - m.m[5] * m.m[7]) * id,
            -(m.m[1] * m.m[8] - m.m[2] * m.m[7]) * id,
            (m.m[1] * m.m[5] - m.m[2] * m.m[4]) * id,
            -(m.m[3] * m.m[8] - m.m[5] * m.m[6]) * id,
            (m.m[0] * m.m[8] - m.m[2] * m.m[6]) * id,
            -(m.m[0] * m.m[5] - m.m[2] * m.m[3]) * id,
            (m.m[3] * m.m[7] - m.m[4] * m.m[6]) * id,
            -(m.m[0] * m.m[7] - m.m[1] * m.m[6]) * id,
            (m.m[0] * m.m[4] - m.m[1] * m.m[3]) * id,
        ],
    }
}

/* === Vec3 === */
pub fn vec3_new(x: f32, y: f32, z: f32) -> Vec3 { Vec3 { x, y, z } }
pub fn vec3_add(a: Vec3, b: Vec3) -> Vec3 { Vec3 { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z } }
pub fn vec3_sub(a: Vec3, b: Vec3) -> Vec3 { Vec3 { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z } }
pub fn vec3_mul(a: Vec3, s: f32) -> Vec3 { Vec3 { x: a.x * s, y: a.y * s, z: a.z * s } }
pub fn vec3_div(a: Vec3, s: f32) -> Vec3 { Vec3 { x: a.x / s, y: a.y / s, z: a.z / s } }
pub fn vec3_dot(a: Vec3, b: Vec3) -> f32 { a.x * b.x + a.y * b.y + a.z * b.z }

pub fn vec3_cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

pub fn vec3_len(a: Vec3) -> f32 { sqrtf(a.x * a.x + a.y * a.y + a.z * a.z) }
pub fn vec3_norm(a: Vec3) -> Vec3 {
    let l = vec3_len(a);
    if l > 0.0 { vec3_div(a, l) } else { a }
}

/* === Vec4 === */
pub fn vec4_new(x: f32, y: f32, z: f32, w: f32) -> Vec4 { Vec4 { x, y, z, w } }
pub fn vec4_add(a: Vec4, b: Vec4) -> Vec4 { Vec4 { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z, w: a.w + b.w } }
pub fn vec4_mul(a: Vec4, s: f32) -> Vec4 { Vec4 { x: a.x * s, y: a.y * s, z: a.z * s, w: a.w * s } }

/* === Mat4 (column-major) === */
pub fn mat4_identity() -> Mat4 {
    let mut m = Mat4 { m: [0.0; 16] };
    m.m[0] = 1.0;
    m.m[5] = 1.0;
    m.m[10] = 1.0;
    m.m[15] = 1.0;
    m
}

pub fn mat4_zero() -> Mat4 {
    Mat4 { m: [0.0; 16] }
}

pub fn mat4_mul(a: Mat4, b: Mat4) -> Mat4 {
    let mut r = Mat4 { m: [0.0; 16] };
    for col in 0..4 {
        for row in 0..4 {
            for k in 0..4 {
                r.m[col * 4 + row] += a.m[k * 4 + row] * b.m[col * 4 + k];
            }
        }
    }
    r
}

pub fn mat4_mul_vec4(m: Mat4, v: Vec4) -> Vec4 {
    Vec4 {
        x: m.m[0] * v.x + m.m[4] * v.y + m.m[8] * v.z + m.m[12] * v.w,
        y: m.m[1] * v.x + m.m[5] * v.y + m.m[9] * v.z + m.m[13] * v.w,
        z: m.m[2] * v.x + m.m[6] * v.y + m.m[10] * v.z + m.m[14] * v.w,
        w: m.m[3] * v.x + m.m[7] * v.y + m.m[11] * v.z + m.m[15] * v.w,
    }
}

pub fn mat4_mul_vec3(m: Mat4, v: Vec3) -> Vec3 {
    let r = mat4_mul_vec4(m, vec4_new(v.x, v.y, v.z, 1.0));
    vec3_new(r.x / r.w, r.y / r.w, r.z / r.w)
}

pub fn mat4_transpose(m: Mat4) -> Mat4 {
    let mut r = Mat4 { m: [0.0; 16] };
    for i in 0..4 {
        for j in 0..4 {
            r.m[j * 4 + i] = m.m[i * 4 + j];
        }
    }
    r
}

pub fn mat4_translate(x: f32, y: f32, z: f32) -> Mat4 {
    let mut m = mat4_identity();
    m.m[12] = x;
    m.m[13] = y;
    m.m[14] = z;
    m
}

pub fn mat4_scale(x: f32, y: f32, z: f32) -> Mat4 {
    let mut m = Mat4 { m: [0.0; 16] };
    m.m[0] = x;
    m.m[5] = y;
    m.m[10] = z;
    m.m[15] = 1.0;
    m
}

pub fn mat4_rotate(angle_deg: f32, axis: Vec3) -> Mat4 {
    let rad = angle_deg * (3.14159265_f32 / 180.0_f32);
    let c = cosf(rad);
    let s = sinf(rad);
    let nc = 1.0 - c;
    let n = vec3_norm(axis);
    let mut m = Mat4 { m: [0.0; 16] };
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

pub fn mat4_perspective(fov_deg: f32, aspect: f32, znear: f32, zfar: f32) -> Mat4 {
    let f = 1.0 / tanf(fov_deg * 3.14159265_f32 / 360.0_f32);
    let id = 1.0 / (znear - zfar);
    let mut m = Mat4 { m: [0.0; 16] };
    m.m[0] = f / aspect;
    m.m[5] = f;
    m.m[10] = (zfar + znear) * id;
    m.m[11] = -1.0;
    m.m[14] = 2.0 * zfar * znear * id;
    m
}

pub fn mat4_ortho(left: f32, right: f32, bottom: f32, top: f32, znear: f32, zfar: f32) -> Mat4 {
    let mut m = Mat4 { m: [0.0; 16] };
    m.m[0] = 2.0 / (right - left);
    m.m[5] = 2.0 / (top - bottom);
    m.m[10] = 2.0 / (znear - zfar);
    m.m[12] = (left + right) / (left - right);
    m.m[13] = (bottom + top) / (bottom - top);
    m.m[14] = (znear + zfar) / (znear - zfar);
    m.m[15] = 1.0;
    m
}

pub fn mat4_lookat(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    let fwd = vec3_norm(vec3_sub(center, eye));
    let side = vec3_norm(vec3_cross(fwd, up));
    let u = vec3_cross(side, fwd);
    let mut m = Mat4 { m: [0.0; 16] };
    m.m[0] = side.x;
    m.m[4] = side.y;
    m.m[8] = side.z;
    m.m[1] = u.x;
    m.m[5] = u.y;
    m.m[9] = u.z;
    m.m[2] = -fwd.x;
    m.m[6] = -fwd.y;
    m.m[10] = -fwd.z;
    m.m[15] = 1.0;
    let t = mat4_translate(-eye.x, -eye.y, -eye.z);
    mat4_mul(m, t)
}

fn mat4_det3x3(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, g: f32, h: f32, i: f32) -> f32 {
    a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

pub fn mat4_inverse(m: Mat4) -> Mat4 {
    let d = mat4_det3x3(
        m.m[0], m.m[4], m.m[8],
        m.m[1], m.m[5], m.m[9],
        m.m[2], m.m[6], m.m[10],
    );
    if d == 0.0 { return mat4_identity(); }
    let invd = 1.0 / d;
    Mat4 {
        m: [
            mat4_det3x3(m.m[5], m.m[9], m.m[13], m.m[6], m.m[10], m.m[14], m.m[7], m.m[11], m.m[15]) * invd,
            -mat4_det3x3(m.m[1], m.m[9], m.m[13], m.m[2], m.m[10], m.m[14], m.m[3], m.m[11], m.m[15]) * invd,
            mat4_det3x3(m.m[1], m.m[5], m.m[13], m.m[2], m.m[6], m.m[14], m.m[3], m.m[7], m.m[15]) * invd,
            -mat4_det3x3(m.m[1], m.m[5], m.m[9], m.m[2], m.m[6], m.m[10], m.m[3], m.m[7], m.m[11]) * invd,
            -mat4_det3x3(m.m[4], m.m[8], m.m[12], m.m[6], m.m[10], m.m[14], m.m[7], m.m[11], m.m[15]) * invd,
            mat4_det3x3(m.m[0], m.m[8], m.m[12], m.m[2], m.m[10], m.m[14], m.m[3], m.m[11], m.m[15]) * invd,
            -mat4_det3x3(m.m[0], m.m[4], m.m[12], m.m[2], m.m[6], m.m[14], m.m[3], m.m[7], m.m[15]) * invd,
            mat4_det3x3(m.m[0], m.m[4], m.m[8], m.m[2], m.m[6], m.m[10], m.m[3], m.m[7], m.m[11]) * invd,
            mat4_det3x3(m.m[4], m.m[8], m.m[12], m.m[5], m.m[9], m.m[13], m.m[7], m.m[11], m.m[15]) * invd,
            -mat4_det3x3(m.m[0], m.m[8], m.m[12], m.m[1], m.m[9], m.m[13], m.m[3], m.m[11], m.m[15]) * invd,
            mat4_det3x3(m.m[0], m.m[4], m.m[12], m.m[1], m.m[5], m.m[13], m.m[3], m.m[7], m.m[15]) * invd,
            -mat4_det3x3(m.m[0], m.m[4], m.m[8], m.m[1], m.m[5], m.m[9], m.m[3], m.m[7], m.m[11]) * invd,
            -mat4_det3x3(m.m[4], m.m[8], m.m[12], m.m[5], m.m[9], m.m[13], m.m[6], m.m[10], m.m[14]) * invd,
            mat4_det3x3(m.m[0], m.m[8], m.m[12], m.m[1], m.m[9], m.m[13], m.m[2], m.m[10], m.m[14]) * invd,
            -mat4_det3x3(m.m[0], m.m[4], m.m[12], m.m[1], m.m[5], m.m[13], m.m[2], m.m[6], m.m[14]) * invd,
            mat4_det3x3(m.m[0], m.m[4], m.m[8], m.m[1], m.m[5], m.m[9], m.m[2], m.m[6], m.m[10]) * invd,
        ],
    }
}
