#![no_std]

use core::ffi::c_void;
use core::ptr;

extern "C" {
    fn fat_exists(path: *const u8) -> i32;
    fn fat_read_file(path: *const u8, buf: *mut c_void, max: u32) -> i32;
    fn fat_get_file_size(path: *const u8) -> i32;
    fn pit_stall(us: u32);
    fn gop_get_width() -> u32;
    fn gop_get_height() -> u32;
    fn gop_fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32);
    fn gop_draw_string(x: u32, y: u32, fg: u32, bg: u32, s: *const u8);
}

pub fn lumieos_installed() -> bool {
    unsafe { fat_exists(b"/system/kernel.lkrn\0" as *const u8) == 1 }
}

fn copy_str_to_buf(buf: &mut [u8], s: &[u8]) {
    let mut i = 0;
    while i < buf.len() - 1 && i < s.len() {
        buf[i] = s[i];
        i += 1;
    }
    buf[i] = 0;
}

pub fn boot_display_msg(attempt: i32, extra: &[u8]) {
    let w = unsafe { gop_get_width() };
    let bg = crate::display::ld_make_color(0x00, 0x00, 0x80);
    let yellow = crate::display::ld_make_color(0xFF, 0xFF, 0x00);

    let mut msg: [u8; 128] = [0u8; 128];
    let mut mp = 0;

    let prefix = b"Boot attempt ";
    for &c in prefix { if mp < 127 { msg[mp] = c; mp += 1; } }

    let mut num_buf: [u8; 8] = [0u8; 8];
    unsafe { lumie_std::format::lumie_itoa(attempt as i64, num_buf.as_mut_ptr(), 10); }
    for &c in num_buf.iter() { if c == 0 { break; } if mp < 127 { msg[mp] = c; mp += 1; } }

    msg[mp] = b':'; mp += 1;
    msg[mp] = b' '; mp += 1;

    for &c in extra { if mp < 127 { msg[mp] = c; mp += 1; } }

    unsafe {
        gop_fill_rect(0, 0, w, 24, bg);
        gop_draw_string(8, 4, yellow, bg, msg.as_ptr());
    }
}

pub fn loader_check_files() -> i32 {
    let req: &[*const u8] = &[
        b"/system/kernel.lkrn\0" as *const u8,
        b"/drivers/kbd.ldrv\0" as *const u8,
        b"/drivers/fs.ldrv\0" as *const u8,
        b"/drivers/mouse.ldrv\0" as *const u8,
        b"/system/shell.lsh\0" as *const u8,
    ];
    let desc: &[*const u8] = &[
        b"Kernel\0" as *const u8,
        b"Keyboard Driver\0" as *const u8,
        b"Filesystem Driver\0" as *const u8,
        b"Mouse Driver\0" as *const u8,
        b"Shell\0" as *const u8,
    ];

    let bg = crate::display::ld_make_color(0x00, 0x00, 0x80);
    let white = crate::display::ld_make_color(0xFF, 0xFF, 0xFF);
    let green = crate::display::ld_make_color(0x00, 0xCC, 0x00);
    let red = crate::display::ld_make_color(0xFF, 0x00, 0x00);
    let yellow = crate::display::ld_make_color(0xFF, 0xFF, 0x00);
    let scr_w = unsafe { gop_get_width() };
    let scr_h = unsafe { gop_get_height() };

    crate::display::loader_drv_clear(bg);
    crate::display::loader_drv_draw_str(
        scr_w / 2 - 10 * 8, scr_h / 4, white, bg, b"Checking system files...",
    );

    let mut missing: i32 = 0;
    for i in 0..5 {
        let exists = unsafe { fat_exists(req[i]) };
        let color = if exists == 1 { green } else { red };
        if exists != 1 { missing += 1; }

        let mut buf: [u8; 128] = [0u8; 128];
        let mut bp = 0;
        buf[bp] = b' '; bp += 1; buf[bp] = b' '; bp += 1;
        if exists == 1 {
            buf[bp] = b'['; bp += 1; buf[bp] = b'O'; bp += 1;
            buf[bp] = b'K'; bp += 1; buf[bp] = b']'; bp += 1;
            buf[bp] = b' '; bp += 1;
        } else {
            buf[bp] = b'['; bp += 1; buf[bp] = b'-'; bp += 1;
            buf[bp] = b'-'; bp += 1; buf[bp] = b']'; bp += 1;
            buf[bp] = b' '; bp += 1;
        }

        let d = unsafe { core::ffi::CStr::from_ptr(desc[i] as *const i8) };
        let d_bytes = d.to_bytes();
        for &c in d_bytes { if bp < 127 { buf[bp] = c; bp += 1; } }

        crate::display::loader_drv_draw_str(
            scr_w / 4, scr_h / 4 + 24 + (i as u32) * 20, color, bg, &buf[..bp],
        );
        unsafe { pit_stall(100000); }
    }

    if missing == 0 {
        crate::display::loader_drv_draw_str(
            scr_w / 2 - 8 * 8, scr_h / 4 + 6 * 20, green, bg, b"All files present!",
        );
        unsafe { pit_stall(500000); }
        return 0;
    }

    let mut summary: [u8; 64] = [0u8; 64];
    unsafe {
        lumie_std::format::lumie_itoa(missing as i64, summary.as_mut_ptr(), 10);
    }
    let mut sp = 0;
    while sp < 64 && summary[sp] != 0 { sp += 1; }
    let suffix = b" file(s) missing!";
    for &c in suffix { if sp < 63 { summary[sp] = c; sp += 1; } }
    summary[sp] = 0;

    crate::display::loader_drv_draw_str(
        scr_w / 2 - 8 * 8, scr_h / 4 + 6 * 20, yellow, bg, &summary[..sp],
    );
    crate::display::loader_drv_draw_str(
        scr_w / 4, scr_h / 4 + 7 * 20, white, bg,
        b"Press ENTER to continue anyway, or any other key to return...",
    );

    let mut timeout = 0;
    loop {
        crate::input::loader_poll_mouse();
        if crate::input::loader_kbhit() {
            let c = crate::input::loader_getchar();
            if c == b'\n' as i32 { break; }
            return -1;
        }
        unsafe { pit_stall(10000); }
        timeout += 1;
        if timeout > 500 { break; }
    }
    0
}
