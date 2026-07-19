
use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;
use crate::ffi::*;

pub fn lumieos_installed() -> bool {
    unsafe { fat_exists(b"/system/kernel.lkrn\0" as *const u8) == 1 }
}

pub struct BootEntry {
    pub boot_num: u16,
    pub desc: [u8; 64],
    pub desc_len: usize,
    pub is_lumie: bool,
}

pub fn read_boot_entries(entries: &mut [BootEntry]) -> usize {
    let st_ptr = unsafe { crate::input::get_ld_st() };
    if st_ptr.is_null() { return 0; }
    let rt = unsafe { (*st_ptr).runtime_services };
    if rt.is_null() { return 0; }

    let global_guid = &EFI_GLOBAL_VARIABLE_GUID as *const EfiGuid;
    let boot_order_name: [u16; 10] = [
        b'B' as u16, b'o' as u16, b'o' as u16, b't' as u16,
        b'O' as u16, b'r' as u16, b'd' as u16, b'e' as u16,
        b'r' as u16, 0,
    ];
    let mut boot_order_buf: [u16; 128] = [0u16; 128];
    let mut boot_order_size: u64 = 256;
    let mut attrs: u32 = 0;

    let gv = unsafe { (*rt).get_variable };
    let st = match gv {
        Some(g) => unsafe {
            g(boot_order_name.as_ptr() as *mut u16, global_guid, &mut attrs,
              &mut boot_order_size, boot_order_buf.as_mut_ptr() as *mut c_void)
        },
        None => return 0,
    };
    if st != 0 { return 0; }

    let count = (boot_order_size / 2) as usize;
    let mut found = 0usize;
    let hex_digits = b"0123456789ABCDEF";

    for i in 0..count {
        if found >= entries.len() { break; }
        let boot_num = boot_order_buf[i];

        let mut name_buf: [u16; 9] = [0u16; 9];
        name_buf[0] = b'B' as u16;
        name_buf[1] = b'o' as u16;
        name_buf[2] = b'o' as u16;
        name_buf[3] = b't' as u16;
        name_buf[4] = hex_digits[((boot_num >> 12) & 0xF) as usize] as u16;
        name_buf[5] = hex_digits[((boot_num >> 8) & 0xF) as usize] as u16;
        name_buf[6] = hex_digits[((boot_num >> 4) & 0xF) as usize] as u16;
        name_buf[7] = hex_digits[(boot_num & 0xF) as usize] as u16;
        name_buf[8] = 0;

        let mut desc_buf: [u16; 128] = [0u16; 128];
        let mut desc_size: u64 = 256;
        let st2 = unsafe {
            gv.unwrap()(name_buf.as_mut_ptr(), global_guid, ptr::null_mut(),
                       &mut desc_size, desc_buf.as_mut_ptr() as *mut c_void)
        };
        if st2 != 0 || desc_size < 4 { continue; }

        /* EFI_LOAD_OPTION: skip first 4 bytes (Attributes) + 2 bytes (FilePathListLength) */
        let desc_words = (desc_size as usize - 6) / 2;
        if desc_words == 0 { continue; }

        let entry = &mut entries[found];
        entry.boot_num = boot_num;
        entry.desc_len = 0;
        entry.is_lumie = false;

        /* Copy UTF-16 description, converting to ASCII */
        let mut lumie_match = true;
        let lumie_name: [u16; 8] = [
            b'L' as u16, b'u' as u16, b'm' as u16, b'i' as u16,
            b'e' as u16, b'O' as u16, b'S' as u16, 0,
        ];
        for j in 0..desc_words.min(63) {
            let ch = unsafe { *ptr::addr_of!(desc_buf[3 + j]) };
            if ch == 0 { break; }
            if j < 63 {
                entry.desc[j] = ch as u8;
                entry.desc_len = j + 1;
            }
            if j < 8 && ch != lumie_name[j] { lumie_match = false; }
        }
        entry.desc[entry.desc_len] = 0;
        entry.is_lumie = lumie_match && entry.desc_len <= 8;
        found += 1;
    }
    found
}

pub fn show_boot_menu(entries: &[BootEntry]) -> i32 {
    let scr_w = unsafe { gop_get_width() };
    let scr_h = unsafe { gop_get_height() };

    let bg = crate::display::ld_make_color(0x00, 0x00, 0x80);
    let white = crate::display::ld_make_color(0xFF, 0xFF, 0xFF);
    let cyan = crate::display::ld_make_color(0x00, 0xFF, 0xFF);
    let yellow = crate::display::ld_make_color(0xFF, 0xFF, 0x00);
    let lcyan = crate::display::ld_make_color(0x55, 0xFF, 0xFF);
    let green = crate::display::ld_make_color(0x00, 0xCC, 0x00);
    let dim = crate::display::ld_make_color(0x88, 0x88, 0x88);

    let mut selected: i32 = 0;
    for i in 0..entries.len() {
        if entries[i].is_lumie { selected = i as i32; break; }
    }

    let page_size: i32 = 8;
    let mut page_offset: i32 = 0;

    loop {
        crate::display::loader_drv_clear(bg);

        let title_y = scr_h / 6;
        crate::display::loader_drv_draw_str(scr_w / 2 - 4 * 8, title_y, lcyan, bg, b"LumieOS");
        crate::display::loader_drv_draw_str(scr_w / 2 - 12 * 8, title_y + 24, white, bg, b"Select Operating System:");

        if entries.len() > page_size as usize {
            crate::display::loader_drv_draw_str(
                scr_w / 2 - 7 * 8, title_y + 48, yellow, bg, b"[more above]",
            );
        }

        let list_y = title_y + if entries.len() > page_size as usize { 72 } else { 52 };
        let visible_start = page_offset as usize;
        let visible_end = (visible_start + page_size as usize).min(entries.len());

        for idx in visible_start..visible_end {
            let rel = (idx - visible_start) as u32;
            let y = list_y + rel * 24;
            let entry = &entries[idx];

            if idx as i32 == selected {
                crate::display::loader_drv_draw_str(
                    scr_w / 4 - 16, y, yellow, bg, b">",
                );
            }

            let mut line: [u8; 80] = [0u8; 80];
            let mut lp = 0;
            /* Number */
            let num_str = b"12345678";
            if idx < 8 {
                line[lp] = num_str[idx]; lp += 1;
                line[lp] = b'.'; lp += 1;
                line[lp] = b' '; lp += 1;
            }
            /* Description */
            for j in 0..entry.desc_len {
                if lp < 79 { line[lp] = entry.desc[j]; lp += 1; }
            }
            /* Tag LumieOS */
            if entry.is_lumie {
                let tag = b" (LumieOS)";
                for &c in tag { if lp < 79 { line[lp] = c; lp += 1; } }
            }
            line[lp] = 0;

            let color = if idx as i32 == selected { yellow } else { white };
            crate::display::loader_drv_draw_str(
                scr_w / 4, y, color, bg, &line[..lp],
            );
        }

        if entries.len() > page_size as usize && (visible_end as i32) < entries.len() as i32 {
            crate::display::loader_drv_draw_str(
                scr_w / 2 - 7 * 8, list_y + (page_size as u32) * 24 + 8, yellow, bg, b"[more below]",
            );
        }

        let hint_y = scr_h - 48;
        crate::display::loader_drv_draw_str(
            scr_w / 4, hint_y, dim, bg,
            b"ENTER: boot   UP/DOWN: navigate   ESC: skip",
        );

        loop {
            crate::input::loader_poll_mouse();

            /* Mouse click */
            let mut cx = 0i32;
            let mut cy = 0i32;
            if crate::input::loader_get_click(&mut cx, &mut cy) {
                for idx in visible_start..visible_end {
                    let rel = (idx - visible_start) as u32;
                    let item_y = list_y + rel * 24;
                    let item_x = scr_w / 4 - 16;
                    let item_w = 480u32;
                    let item_h = 22u32;
                    if cx >= item_x as i32 && cx < (item_x + item_w) as i32 && cy >= item_y as i32 && cy < (item_y + item_h) as i32 {
                        selected = idx as i32;
                        /* Double-click = confirm */
                        return selected;
                    }
                }
            }

            if crate::input::loader_kbhit() {
                let c = crate::input::loader_getchar();
                if c == b'\n' as i32 || c == 0x0D {
                    return selected;
                }
                if c == 0xE1 { /* UP */
                    if selected > 0 {
                        selected -= 1;
                        if (selected as i32) < page_offset {
                            page_offset = selected;
                        }
                    }
                    break;
                }
                if c == 0xE2 { /* DOWN */
                    if (selected as usize) < entries.len() - 1 {
                        selected += 1;
                        if (selected as i32) >= page_offset + page_size {
                            page_offset = selected - page_size + 1;
                        }
                    }
                    break;
                }
                if c == 0x1B { /* ESC - skip to LumieOS boot */
                    for i in 0..entries.len() {
                        if entries[i].is_lumie { return i as i32; }
                    }
                    return 0;
                }
            }
            unsafe { pit_stall(10000); }
        }
    }
}

pub fn set_boot_next_and_reboot(boot_num: u16) {
    let st_ptr = unsafe { crate::input::get_ld_st() };
    if st_ptr.is_null() { return; }
    let rt = unsafe { (*st_ptr).runtime_services };
    if rt.is_null() { return; }

    let global_guid = &EFI_GLOBAL_VARIABLE_GUID as *const EfiGuid;
    let boot_next_name: [u16; 9] = [
        b'B' as u16, b'o' as u16, b'o' as u16, b't' as u16,
        b'N' as u16, b'e' as u16, b'x' as u16, b't' as u16, 0,
    ];

    if let Some(sv) = unsafe { (*rt).set_variable } {
        unsafe {
            sv(boot_next_name.as_ptr() as *mut u16, global_guid,
               0x01 | 0x04 | 0x08, /* NON_VOLATILE | BOOTSERVICE_ACCESS | RUNTIME_ACCESS */
               2, &(boot_num as u16) as *const u16 as *mut c_void);
        }
    }

    unsafe { lumie_reboot(); }
}

pub fn detect_other_os() -> bool {
    let st_ptr = unsafe { crate::input::get_ld_st() };
    if st_ptr.is_null() { return false; }
    let rt = unsafe { (*st_ptr).runtime_services };
    if rt.is_null() { return false; }

    let global_guid = &EFI_GLOBAL_VARIABLE_GUID as *const EfiGuid;
    let boot_order_name: [u16; 10] = [
        b'B' as u16, b'o' as u16, b'o' as u16, b't' as u16,
        b'O' as u16, b'r' as u16, b'd' as u16, b'e' as u16,
        b'r' as u16, 0,
    ];
    let mut boot_order_buf: [u16; 128] = [0u16; 128];
    let mut boot_order_size: u64 = 256;
    let mut attrs: u32 = 0;

    let gv = unsafe { (*rt).get_variable };
    let st = match gv {
        Some(g) => unsafe {
            g(boot_order_name.as_ptr() as *mut u16, global_guid, &mut attrs,
              &mut boot_order_size, boot_order_buf.as_mut_ptr() as *mut c_void)
        },
        None => return false,
    };
    if st != 0 { return false; }

    let count = (boot_order_size / 2) as usize;
    let hex_digits = b"0123456789ABCDEF";

    for i in 0..count {
        let boot_num = boot_order_buf[i];
        let mut name_buf: [u16; 9] = [0u16; 9];
        name_buf[0] = b'B' as u16;
        name_buf[1] = b'o' as u16;
        name_buf[2] = b'o' as u16;
        name_buf[3] = b't' as u16;
        name_buf[4] = hex_digits[((boot_num >> 12) & 0xF) as usize] as u16;
        name_buf[5] = hex_digits[((boot_num >> 8) & 0xF) as usize] as u16;
        name_buf[6] = hex_digits[((boot_num >> 4) & 0xF) as usize] as u16;
        name_buf[7] = hex_digits[(boot_num & 0xF) as usize] as u16;
        name_buf[8] = 0;

        let mut desc_buf: [u16; 128] = [0u16; 128];
        let mut desc_size: u64 = 256;
        let st2 = unsafe {
            gv.unwrap()(name_buf.as_mut_ptr(), global_guid, ptr::null_mut(),
                       &mut desc_size, desc_buf.as_mut_ptr() as *mut c_void)
        };
        if st2 == 0 && desc_size > 6 {
            let desc_words = (desc_size as usize - 6) / 2;
            let lumie_name: [u16; 8] = [
                b'L' as u16, b'u' as u16, b'm' as u16, b'i' as u16,
                b'e' as u16, b'O' as u16, b'S' as u16, 0,
            ];
            let mut is_lumie = true;
            for j in 0..desc_words.min(8) {
                let ch = unsafe { *ptr::addr_of!(desc_buf[3 + j]) };
                if j < 8 && ch != lumie_name[j] { is_lumie = false; break; }
            }
            if !is_lumie && desc_words > 0 {
                return true;
            }
        }
    }
    false
}

#[allow(dead_code)]
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

    if mp < 127 { msg[mp] = b':'; mp += 1; }
    if mp < 127 { msg[mp] = b' '; mp += 1; }

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
