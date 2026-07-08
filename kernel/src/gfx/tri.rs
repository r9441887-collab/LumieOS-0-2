use super::context;

fn ifloor(x: f32) -> i32 {
    let xi = x as i32;
    if x >= 0.0 || x as i32 as f32 == x { xi } else { xi - 1 }
}
fn iceil(x: f32) -> i32 {
    let xi = x as i32;
    if x <= 0.0 || x as i32 as f32 == x { xi } else { xi + 1 }
}

pub fn tri_rasterize(
    x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32,
    c0: u32, c1: u32, c2: u32,
    buf: &mut [u32], buf_w: u32, buf_h: u32,
) {
    let min_x = ifloor(x0.min(x1).min(x2)).max(0);
    let min_y = ifloor(y0.min(y1).min(y2)).max(0);
    let max_x = iceil(x0.max(x1).max(x2)).min(buf_w as i32 - 1);
    let max_y = iceil(y0.max(y1).max(y2)).min(buf_h as i32 - 1);

    if min_x > max_x || min_y > max_y { return; }

    let dx20 = x2 - x0;
    let dy20 = y2 - y0;
    let dx10 = x1 - x0;
    let dy10 = y1 - y0;
    let area = dx20 * dy10 - dy20 * dx10;
    if area.abs() < 0.001 { return; }
    let inv_area = 1.0 / area;

    let (r0, g0, b0, a0) = unpack_color(c0);
    let (r1, g1, b1, a1) = unpack_color(c1);
    let (r2, g2, b2, a2) = unpack_color(c2);

    for py in min_y..=max_y {
        let fy = py as f32 + 0.5;
        for px in min_x..=max_x {
            let fx = px as f32 + 0.5;
            let dx0 = fx - x0;
            let dy0 = fy - y0;
            let w2 = (dx20 * dy0 - dy20 * dx0) * inv_area;
            let w1 = (dx0 * dy10 - dy0 * dx10) * inv_area;
            let w0 = 1.0 - w1 - w2;

            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let ri = (w0 * r0 + w1 * r1 + w2 * r2) as u32;
                let gi = (w0 * g0 + w1 * g1 + w2 * g2) as u32;
                let bi = (w0 * b0 + w1 * b1 + w2 * b2) as u32;
                let ai = (w0 * a0 + w1 * a1 + w2 * a2) as u32;
                let color = (ai.min(255) << 24) | (ri.min(255) << 16) | (gi.min(255) << 8) | bi.min(255);
                let idx = (py as u32 * buf_w + px as u32) as usize;
                if idx < buf.len() {
                    buf[idx] = color;
                }
            }
        }
    }
}

pub unsafe fn tri_fill(
    x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32,
    color: u32,
) {
    let w = context::gfx_width();
    let h = context::gfx_height();

    let min_x = ifloor(x0.min(x1).min(x2)).max(0) as u32;
    let min_y = ifloor(y0.min(y1).min(y2)).max(0) as u32;
    let max_x = iceil(x0.max(x1).max(x2)).min(w as i32 - 1) as u32;
    let max_y = iceil(y0.max(y1).max(y2)).min(h as i32 - 1) as u32;

    if min_x > max_x || min_y > max_y { return; }
    let bw = max_x - min_x + 1;
    let bh = max_y - min_y + 1;
    let size = (bw as u64) * (bh as u64);
    if size > 1024 * 1024 { return; }

    let buf = crate::mm::alloc(size * 4);
    if buf.is_null() { return; }
    core::ptr::write_bytes(buf, 0, (size * 4) as usize);

    let slice = core::slice::from_raw_parts_mut(buf as *mut u32, size as usize);
    tri_rasterize(
        x0 - min_x as f32, y0 - min_y as f32,
        x1 - min_x as f32, y1 - min_y as f32,
        x2 - min_x as f32, y2 - min_y as f32,
        color, color, color,
        slice, bw, bh,
    );

    blit_rect_no_blend(buf as *const u32, bw * 4, min_x, min_y, bw, bh);
    crate::mm::free(buf);
}

unsafe fn blit_rect_no_blend(src: *const u32, src_pitch: u32, dst_x: u32, dst_y: u32, w: u32, h: u32) {
    let fb = context::gfx_ctx();
    for j in 0..h {
        let dy = dst_y + j;
        if dy >= fb.height { break; }
        for i in 0..w {
            let dx = dst_x + i;
            if dx >= fb.width { break; }
            let pixel = *src.offset((j as u64 * src_pitch as u64 / 4 + i as u64) as isize);
            if pixel & 0xFF000000 != 0 {
                context::gfx_fb_write(dx, dy, pixel);
            }
        }
    }
}

pub unsafe fn tri_fill_gradient(
    x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32,
    c0: u32, c1: u32, c2: u32,
) {
    let w = context::gfx_width();
    let h = context::gfx_height();

    let min_x = ifloor(x0.min(x1).min(x2)).max(0) as u32;
    let min_y = ifloor(y0.min(y1).min(y2)).max(0) as u32;
    let max_x = iceil(x0.max(x1).max(x2)).min(w as i32 - 1) as u32;
    let max_y = iceil(y0.max(y1).max(y2)).min(h as i32 - 1) as u32;

    if min_x > max_x || min_y > max_y { return; }
    let bw = max_x - min_x + 1;
    let bh = max_y - min_y + 1;
    let size = (bw as u64) * (bh as u64);
    if size > 1024 * 1024 { return; }

    let buf = crate::mm::alloc(size * 4);
    if buf.is_null() { return; }
    core::ptr::write_bytes(buf, 0, (size * 4) as usize);

    let slice = core::slice::from_raw_parts_mut(buf as *mut u32, size as usize);
    tri_rasterize(
        x0 - min_x as f32, y0 - min_y as f32,
        x1 - min_x as f32, y1 - min_y as f32,
        x2 - min_x as f32, y2 - min_y as f32,
        c0, c1, c2,
        slice, bw, bh,
    );

    blit_rect_no_blend(buf as *const u32, bw * 4, min_x, min_y, bw, bh);
    crate::mm::free(buf);
}

#[inline]
fn unpack_color(c: u32) -> (f32, f32, f32, f32) {
    (
        ((c >> 16) & 0xFF) as f32,
        ((c >> 8) & 0xFF) as f32,
        (c & 0xFF) as f32,
        ((c >> 24) & 0xFF) as f32,
    )
}
