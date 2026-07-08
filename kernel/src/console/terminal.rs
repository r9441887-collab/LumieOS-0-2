
use core::ptr;
use lumie_std::LumieColor;
use super::gop;

const TAB_WIDTH: i32 = 4;

#[repr(C)]
pub struct Terminal {
    pub x: i32,
    pub y: i32,
    pub cols: i32,
    pub rows: i32,
    pub fg_color: u32,
    pub bg_color: u32,
    pub cursor_enabled: i32,
    pub screen_buf: *mut u8,
    pub color_buf: *mut u32,
    pub screen_buf_size: i32,
}

pub static mut TERM: Terminal = Terminal {
    x: 0,
    y: 0,
    cols: 0,
    rows: 0,
    fg_color: 0x00FFFFFF,
    bg_color: 0x000000AA,
    cursor_enabled: 1,
    screen_buf: ptr::null_mut(),
    color_buf: ptr::null_mut(),
    screen_buf_size: 0,
};

pub unsafe fn term_init() {
    TERM.cols = (gop::get_width() / 8) as i32;
    TERM.rows = (gop::get_height() / 16) as i32;
    TERM.screen_buf_size = TERM.cols * TERM.rows;
}

unsafe fn update_line_from_buf(row: i32) {
    let y = row * 16;
    gop::fill_rect(0, y as u32, gop::get_width(), 16, TERM.bg_color);
    for col in 0..TERM.cols {
        let idx = (row * TERM.cols + col) as isize;
        let c = if !TERM.screen_buf.is_null() {
            ptr::read(TERM.screen_buf.offset(idx))
        } else {
            b' '
        };
        let fg = if !TERM.color_buf.is_null() {
            ptr::read(TERM.color_buf.offset(idx))
        } else {
            TERM.fg_color
        };
        gop::draw_char((col * 8) as u32, y as u32, fg, TERM.bg_color, c);
    }
}

unsafe fn scroll_up() {
    for row in 1..TERM.rows {
        for col in 0..TERM.cols {
            let dst = (row - 1) * TERM.cols + col;
            let src = row * TERM.cols + col;
            if !TERM.screen_buf.is_null() {
                ptr::write(
                    TERM.screen_buf.offset(dst as isize),
                    ptr::read(TERM.screen_buf.offset(src as isize)),
                );
            }
            if !TERM.color_buf.is_null() {
                ptr::write(
                    TERM.color_buf.offset(dst as isize),
                    ptr::read(TERM.color_buf.offset(src as isize)),
                );
            }
        }
    }
    let last_row = TERM.rows - 1;
    for col in 0..TERM.cols {
        if !TERM.screen_buf.is_null() {
            ptr::write(
                TERM.screen_buf.offset((last_row * TERM.cols + col) as isize),
                b' ',
            );
        }
        if !TERM.color_buf.is_null() {
            ptr::write(
                TERM.color_buf.offset((last_row * TERM.cols + col) as isize),
                TERM.fg_color,
            );
        }
    }
    gop::fill_rect(0, 0, gop::get_width(), gop::get_height(), TERM.bg_color);
    for row in 0..TERM.rows {
        update_line_from_buf(row);
    }
}

pub unsafe fn term_newline() {
    TERM.x = 0;
    TERM.y += 1;
    if TERM.y >= TERM.rows {
        TERM.y = TERM.rows - 1;
        scroll_up();
    }
}

pub unsafe fn term_clear(bg: u32) {
    TERM.bg_color = bg;
    gop::fill_rect(0, 0, gop::get_width(), gop::get_height(), bg);
    TERM.x = 0;
    TERM.y = 0;
}

pub unsafe fn term_clear_color(c: LumieColor) {
    let raw = match c {
        LumieColor::Black => gop::gop_make_color(0, 0, 0),
        LumieColor::Blue => gop::gop_make_color(0x00, 0x00, 0xAA),
        LumieColor::Green => gop::gop_make_color(0, 0xAA, 0),
        LumieColor::Cyan => gop::gop_make_color(0, 0xAA, 0xAA),
        LumieColor::Red => gop::gop_make_color(0xAA, 0, 0),
        LumieColor::White => gop::gop_make_color(0xAA, 0xAA, 0xAA),
        LumieColor::DarkGray => gop::gop_make_color(0x55, 0x55, 0x55),
        _ => TERM.bg_color,
    };
    term_clear(raw);
}

pub unsafe fn term_putchar(c: u8) {
    match c {
        b'\n' => {
            term_newline();
            return;
        }
        b'\r' => {
            TERM.x = 0;
            return;
        }
        b'\x08' => {
            if TERM.x > 0 {
                TERM.x -= 1;
            }
            return;
        }
        b'\t' => {
            let next = (TERM.x / TAB_WIDTH + 1) * TAB_WIDTH;
            while TERM.x < next && TERM.x < TERM.cols {
                term_putchar(b' ');
            }
            return;
        }
        _ => {}
    }

    if TERM.x >= TERM.cols {
        term_newline();
    }

    let idx = (TERM.y * TERM.cols + TERM.x) as isize;
    if !TERM.screen_buf.is_null() && idx < TERM.screen_buf_size as isize {
        ptr::write(TERM.screen_buf.offset(idx), c);
    }
    if !TERM.color_buf.is_null() && idx < TERM.screen_buf_size as isize {
        ptr::write(TERM.color_buf.offset(idx), TERM.fg_color);
    }

    gop::draw_char(
        (TERM.x * 8) as u32,
        (TERM.y * 16) as u32,
        TERM.fg_color,
        TERM.bg_color,
        c,
    );
    TERM.x += 1;
}

pub unsafe fn term_write(s: *const u8) {
    if s.is_null() {
        return;
    }
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 {
            break;
        }
        term_putchar(c);
        i += 1;
    }
}

pub unsafe fn term_writeln(s: *const u8) {
    term_write(s);
    term_newline();
}

pub unsafe fn term_write_str(s: &str) {
    for &c in s.as_bytes() {
        term_putchar(c);
    }
}

pub unsafe fn term_writeln_str(s: &str) {
    term_write_str(s);
    term_newline();
}

pub unsafe fn term_set_fg(c: u32) {
    TERM.fg_color = c;
}

pub unsafe fn term_set_bg(c: u32) {
    TERM.bg_color = c;
}

pub unsafe fn term_set_pos(x: i32, y: i32) {
    if x >= 0 && x < TERM.cols {
        TERM.x = x;
    }
    if y >= 0 && y < TERM.rows {
        TERM.y = y;
    }
}

pub fn term_get_width() -> i32 {
    unsafe { TERM.cols }
}

pub fn term_get_height() -> i32 {
    unsafe { TERM.rows }
}

pub fn term_get_x() -> i32 {
    unsafe { TERM.x }
}

pub fn term_get_y() -> i32 {
    unsafe { TERM.y }
}

pub unsafe fn term_set_cursor(visible: bool) {
    TERM.cursor_enabled = visible as i32;
}

pub unsafe fn term_set_buf(buf: Option<&mut [u8]>, colors: Option<&mut [u32]>) {
    match buf {
        Some(b) => {
            TERM.screen_buf = b.as_mut_ptr();
            TERM.screen_buf_size = b.len() as i32;
        }
        None => {
            TERM.screen_buf = ptr::null_mut();
            TERM.screen_buf_size = 0;
        }
    }
    match colors {
        Some(c) => TERM.color_buf = c.as_mut_ptr(),
        None => TERM.color_buf = ptr::null_mut(),
    }
    if TERM.cols > 0 && TERM.rows > 0 {
        TERM.screen_buf_size = TERM.cols * TERM.rows;
    }
}
