use crate::editor::{EditorState, MAX_LINE_LEN, MAX_LINES};

pub(crate) fn str_len(s: &[u8]) -> i32 {
    for i in 0..s.len() {
        if s[i] == 0 {
            return i as i32;
        }
    }
    s.len() as i32
}

pub(crate) fn str_cpy(dst: &mut [u8], src: &[u8]) {
    let mut i = 0;
    while i < dst.len() {
        if i < src.len() {
            dst[i] = src[i];
            if src[i] == 0 {
                break;
            }
        } else {
            dst[i] = 0;
            break;
        }
        i += 1;
    }
}

pub(crate) fn str_cat(dst: &mut [u8], src: &[u8]) {
    let start = str_len(dst) as usize;
    if start >= dst.len() {
        return;
    }
    let mut j = start;
    for i in 0..src.len() {
        if j >= dst.len() {
            break;
        }
        dst[j] = src[i];
        if src[i] == 0 {
            break;
        }
        j += 1;
    }
    if j < dst.len() {
        dst[j] = 0;
    }
}

pub(crate) fn i_to_a(mut value: i32, buf: &mut [u8], base: i32) -> i32 {
    if value == 0 {
        buf[0] = b'0';
        buf[1] = 0;
        return 1;
    }
    let mut i = 0usize;
    let mut digits = [0u8; 16];
    while value > 0 && i < 16 {
        let d = value % base;
        digits[i] = if d < 10 {
            b'0' + d as u8
        } else {
            b'A' + (d - 10) as u8
        };
        value /= base;
        i += 1;
    }
    let mut j = 0i32;
    while i > 0 {
        i -= 1;
        if (j as usize) < buf.len().saturating_sub(1) {
            buf[j as usize] = digits[i];
            j += 1;
        }
    }
    buf[j as usize] = 0;
    j
}

pub fn insert_char(ed: &mut EditorState, c: u8) {
    if ed.cursor_y >= ed.num_lines {
        return;
    }
    let li = ed.cursor_y as usize;
    let len = str_len(&ed.lines[li]);
    if len < (MAX_LINE_LEN - 1) as i32 {
        let mut i = len;
        while i >= ed.cursor_x {
            ed.lines[li][(i + 1) as usize] = ed.lines[li][i as usize];
            i -= 1;
        }
        ed.lines[li][ed.cursor_x as usize] = c;
        ed.cursor_x += 1;
        ed.modified = true;
    }
}

pub fn delete_char(ed: &mut EditorState) {
    if ed.cursor_y >= ed.num_lines {
        return;
    }
    let li = ed.cursor_y as usize;
    let len = str_len(&ed.lines[li]);

    if ed.cursor_x > 0 {
        let mut i = ed.cursor_x - 1;
        while i < len {
            ed.lines[li][i as usize] = ed.lines[li][(i + 1) as usize];
            i += 1;
        }
        ed.cursor_x -= 1;
        ed.modified = true;
    } else if ed.cursor_y > 0 {
        let prev_li = (ed.cursor_y - 1) as usize;
        let prev_len = str_len(&ed.lines[prev_li]);
        let mut i = 0i32;
        while i < len && (prev_len + i) as usize < MAX_LINE_LEN - 1 {
            ed.lines[prev_li][(prev_len + i) as usize] = ed.lines[li][i as usize];
            i += 1;
        }
        let mut joined = prev_len + len;
        if joined >= MAX_LINE_LEN as i32 - 1 {
            joined = MAX_LINE_LEN as i32 - 1;
        }
        ed.lines[prev_li][joined as usize] = 0;

        let mut i = ed.cursor_y as usize;
        while i < (ed.num_lines - 1) as usize {
            str_cpy(&mut ed.lines[i], &ed.lines[i + 1]);
            i += 1;
        }
        ed.num_lines -= 1;
        ed.cursor_y -= 1;
        ed.cursor_x = prev_len;
        ed.modified = true;
    }
}

pub fn newline(ed: &mut EditorState) {
    if ed.num_lines as usize >= MAX_LINES {
        return;
    }

    let li = ed.cursor_y as usize;
    let len = str_len(&ed.lines[li]);

    let mut i = ed.num_lines as usize;
    while i > ed.cursor_y as usize + 1 {
        str_cpy(&mut ed.lines[i], &ed.lines[i - 1]);
        i -= 1;
    }

    let mut j = 0i32;
    let mut i = ed.cursor_x;
    while i < len && j < MAX_LINE_LEN as i32 - 1 {
        ed.lines[ed.cursor_y as usize + 1][j as usize] = ed.lines[li][i as usize];
        j += 1;
        i += 1;
    }
    ed.lines[ed.cursor_y as usize + 1][j as usize] = 0;
    ed.lines[li][ed.cursor_x as usize] = 0;

    ed.num_lines += 1;
    ed.cursor_y += 1;
    ed.cursor_x = 0;
    ed.modified = true;
}

pub fn save(ed: &mut EditorState, services: &dyn crate::EditorServices) {
    let mut total_size = 0i32;
    for i in 0..ed.num_lines as usize {
        let len = str_len(&ed.lines[i]);
        total_size += len + 1;
    }

    let mut buf = [0u8; 64 * 1024];
    let max_sz: i32 = if total_size + 1 < 64 * 1024 {
        total_size + 1
    } else {
        64 * 1024
    };

    if total_size > max_sz - 1 {
        status_msg(ed, services, "File too large to save!");
        return;
    }

    let mut pos = 0i32;
    for i in 0..ed.num_lines as usize {
        let len = str_len(&ed.lines[i]);
        let mut j = 0i32;
        while j < len {
            buf[pos as usize] = ed.lines[i][j as usize];
            pos += 1;
            j += 1;
        }
        buf[pos as usize] = b'\n';
        pos += 1;
    }

    let fname = fname_to_str(&ed.filename);
    let result = services.fs_write(fname, &buf[..pos as usize]);

    if result == 0 {
        ed.modified = false;
        status_msg(ed, services, "File saved!");
    } else {
        status_msg(ed, services, "Error: read-only filesystem");
    }
}

pub fn status_msg(ed: &EditorState, services: &dyn crate::EditorServices, msg: &str) {
    let cols = if ed.windowed {
        ed.win_w / 8
    } else {
        services.term_get_width()
    };

    let fg = services.gop_make_color(0xFF, 0xFF, 0xFF);
    let bg = services.gop_make_color(0x00, 0x00, 0x80);

    let (x0, yy): (i32, i32) = if ed.windowed {
        (4, ed.win_h - 16)
    } else {
        let yy = (services.term_get_height() - 1) * 16;
        services.gop_fill_rect(0, yy as u32, services.gop_get_width(), 16, bg);
        (0, yy)
    };

    let display_cols = if cols < 0 { 80 } else { cols };
    services.gop_fill_rect(
        x0 as u32,
        yy as u32,
        (display_cols * 8) as u32,
        16,
        bg,
    );

    let msg_bytes = msg.as_bytes();
    let max_len = display_cols.min(msg_bytes.len() as i32);
    for i in 0..max_len {
        services.gop_draw_char(
            (x0 + i * 8) as u32,
            yy as u32,
            fg,
            bg,
            msg_bytes[i as usize],
        );
    }
}

fn fname_to_str(fname: &[u8; MAX_LINE_LEN]) -> &str {
    let len = str_len(fname) as usize;
    core::str::from_utf8(&fname[..len]).unwrap_or("")
}
