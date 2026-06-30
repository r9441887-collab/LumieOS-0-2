#![no_std]

use core::ptr;
use crate::uefi::*;

pub fn ld_make_color(r: u8, g: u8, b: u8) -> u32 {
    let fb = unsafe { crate::gop_get_fb() };
    if !fb.is_null() {
        let fmt = unsafe { (*fb).pixel_format };
        if fmt == 0 {
            return (r as u32) | ((g as u32) << 8) | ((b as u32) << 16);
        }
    }
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub fn loader_drv_clear(color: u32) {
    let w = unsafe { crate::gop_get_width() };
    let h = unsafe { crate::gop_get_height() };
    unsafe { crate::gop_fill_rect(0, 0, w, h, color); }
}

pub fn loader_drv_fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) {
    unsafe { crate::gop_fill_rect(x, y, w, h, color); }
}

pub fn loader_drv_draw_str(x: u32, y: u32, fg: u32, bg: u32, s: &[u8]) {
    let mut null_term: [u8; 256] = [0u8; 256];
    let len = if s.len() > 255 { 255 } else { s.len() };
    null_term[..len].copy_from_slice(&s[..len]);
    null_term[len] = 0;
    unsafe { crate::gop_draw_string(x, y, fg, bg, null_term.as_ptr()); }
}

pub fn loader_drv_progress(x: u32, y: u32, w: u32, h: u32, fg: u32, bg: u32, pct: i32) {
    let pct = if pct > 100 { 100 } else if pct < 0 { 0 } else { pct };
    loader_drv_fill_rect(x, y, w, h, bg);
    if pct > 0 {
        let mut filled = (w as i32 * pct) / 100;
        if filled < 1 { filled = 1; }
        loader_drv_fill_rect(x, y, filled as u32, h, fg);
    }
}

const CURSOR_BITS: [[u8; 2]; 16] = [
    [0x80, 0x00], [0xC0, 0x00], [0xE0, 0x00], [0xF0, 0x00],
    [0xF8, 0x00], [0xFC, 0x00], [0xFE, 0x00], [0xFF, 0x00],
    [0xFF, 0x80], [0xFE, 0xC0], [0xFC, 0xE0], [0xF0, 0x70],
    [0xE0, 0x38], [0xC0, 0x1C], [0x80, 0x0E], [0x00, 0x04],
];

pub static mut CURSOR_BG: [[u32; 16]; 16] = [[0u32; 16]; 16];
pub static mut CURSOR_X: i32 = 0;
pub static mut CURSOR_Y: i32 = 0;

pub fn loader_cursor_restore() {
    let w = unsafe { crate::gop_get_width() };
    let h = unsafe { crate::gop_get_height() };
    unsafe {
        for row in 0..16 {
            for col in 0..16 {
                let px = CURSOR_X + col;
                let py = CURSOR_Y + row;
                if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 { continue; }
                crate::gop_put_pixel(px as u32, py as u32, CURSOR_BG[row as usize][col as usize]);
            }
        }
    }
}

pub fn loader_cursor_draw() {
    let w = unsafe { crate::gop_get_width() };
    let h = unsafe { crate::gop_get_height() };
    let blue = ld_make_color(0x00, 0x55, 0xFF);
    let white = ld_make_color(0xFF, 0xFF, 0xFF);
    unsafe {
        for row in 0..16 {
            for col in 0..16 {
                let px = CURSOR_X + col;
                let py = CURSOR_Y + row;
                if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 { continue; }
                CURSOR_BG[row as usize][col as usize] = crate::gop_get_pixel(px as u32, py as u32);
                let bits = ((CURSOR_BITS[row as usize][0] as u16) << 8) | CURSOR_BITS[row as usize][1] as u16;
                if bits & (0x8000u16 >> col) != 0 {
                    let color = if row == 0 || col == 0 { white } else { blue };
                    crate::gop_put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

pub fn loader_boot_screen() {
    let scr_w = 1024u32;
    let scr_h = 768u32;

    if !crate::boot::lumieos_installed() { return; }

    let bg = ld_make_color(0x00, 0x00, 0x80);
    let white = ld_make_color(0xFF, 0xFF, 0xFF);
    let cyan = ld_make_color(0x00, 0xFF, 0xFF);
    let lcyan = ld_make_color(0x55, 0xFF, 0xFF);
    let dkcyan = ld_make_color(0x00, 0x88, 0x88);
    let green = ld_make_color(0x00, 0xCC, 0x00);
    let yellow = ld_make_color(0xFF, 0xFF, 0x00);

    unsafe {
        CURSOR_X = (scr_w / 2) as i32;
        CURSOR_Y = (scr_h / 2) as i32;
    }

    loader_drv_clear(bg);

    let logo_y = scr_h / 5;
    let line_h = 20u32;

    loader_drv_draw_str(scr_w / 2 - 4 * 8, logo_y, lcyan, bg, b"LumieOS");
    loader_drv_draw_str(scr_w / 2 - 8 * 8, logo_y + line_h, white, bg, b"(Windows Edition)");

    let bar_y = logo_y + 3 * line_h;
    let bar_w = scr_w / 2;
    let bar_x = (scr_w - bar_w) / 2;
    let status_y = bar_y + 24;
    let spin_y = status_y + 24;

    let mut pct: i32 = 0;
    let mut phase: usize = 0;
    let mut spin: i32 = 0;
    let spinner: &[u8; 4] = b"|/-\\";
    let phases: &[&[u8]; 6] = &[
        b"Checking filesystem...", b"Loading keyboard driver...",
        b"Loading mouse driver...", b"Loading filesystem driver...",
        b"Loading kernel...", b"Loading shell...",
    ];

    while pct < 100 {
        pct += 1;
        if pct > 100 { pct = 100; }

        let spin_ch = [spinner[(spin as usize) & 3], 0u8];
        loader_drv_fill_rect(bar_x + bar_w + 8, spin_y - 4, 16, 16, bg);
        loader_drv_draw_str(bar_x + bar_w + 8, spin_y - 4, lcyan, bg, &spin_ch);
        spin += 1;

        let mut pct_str: [u8; 8] = [0u8; 8];
        unsafe { lumie_std::format::lumie_itoa(pct as i64, pct_str.as_mut_ptr(), 10); }
        let mut plen = 0;
        while plen < 8 && pct_str[plen] != 0 { plen += 1; }
        loader_drv_fill_rect(bar_x, spin_y, bar_w, 16, bg);
        loader_drv_draw_str(bar_x, spin_y, yellow, bg, &pct_str[..plen]);
        loader_drv_draw_str(bar_x + 16, spin_y, white, bg, b"% complete");

        let new_phase = ((pct * 6) / 100) as usize;
        let new_phase = if new_phase > 5 { 5 } else { new_phase };
        if new_phase != phase {
            phase = new_phase;
            if phase < phases.len() {
                loader_drv_fill_rect(bar_x, status_y, bar_w, 16, bg);
                loader_drv_draw_str(bar_x, status_y, cyan, bg, phases[phase]);
            }
        }

        loader_drv_progress(bar_x, bar_y, bar_w, 16, green, dkcyan, pct);
        if crate::input::loader_kbhit() {
            let _c = crate::input::loader_getchar();
            break;
        }
        unsafe { crate::pit_stall(20000); }
    }

    loader_drv_fill_rect(bar_x, bar_y, bar_w, 16, bg);
    loader_drv_fill_rect(bar_x, status_y, bar_w, 16, bg);
    loader_drv_fill_rect(bar_x, spin_y, bar_w + 32, 16, bg);
}
