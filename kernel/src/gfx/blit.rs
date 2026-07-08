use super::context;

pub unsafe fn blit_rect(
    src_buf: *const u32, src_pitch: u32,
    dst_x: u32, dst_y: u32, w: u32, h: u32,
    blend: bool,
) {
    let fb = context::gfx_ctx();
    for j in 0..h {
        let dy = dst_y + j;
        if dy >= fb.height { break; }
        for i in 0..w {
            let dx = dst_x + i;
            if dx >= fb.width { break; }
            let src_pixel = *src_buf.offset((j as u64 * src_pitch as u64 / 4 + i as u64) as isize);
            let pixel = if blend {
                let dst_pixel = context::gfx_fb_read(dx, dy);
                blend_over(src_pixel, dst_pixel)
            } else {
                src_pixel
            };
            context::gfx_fb_write(dx, dy, pixel);
        }
    }
}

#[inline]
pub fn blend_over(src: u32, dst: u32) -> u32 {
    let sa = (src >> 24) & 0xFF;
    if sa == 0xFF { return src; }
    if sa == 0 { return dst; }
    let sr = (src >> 16) & 0xFF;
    let sg = (src >> 8) & 0xFF;
    let sb = src & 0xFF;
    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8) & 0xFF;
    let db = dst & 0xFF;
    let out_r = (sr * sa + dr * (255 - sa)) / 255;
    let out_g = (sg * sa + dg * (255 - sa)) / 255;
    let out_b = (sb * sa + db * (255 - sa)) / 255;
    0xFF000000 | ((out_r as u32) << 16) | ((out_g as u32) << 8) | out_b as u32
}

pub unsafe fn blit_rect_tinted(
    src_buf: *const u32, src_pitch: u32,
    dst_x: u32, dst_y: u32, w: u32, h: u32,
    tint: u32, blend: bool,
) {
    let tr = (tint >> 16) & 0xFF;
    let tg = (tint >> 8) & 0xFF;
    let tb = tint & 0xFF;
    let fb = context::gfx_ctx();
    for j in 0..h {
        let dy = dst_y + j;
        if dy >= fb.height { break; }
        for i in 0..w {
            let dx = dst_x + i;
            if dx >= fb.width { break; }
            let src_pixel = *src_buf.offset((j as u64 * src_pitch as u64 / 4 + i as u64) as isize);
            let sa = (src_pixel >> 24) & 0xFF;
            let tinted = (((sa as u32 * tr / 255) & 0xFF) << 16)
                       | (((sa as u32 * tg / 255) & 0xFF) << 8)
                       | ((sa as u32 * tb / 255) & 0xFF)
                       | ((sa as u32) << 24);
            let pixel = if blend {
                let dst_pixel = context::gfx_fb_read(dx, dy);
                blend_over(tinted, dst_pixel)
            } else {
                tinted
            };
            context::gfx_fb_write(dx, dy, pixel);
        }
    }
}
