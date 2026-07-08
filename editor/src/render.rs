use crate::buffer;
use crate::editor::EditorState;

pub fn render(ed: &EditorState, services: &dyn crate::EditorServices) {
    let ox = ed.render_offx;
    let oy = ed.render_offy;

    let (x0, y0, rows, cols): (i32, i32, i32, i32) = if ed.windowed {
        let x0 = ox + 4;
        let y0 = oy + 4;
        let rows = (ed.win_h - 20) / 16;
        let cols = (ed.win_w - 8) / 8;
        (x0, y0, rows, cols)
    } else {
        let rows = services.term_get_height() - 1;
        let cols = services.term_get_width();
        (0, 0, rows, cols)
    };

    let status_row = y0 + rows * 16;
    let bg = services.gop_make_color(0, 0, 0);
    let c55 = services.gop_make_color(0x55, 0x55, 0x55);
    let fg_line = services.gop_make_color(0xAA, 0xAA, 0xAA);
    let fg_status = services.gop_make_color(0xFF, 0xFF, 0xFF);
    let bg_status = services.gop_make_color(0x00, 0x00, 0x80);

    services.gop_fill_rect(x0 as u32, y0 as u32, (cols * 8) as u32, (rows * 16) as u32, bg);

    for row in 0..rows {
        let line_idx = ed.offset_y + row;
        let yy = y0 + row * 16;

        if line_idx < 0 || line_idx >= ed.num_lines {
            for c in 0..cols {
                services.gop_draw_char(
                    (x0 + c * 8) as u32, yy as u32, c55, bg, b' ',
                );
            }
            continue;
        }

        let li = line_idx as usize;
        let mut ln_buf = [0u8; 16];
        let llen = buffer::i_to_a(line_idx + 1, &mut ln_buf, 10);
        let mut cpos = 0i32;

        for _c in 0..4 - llen {
            if cpos >= cols {
                break;
            }
            services.gop_draw_char((x0 + cpos * 8) as u32, yy as u32, c55, bg, b' ');
            cpos += 1;
        }
        for c in 0..llen {
            if cpos >= cols {
                break;
            }
            services.gop_draw_char(
                (x0 + cpos * 8) as u32, yy as u32, c55, bg, ln_buf[c as usize],
            );
            cpos += 1;
        }
        if cpos < cols {
            services.gop_draw_char((x0 + cpos * 8) as u32, yy as u32, c55, bg, b' ');
            cpos += 1;
        }
        if cpos < cols {
            services.gop_draw_char((x0 + cpos * 8) as u32, yy as u32, c55, bg, b'|');
            cpos += 1;
        }
        if cpos < cols {
            services.gop_draw_char((x0 + cpos * 8) as u32, yy as u32, c55, bg, b' ');
            cpos += 1;
        }

        let line_len = buffer::str_len(&ed.lines[li]);
        let start = ed.offset_x;
        let mut end = start + (cols - 6);
        if end > line_len {
            end = line_len;
        }

        let mut ci = start;
        while ci < end && cpos < cols {
            let ch = ed.lines[li][ci as usize];
            if ch == b'\t' {
                let stop = cpos + crate::editor::TAB_STOP;
                while cpos < stop && cpos < cols {
                    services.gop_draw_char(
                        (x0 + cpos * 8) as u32, yy as u32, fg_line, bg, b' ',
                    );
                    cpos += 1;
                }
            } else {
                services.gop_draw_char(
                    (x0 + cpos * 8) as u32, yy as u32, fg_line, bg, ch,
                );
                cpos += 1;
            }
            ci += 1;
        }

        while cpos < cols {
            services.gop_draw_char((x0 + cpos * 8) as u32, yy as u32, fg_line, bg, b' ');
            cpos += 1;
        }
    }

    if ed.windowed {
        services.gop_fill_rect(
            x0 as u32, status_row as u32, (cols * 8) as u32, 16, bg_status,
        );
    } else {
        services.gop_fill_rect(
            0, status_row as u32, services.gop_get_width(), 16, bg_status,
        );
    }

    let mut status = [0u8; 128];
    let fname_len = buffer::str_len(&ed.filename);
    let mut si = 0usize;
    let mut fi = 0usize;
    while fi < fname_len as usize && si < status.len() - 1 && ed.filename[fi] != 0 {
        status[si] = ed.filename[fi];
        si += 1;
        fi += 1;
    }
    let dash: &[u8] = b" - ";
    for &b in dash {
        if si >= status.len() - 1 {
            break;
        }
        status[si] = b;
        si += 1;
    }
    let mut sz_buf = [0u8; 16];
    let sz_len = buffer::i_to_a(ed.num_lines, &mut sz_buf, 10);
    let mut szi = 0i32;
    while szi < sz_len && si < status.len() - 1 {
        status[si] = sz_buf[szi as usize];
        si += 1;
        szi += 1;
    }
    let space: &[u8] = b" lines";
    for &b in space {
        if si >= status.len() - 1 {
            break;
        }
        status[si] = b;
        si += 1;
    }
    if ed.modified {
        let mod_str: &[u8] = b" [MODIFIED]";
        for &b in mod_str {
            if si >= status.len() - 1 {
                break;
            }
            status[si] = b;
            si += 1;
        }
    }
    status[si] = 0;

    let mut sti = 0i32;
    while sti < cols && status[sti as usize] != 0 {
        services.gop_draw_char(
            (x0 + sti * 8) as u32, status_row as u32, fg_status, bg_status,
            status[sti as usize],
        );
        sti += 1;
    }
}
