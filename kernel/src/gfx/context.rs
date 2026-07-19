use crate::drivers::nv_gpu::G_NV_STATE;
use crate::console::gop;

pub struct GfxCtx {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub fb_base: u64,
    pub gpu_active: bool,
}

static mut CTX: GfxCtx = GfxCtx {
    width: 0,
    height: 0,
    pitch: 0,
    fb_base: 0,
    gpu_active: false,
};

pub fn gfx_ctx() -> &'static GfxCtx {
    unsafe { &CTX }
}

pub unsafe fn gfx_init() {
    CTX.width = gop::get_width();
    CTX.height = gop::get_height();
    CTX.pitch = gop::FB_INFO.pitch;
    CTX.fb_base = gop::FB_INFO.base;
    CTX.gpu_active = gop::nv_active();

    if CTX.gpu_active {
        let base = if G_NV_STATE.double_buffer != 0 {
            if G_NV_STATE.front_buf != 0 {
                G_NV_STATE.backbuffer_offset
            } else {
                G_NV_STATE.fb_offset
            }
        } else {
            G_NV_STATE.fb_offset
        };
        CTX.fb_base = G_NV_STATE.bar1_base + base;
    }
}

pub unsafe fn gfx_fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    let ctx = &CTX;
    if !ctx.gpu_active {
        gop::fill_rect(x, y, w, h, color);
        return;
    }
    crate::drivers::nv_gpu::nv_gpu_fill_rect(x, y, w, h, color);
}

pub unsafe fn gfx_put_pixel(x: u32, y: u32, color: u32) {
    let ctx = &CTX;
    if !ctx.gpu_active {
        gop::put_pixel(x, y, color);
        return;
    }
    crate::drivers::nv_gpu::nv_gpu_put_pixel(x, y, color);
}

pub unsafe fn gfx_get_pixel(x: u32, y: u32) -> u32 {
    let ctx = &CTX;
    if !ctx.gpu_active {
        return gop::get_pixel(x, y);
    }
    crate::drivers::nv_gpu::nv_gpu_get_pixel(x, y)
}

pub unsafe fn gfx_vsync() {
    if CTX.gpu_active {
        crate::drivers::nv_gpu::nv_gpu_vsync();
    }
}

pub unsafe fn gfx_flip() {
    if CTX.gpu_active {
        crate::drivers::nv_gpu::nv_gpu_flip();
    }
}

pub fn gfx_width() -> u32 { unsafe { CTX.width } }
pub fn gfx_height() -> u32 { unsafe { CTX.height } }

pub unsafe fn gfx_fb_write(x: u32, y: u32, pixel: u32) {
    if x >= CTX.width || y >= CTX.height { return; }
    let off = CTX.fb_base + y as u64 * CTX.pitch as u64 + x as u64 * 4;
    core::ptr::write_volatile(off as *mut u32, pixel);
}

pub unsafe fn gfx_fb_read(x: u32, y: u32) -> u32 {
    if x >= CTX.width || y >= CTX.height { return 0; }
    let off = CTX.fb_base + y as u64 * CTX.pitch as u64 + x as u64 * 4;
    core::ptr::read_volatile(off as *const u32)
}
