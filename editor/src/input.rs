use crate::buffer;
use crate::editor::EditorState;

const KBD_UP: i32 = 0xE0;
const KBD_DOWN: i32 = 0xE1;
const KBD_LEFT: i32 = 0xE2;
const KBD_RIGHT: i32 = 0xE3;
const KBD_HOME: i32 = 0xE4;
const KBD_END: i32 = 0xE5;
const KBD_PGUP: i32 = 0xE6;
const KBD_PGDN: i32 = 0xE7;

pub fn handle_key(ed: &mut EditorState, c: i32, services: &dyn crate::EditorServices) -> bool {
    match c {
        KBD_UP => {
            if ed.cursor_y > 0 {
                ed.cursor_y -= 1;
            }
            true
        }
        KBD_DOWN => {
            if ed.cursor_y < ed.num_lines - 1 {
                ed.cursor_y += 1;
            }
            true
        }
        KBD_LEFT => {
            if ed.cursor_x > 0 {
                ed.cursor_x -= 1;
            } else if ed.cursor_y > 0 {
                ed.cursor_y -= 1;
                ed.cursor_x = buffer::str_len(&ed.lines[ed.cursor_y as usize]);
            }
            true
        }
        KBD_RIGHT => {
            let len = buffer::str_len(&ed.lines[ed.cursor_y as usize]);
            if ed.cursor_x < len {
                ed.cursor_x += 1;
            } else if ed.cursor_y < ed.num_lines - 1 {
                ed.cursor_y += 1;
                ed.cursor_x = 0;
            }
            true
        }
        KBD_HOME => {
            ed.cursor_x = 0;
            true
        }
        KBD_END => {
            ed.cursor_x = buffer::str_len(&ed.lines[ed.cursor_y as usize]);
            true
        }
        KBD_PGUP => {
            ed.cursor_y -= 20;
            if ed.cursor_y < 0 {
                ed.cursor_y = 0;
            }
            true
        }
        KBD_PGDN => {
            ed.cursor_y += 20;
            if ed.cursor_y >= ed.num_lines {
                ed.cursor_y = ed.num_lines - 1;
            }
            true
        }
        0x11 => {
            if ed.modified && !ed.pending_quit {
                ed.pending_quit = true;
                buffer::status_msg(
                    ed,
                    services,
                    "Unsaved changes! Press Ctrl+S to save, Ctrl+Q again to quit",
                );
            } else {
                ed.done = true;
            }
            true
        }
        0x13 => {
            buffer::save(ed, services);
            ed.pending_quit = false;
            true
        }
        b'\n' as i32 => {
            buffer::newline(ed);
            true
        }
        b'\b' as i32 | 0x7F => {
            buffer::delete_char(ed);
            true
        }
        b'\t' as i32 => {
            for _ in 0..crate::editor::TAB_STOP {
                buffer::insert_char(ed, b' ');
            }
            true
        }
        _ => {
            if c >= 0x20 && c <= 0x7E {
                buffer::insert_char(ed, c as u8);
                true
            } else {
                false
            }
        }
    }
}

pub fn update_scroll(ed: &mut EditorState, services: &dyn crate::EditorServices) {
    let cols = if ed.windowed {
        ed.win_w / 8
    } else {
        services.term_get_width()
    };
    let rows = if ed.windowed {
        (ed.win_h - 20) / 16
    } else {
        services.term_get_height() - 1
    };

    let len = buffer::str_len(&ed.lines[ed.cursor_y as usize]);
    if ed.cursor_x > len {
        ed.cursor_x = len;
    }

    if ed.cursor_x < ed.offset_x {
        ed.offset_x = ed.cursor_x;
    }
    if ed.cursor_x >= ed.offset_x + cols - 6 {
        ed.offset_x = ed.cursor_x - cols + 7;
    }
    if ed.cursor_y < ed.offset_y {
        ed.offset_y = ed.cursor_y;
    }
    if ed.cursor_y >= ed.offset_y + rows {
        ed.offset_y = ed.cursor_y - rows + 1;
    }
}
