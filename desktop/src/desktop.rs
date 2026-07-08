#![allow(static_mut_refs)]
use crate::render;
use crate::widgets::Window;
use crate::DesktopServices;

pub const TASKBAR_H: i32 = 40;
pub const ICON_W: i32 = 64;
pub const ICON_H: i32 = 64;
const ICON_GAP: i32 = 100;
const ICONS_TOP: i32 = 60;
pub const MAX_WINS: usize = 16;
pub const TITLE_H: i32 = 22;
pub const RESIZE_H: i32 = 8;
pub const RESIZE_W: i32 = 8;
const WIN_MIN_W: i32 = 200;
const WIN_MIN_H: i32 = 120;
const NUM_ICONS: usize = 4;
#[allow(dead_code)]
const CUR_W: i32 = 12;
const CUR_H: i32 = 16;
const CTX_N: usize = 4;
const CTX_W: i32 = 160;
const CTX_H: i32 = CTX_N as i32 * 22 + 4;

const CURSOR_DATA: [[i32; 2]; 16] = [
    [4,7], [3,8], [2,9], [1,10], [0,5], [0,5],
    [0,5], [0,5], [0,6], [0,7], [0,7], [1,7],
    [2,7], [3,7], [4,7], [5,7],
];

const CTX_ITEMS: [&str; CTX_N] = ["New Text File", "Run as Admin", "Refresh", "Properties"];

#[derive(Clone, Copy, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
enum WinContent {
    None = 0,
    FileManager = 1,
    Editor = 2,
}

#[derive(Clone, Copy)]
struct DesktopWin {
    win: Window,
    content: WinContent,
}

static mut G_WINS: [DesktopWin; MAX_WINS] = unsafe { core::mem::zeroed() };
static mut G_NUM_WINS: usize = 0;
static mut G_ACTIVE: i32 = -1;
static mut G_MX: i32 = 0;
static mut G_MY: i32 = 0;
static mut G_PREV_BTN: u8 = 0;
static mut G_TICKS: u64 = 0;
static mut G_DRAG_WIN: i32 = -1;
static mut G_DRAG_OX: i32 = 0;
static mut G_DRAG_OY: i32 = 0;
static mut G_RESIZE_WIN: i32 = -1;
static mut G_RESIZE_OW: i32 = 0;
static mut G_RESIZE_OH: i32 = 0;
static mut G_BTN_DOWN: i32 = 0;
static mut G_CTX_OPEN: i32 = 0;
static mut G_CTX_X: i32 = 0;
static mut G_CTX_Y: i32 = 0;
static mut G_CTX_HOVER: i32 = -1;
static mut G_FM_OPEN: i32 = 0;
static mut G_FM_WIN: i32 = -1;
static mut G_FM_PATH: [u8; 256] = [0; 256];
static mut G_FM_SCROLL: i32 = 0;
static mut G_FM_SEL: i32 = 0;
static mut G_FM_COUNT: i32 = 0;
static mut G_TICK_COUNT: i32 = 0;
static mut G_FM_PENDING_DBL: i32 = 0;
static mut G_EDIT_OPEN: i32 = 0;
static mut G_EDIT_WIN: i32 = -1;
static mut G_EDIT_FILENAME: [u8; 256] = [0; 256];
static mut G_MSG_TEXT: [u8; 64] = [0; 64];
static mut G_MSG_TICKS: i32 = 0;
static mut G_ICON_X: [i32; NUM_ICONS] = [0; NUM_ICONS];
static mut G_ICON_Y: [i32; NUM_ICONS] = [0; NUM_ICONS];
static mut G_ICON_LABEL: [&str; NUM_ICONS] = ["", "", "", ""];
static mut G_ICON_COLOR: [u32; NUM_ICONS] = [0; 4];

fn make_color(r: u8, g: u8, b: u8) -> u32 {
    render::make_color(r, g, b)
}

fn c_black() -> u32 { make_color(0, 0, 0) }
fn c_white() -> u32 { make_color(255, 255, 255) }
#[allow(dead_code)]
fn c_bg_top() -> u32 { make_color(0, 0, 0x60) }
#[allow(dead_code)]
fn c_bg_bot() -> u32 { make_color(0, 0, 0x20) }
fn c_grid() -> u32 { make_color(0x35, 0x35, 0x55) }
fn c_tb_top() -> u32 { make_color(0x2A, 0x2A, 0x3A) }
#[allow(dead_code)]
fn c_tb_bot() -> u32 { make_color(0x16, 0x16, 0x26) }
fn c_title_t() -> u32 { make_color(0, 0x50, 0x90) }
#[allow(dead_code)]
fn c_title_b() -> u32 { make_color(0, 0x20, 0x60) }
fn c_title_it() -> u32 { make_color(0x30, 0x30, 0x50) }
#[allow(dead_code)]
fn c_title_ib() -> u32 { make_color(0x18, 0x18, 0x30) }
fn c_win_bg() -> u32 { make_color(0x3C, 0x3C, 0x3C) }
fn c_win_ibg() -> u32 { make_color(0x2C, 0x2C, 0x2C) }
fn c_close_red() -> u32 { make_color(0xE0, 0x30, 0x30) }
fn c_shadow() -> u32 { make_color(0, 0, 0x10) }
fn c_icon_sh() -> u32 { make_color(0x30, 0x90, 0x30) }
fn c_icon_fi() -> u32 { make_color(0xA0, 0x70, 0x20) }
fn c_icon_tr() -> u32 { make_color(0x90, 0x20, 0x20) }
fn c_hover_glow() -> u32 { make_color(0x80, 0xB0, 0xFF) }
fn c_menu_bg() -> u32 { make_color(0xF0, 0xF0, 0xF0) }
fn c_menu_hl() -> u32 { make_color(0x40, 0x70, 0xE0) }
fn c_menu_border() -> u32 { make_color(0x80, 0x80, 0x80) }
fn c_fm_tool() -> u32 { make_color(0x50, 0x50, 0x50) }
fn c_fm_sel() -> u32 { make_color(0x30, 0x50, 0x80) }

unsafe fn show_msg(text: &str) {
    let len = text.len().min(63);
    G_MSG_TEXT[..len].copy_from_slice(text.as_bytes());
    G_MSG_TEXT[len] = 0;
    G_MSG_TICKS = 120;
}

pub unsafe fn draw_background(svc: &dyn DesktopServices) {
    let sw = svc.gop_get_width() as i32;
    let sh = svc.gop_get_height() as i32;
    let desktop_h = sh - TASKBAR_H;
    let mut y = 0;
    while y < sh {
        let mut h = 4;
        if y + h > sh { h = sh - y; }
        let r: i32;
        if y < desktop_h {
            r = 0x60 + ((0x20 - 0x60) * y / if desktop_h > 0 { desktop_h } else { 1 });
        } else {
            r = 0x20;
        }
        let r = r.max(0).min(255) as u8;
        svc.gop_fill_rect(0, y as u32, sw as u32, h as u32, make_color(0, 0, r));
        y += h;
    }
    let mut dy = 8;
    while dy < desktop_h {
        let mut dx = 8;
        while dx < sw {
            svc.gop_fill_rect(dx as u32, dy as u32, 1, 1, c_grid());
            dx += 24;
        }
        dy += 24;
    }
}

unsafe fn rounded_rect(svc: &dyn DesktopServices, x: i32, y: i32, w: i32, h: i32, r: i32, color: u32) {
    svc.gop_fill_rect((x + r) as u32, y as u32, (w - r * 2) as u32, h as u32, color);
    svc.gop_fill_rect(x as u32, (y + r) as u32, w as u32, (h - r * 2) as u32, color);
    for i in 0..r {
        let len = r - i;
        svc.gop_fill_rect((x + i) as u32, (y + len) as u32, 1, 1, color);
        svc.gop_fill_rect((x + w - 1 - i) as u32, (y + len) as u32, 1, 1, color);
        svc.gop_fill_rect((x + i) as u32, (y + h - 1 - len) as u32, 1, 1, color);
        svc.gop_fill_rect((x + w - 1 - i) as u32, (y + h - 1 - len) as u32, 1, 1, color);
    }
}

unsafe fn draw_cursor(svc: &dyn DesktopServices, mx: i32, my: i32) {
    let ox = mx + 1;
    let oy = my + 1;
    for row in 0..CUR_H {
        let x0 = CURSOR_DATA[row as usize][0];
        let x1 = CURSOR_DATA[row as usize][1];
        if x0 <= x1 {
            svc.gop_fill_rect((ox + x0 - 1) as u32, (oy + row) as u32, (x1 - x0 + 3) as u32, 1, c_white());
            svc.gop_fill_rect((ox + x0) as u32, (oy + row) as u32, (x1 - x0 + 1) as u32, 1, c_black());
        }
    }
}

unsafe fn icon_hit(mx: i32, my: i32, idx: usize) -> bool {
    let ix = G_ICON_X[idx];
    let iy = G_ICON_Y[idx];
    mx >= ix && mx < ix + ICON_W && my >= iy && my < iy + ICON_H + 18
}

unsafe fn draw_one_icon(svc: &dyn DesktopServices, idx: usize) {
    let x = G_ICON_X[idx];
    let y = G_ICON_Y[idx];
    let r = 6;
    let glow = if G_MX >= x && G_MX < x + ICON_W && G_MY >= y && G_MY < y + ICON_H && G_ACTIVE < 0 { 1 } else { 0 };
    svc.gop_fill_rect((x + 3) as u32, (y + 3) as u32, ICON_W as u32, ICON_H as u32, c_black());
    for i in 0..ICON_H {
        let light = 30 - (i * 25 / ICON_H);
        let c = if glow != 0 {
            make_color(
                (0x60 + light).min(255) as u8,
                (0x90 + light).min(255) as u8,
                (0xE0 + light).min(255) as u8,
            )
        } else {
            let (cr, cg, cb) = if idx == 1 {
                ((0xA0 + light).min(255), (0x70 + light).min(255), (0x20 + light).min(255))
            } else if idx == 2 {
                ((0x40 + light).min(255), (0x40 + light).min(255), (0x90 + light).min(255))
            } else if idx == 3 {
                ((0x90 + light).min(255), (0x20 + light).min(255), (0x20 + light).min(255))
            } else {
                ((0x30 + light).min(255), (0x90 + light).min(255), (0x30 + light).min(255))
            };
            make_color(cr as u8, cg as u8, cb as u8)
        };
        svc.gop_fill_rect((x + r) as u32, (y + i) as u32, (ICON_W - r * 2) as u32, 1, c);
    }
    for cy in 0..r {
        let len = r - cy;
        let light = 30 - (cy * 25 / r);
        let c = if glow != 0 {
            make_color(
                (0x60 + light).min(255) as u8,
                (0x90 + light).min(255) as u8,
                (0xE0 + light).min(255) as u8,
            )
        } else {
            G_ICON_COLOR[idx]
        };
        svc.gop_fill_rect((x + cy) as u32, (y + len) as u32, 1, (ICON_H - len * 2) as u32, c);
        svc.gop_fill_rect((x + ICON_W - 1 - cy) as u32, (y + len) as u32, 1, (ICON_H - len * 2) as u32, c);
    }
    rounded_rect(svc, x, y, ICON_W, ICON_H, r, c_white());
    if glow != 0 {
        for i in 1..=2 {
            rounded_rect(svc, x - i, y - i, ICON_W + i * 2, ICON_H + i * 2, r + 1, c_hover_glow());
        }
    }
    let label = G_ICON_LABEL[idx];
    let lx = x + (ICON_W / 2) - (label.len() as i32 * 8 / 2);
    draw_string_fb(svc, lx as u32, (y + ICON_H + 4) as u32, c_white(), make_color(0, 0, 0x60), label);
}

unsafe fn setup_icons(svc: &dyn DesktopServices) {
    let sw = svc.gop_get_width() as i32;
    let sh = svc.gop_get_height() as i32;
    G_ICON_X[0] = 50;
    G_ICON_Y[0] = ICONS_TOP;
    G_ICON_X[1] = 50 + ICON_GAP;
    G_ICON_Y[1] = ICONS_TOP;
    G_ICON_X[2] = 50 + ICON_GAP * 2;
    G_ICON_Y[2] = ICONS_TOP;
    G_ICON_X[3] = sw - ICON_W - 24;
    G_ICON_Y[3] = sh - TASKBAR_H - ICON_H - 40;
    G_ICON_LABEL[0] = "Shell";
    G_ICON_LABEL[1] = "Files";
    G_ICON_LABEL[2] = "Notepad";
    G_ICON_LABEL[3] = "Trash";
    G_ICON_COLOR[0] = c_icon_sh();
    G_ICON_COLOR[1] = c_icon_fi();
    G_ICON_COLOR[2] = make_color(0x50, 0x50, 0x90);
    G_ICON_COLOR[3] = c_icon_tr();
}

unsafe fn draw_icons(svc: &dyn DesktopServices) {
    for i in 0..NUM_ICONS {
        draw_one_icon(svc, i);
    }
}

pub unsafe fn draw_taskbar(svc: &dyn DesktopServices, sw: u32, sh: u32, _taskbar_h: u32) {
    let sw = sw as i32;
    let sh = sh as i32;
    let ty = sh - TASKBAR_H;
    for i in 0..TASKBAR_H {
        let step = 0x2A - (i * 0x14 / TASKBAR_H);
        let g = step;
        let b = 0x3A - (i * 0x14 / TASKBAR_H);
        svc.gop_fill_rect(0, (ty + i) as u32, sw as u32, 1, make_color(step as u8, g as u8, b as u8));
    }
    svc.gop_fill_rect(0, ty as u32, sw as u32, 1, make_color(0x40, 0x40, 0x50));
    let sb_w = 80;
    let sb_h = TASKBAR_H - 6;
    let sb_x = 4;
    let sb_y = ty + 3;
    let sb_col = make_color(0x20, 0x70, 0x20);
    rounded_rect(svc, sb_x, sb_y, sb_w, sb_h, 4, sb_col);
    draw_string_fb(svc, (sb_x + 8) as u32, (sb_y + (sb_h - 8) / 2) as u32, c_white(), sb_col, "LumieOS");
    if G_ACTIVE >= 0 && (G_ACTIVE as usize) < MAX_WINS && G_WINS[G_ACTIVE as usize].win.open {
        let title = G_WINS[G_ACTIVE as usize].win.title_str();
        let tw = title.len() as i32 * 8;
        let mut tx = (sw - tw) / 2;
        if tx < sb_x + sb_w + 10 { tx = sb_x + sb_w + 10; }
        draw_string_fb(svc, tx as u32, (ty + (TASKBAR_H - 8) / 2) as u32, make_color(0xCC, 0xCC, 0xCC), c_tb_top(), title);
    }
    svc.gop_fill_rect((sw - 66) as u32, (ty + 6) as u32, 1, (TASKBAR_H - 12) as u32, make_color(0x50, 0x50, 0x60));
}

unsafe fn draw_window_shadow(svc: &dyn DesktopServices, x: i32, y: i32, w: i32, h: i32) {
    svc.gop_fill_rect((x + 4) as u32, (y + 4) as u32, w as u32, h as u32, c_shadow());
}

unsafe fn draw_window_title(svc: &dyn DesktopServices, x: i32, y: i32, w: i32, active: bool) {
    for i in 0..TITLE_H {
        let c = if active {
            let g = 0x50 - (i * 0x30 / TITLE_H);
            let b = 0x90 - (i * 0x30 / TITLE_H);
            make_color(0, g as u8, b as u8)
        } else {
            let r = 0x30 - (i * 0x18 / TITLE_H);
            let g = 0x30 - (i * 0x18 / TITLE_H);
            let b = 0x50 - (i * 0x20 / TITLE_H);
            make_color(r as u8, g as u8, b as u8)
        };
        svc.gop_fill_rect(x as u32, (y + i) as u32, w as u32, 1, c);
    }
}

unsafe fn draw_window(svc: &dyn DesktopServices, idx: usize) {
    let win = &G_WINS[idx];
    if !win.win.open { return; }
    let x = win.win.x;
    let y = win.win.y;
    let w = win.win.w;
    let h = win.win.h;
    let active = idx as i32 == G_ACTIVE;
    draw_window_shadow(svc, x, y, w, h);
    svc.gop_fill_rect(x as u32, y as u32, w as u32, h as u32, if active { c_win_bg() } else { c_win_ibg() });
    let ty = y - TITLE_H;
    draw_window_title(svc, x, ty, w, active);
    let title = win.win.title_str();
    draw_string_fb(svc, (x + 6) as u32, (ty + (TITLE_H - 8) / 2) as u32, c_white(),
        if active { c_title_t() } else { c_title_it() }, title);
    let cx = x + w - 18;
    let cy = ty + 3;
    svc.gop_fill_rect((cx + 1) as u32, cy as u32, 14, 1, c_close_red());
    svc.gop_fill_rect(cx as u32, (cy + 1) as u32, 16, 14, c_close_red());
    svc.gop_fill_rect((cx + 1) as u32, (cy + 15) as u32, 14, 1, c_close_red());
    draw_string_fb(svc, (cx + 4) as u32, (cy + 4) as u32, c_white(), c_close_red(), "X");
    if active {
        svc.gop_fill_rect((x + w - RESIZE_W) as u32, (y + h - RESIZE_H) as u32, RESIZE_W as u32, RESIZE_H as u32, make_color(0x50, 0x50, 0x50));
        for i in 0..3 {
            svc.gop_fill_rect((x + w - 6 - i * 3) as u32, (y + h - 4 + i * 2) as u32, 3, 1, c_white());
        }
    }
    svc.gop_fill_rect((x + 1) as u32, (y + 1) as u32, (w - 2) as u32, 1, make_color(0x50, 0x50, 0x50));
    svc.gop_fill_rect((x + 1) as u32, (y + h - 2) as u32, (w - 2) as u32, 1, make_color(0x20, 0x20, 0x20));
    match win.content {
        WinContent::FileManager => fm_draw(svc, x + 2, y + 2, w - 4, h - 4),
        WinContent::Editor => edit_draw(svc, x + 2, y + 2, w - 4, h - 4),
        WinContent::None => {}
    }
}

unsafe fn win_from_pt(mx: i32, my: i32, skip: i32) -> i32 {
    for i in (0..G_NUM_WINS).rev() {
        if i as i32 == skip { continue; }
        let w = &G_WINS[i];
        if !w.win.open { continue; }
        let x = w.win.x;
        let y = w.win.y;
        let xw = x + w.win.w;
        let yh = y + w.win.h;
        let ty = y - TITLE_H;
        if mx >= x && mx < xw && my >= ty && my < yh {
            return i as i32;
        }
    }
    -1
}

unsafe fn win_resize_hit(mx: i32, my: i32, idx: i32) -> bool {
    if idx < 0 || idx as usize >= G_NUM_WINS { return false; }
    let win = &G_WINS[idx as usize];
    if !win.win.open { return false; }
    let rx = win.win.x + win.win.w - RESIZE_W;
    let ry = win.win.y + win.win.h - RESIZE_H;
    mx >= rx && mx < rx + RESIZE_W + 4 && my >= ry && my < ry + RESIZE_H + 4
}

fn open_window(svc: &dyn DesktopServices, title: &str, content: WinContent, w: i32, h: i32) -> i32 {
    unsafe {
        if G_NUM_WINS >= MAX_WINS { return -1; }
        let idx = G_NUM_WINS;
        let dw = &mut G_WINS[idx];
        let sw = svc.gop_get_width() as i32;
        let sh = svc.gop_get_height() as i32;
        dw.win = Window::new(title, (sw - w) / 2, (sh - h - TASKBAR_H) / 2, w, h);
        if dw.win.y < 0 { dw.win.y = 0; }
        dw.win.open = true;
        dw.win.active = true;
        dw.content = content;
        if G_ACTIVE >= 0 && (G_ACTIVE as usize) < MAX_WINS {
            G_WINS[G_ACTIVE as usize].win.active = false;
        }
        G_ACTIVE = idx as i32;
        G_NUM_WINS += 1;
        idx as i32
    }
}

unsafe fn close_window(idx: i32) {
    if idx < 0 || idx as usize >= G_NUM_WINS { return; }
    let dw = &mut G_WINS[idx as usize];
    if idx == G_FM_WIN { G_FM_OPEN = 0; G_FM_WIN = -1; }
    if idx == G_EDIT_WIN { G_EDIT_OPEN = 0; G_EDIT_WIN = -1; }
    dw.win.open = false;
    if G_ACTIVE == idx {
        G_ACTIVE = -1;
        for i in (0..G_NUM_WINS).rev() {
            if G_WINS[i].win.open {
                G_ACTIVE = i as i32;
                G_WINS[i].win.active = true;
                break;
            }
        }
    }
}

unsafe fn draw_context_menu(svc: &dyn DesktopServices) {
    if G_CTX_OPEN == 0 { return; }
    let x = G_CTX_X;
    let y = G_CTX_Y;
    svc.gop_fill_rect((x + 2) as u32, (y + 2) as u32, CTX_W as u32, CTX_H as u32, c_black());
    svc.gop_fill_rect(x as u32, y as u32, CTX_W as u32, CTX_H as u32, c_menu_bg());
    svc.gop_fill_rect(x as u32, y as u32, CTX_W as u32, 1, c_menu_border());
    svc.gop_fill_rect(x as u32, (y + CTX_H - 1) as u32, CTX_W as u32, 1, c_menu_border());
    svc.gop_fill_rect(x as u32, y as u32, 1, CTX_H as u32, c_menu_border());
    svc.gop_fill_rect((x + CTX_W - 1) as u32, y as u32, 1, CTX_H as u32, c_menu_border());
    for i in 0..CTX_N {
        let iy = y + 2 + i as i32 * 22;
        if i as i32 == G_CTX_HOVER {
            svc.gop_fill_rect((x + 2) as u32, iy as u32, (CTX_W - 4) as u32, 20, c_menu_hl());
            draw_string_fb(svc, (x + 8) as u32, (iy + 6) as u32, c_white(), c_menu_hl(), CTX_ITEMS[i]);
        } else {
            draw_string_fb(svc, (x + 8) as u32, (iy + 6) as u32, c_black(), c_menu_bg(), CTX_ITEMS[i]);
        }
    }
}

unsafe fn ctx_hit_test(mx: i32, my: i32) -> i32 {
    if G_CTX_OPEN == 0 { return -1; }
    if mx < G_CTX_X || mx >= G_CTX_X + CTX_W || my < G_CTX_Y || my >= G_CTX_Y + CTX_H {
        return -1;
    }
    (my - G_CTX_Y - 2) / 22
}

unsafe fn ctx_handle(svc: &dyn DesktopServices, item: i32) {
    G_CTX_OPEN = 0;
    match item {
        0 => {
            svc.fs_write("/newfile.txt", &[0]);
            edit_open_win(svc, "/newfile.txt");
        }
        1 => {
            svc.shell_run();
        }
        2 => {
            setup_icons(svc);
        }
        3 => {
            show_msg("LumieOS v0.1 | x86_64 | 128MB RAM");
        }
        _ => {}
    }
}

unsafe fn fm_refresh() {
    G_FM_SCROLL = 0;
    G_FM_COUNT = 0;
    G_FM_SEL = 0;
}

#[allow(dead_code)]
unsafe fn fm_nav_up() {
    let path = core::str::from_utf8(&G_FM_PATH).unwrap_or("/");
    let trimmed = path.trim_end_matches('\0');
    let last = if trimmed == "/" { 1usize } else { trimmed.len() - 1 };
    if last <= 1 {
        G_FM_PATH[..2].copy_from_slice(b"/\0");
    } else {
        let mut pos = last;
        if G_FM_PATH[pos] == b'/' { pos = pos.wrapping_sub(1); }
        while pos > 0 && G_FM_PATH[pos] != b'/' { pos = pos.wrapping_sub(1); }
        G_FM_PATH[pos] = b'/';
        G_FM_PATH[pos + 1] = 0;
    }
    fm_refresh();
}

#[allow(dead_code)]
unsafe fn fm_enter_dir(name: &str) {
    if name == ".." {
        fm_nav_up();
        return;
    }
    let cur_len = {
        let mut n = 0usize;
        while n < 256 && G_FM_PATH[n] != 0 { n += 1; }
        n
    };
    if name.len() == 0 { return; }
    let needs_sep = cur_len > 0 && G_FM_PATH[cur_len - 1] != b'/';
    let extra = if needs_sep { 1 + name.len() } else { name.len() };
    if cur_len + extra >= 255 { return; }
    let mut pos = cur_len;
    if needs_sep {
        G_FM_PATH[pos] = b'/';
        pos += 1;
    }
    G_FM_PATH[pos..pos + name.len()].copy_from_slice(name.as_bytes());
    pos += name.len();
    G_FM_PATH[pos] = 0;
    fm_refresh();
}

#[allow(dead_code)]
unsafe fn fm_open_file(svc: &dyn DesktopServices, name: &str) {
    let cur_len = {
        let mut n = 0usize;
        while n < 256 && G_FM_PATH[n] != 0 { n += 1; }
        n
    };
    let mut full_path = [0u8; 256];
    full_path[..cur_len].copy_from_slice(&G_FM_PATH[..cur_len]);
    let mut pos = cur_len;
    if pos > 0 && full_path[pos - 1] != b'/' {
        full_path[pos] = b'/';
        pos += 1;
    }
    let name_bytes = name.as_bytes();
    if pos + name_bytes.len() >= 255 { return; }
    full_path[pos..pos + name_bytes.len()].copy_from_slice(name_bytes);
    pos += name_bytes.len();
    full_path[pos] = 0;
    let fp = core::str::from_utf8(&full_path[..pos]).unwrap_or("/newfile.txt");
    edit_open_win(svc, fp);
}

unsafe fn fm_open_win(svc: &dyn DesktopServices) {
    if G_FM_OPEN != 0 && G_FM_WIN >= 0 && (G_FM_WIN as usize) < MAX_WINS && G_WINS[G_FM_WIN as usize].win.open {
        close_window(G_FM_WIN);
    }
    G_FM_PATH[0] = b'/';
    G_FM_PATH[1] = 0;
    G_FM_OPEN = 1;
    G_FM_SCROLL = 0;
    G_FM_SEL = 0;
    G_FM_PENDING_DBL = 0;
    fm_refresh();
    G_FM_WIN = open_window(svc, "File Manager", WinContent::FileManager, 700, 450);
}

unsafe fn fm_draw(svc: &dyn DesktopServices, x: i32, y: i32, w: i32, h: i32) {
    let mut cur_y = y;
    svc.gop_fill_rect(x as u32, cur_y as u32, w as u32, 28, c_fm_tool());
    svc.gop_fill_rect(x as u32, (cur_y + 28) as u32, w as u32, 1, make_color(0x60, 0x60, 0x60));
    let btn_col = make_color(0x3A, 0x3A, 0x3A);
    let path_col = make_color(0x20, 0x20, 0x20);
    let mut bx = x + 4;
    let by = cur_y + 4;
    svc.gop_fill_rect(bx as u32, by as u32, 36, 20, btn_col);
    draw_string_fb(svc, (bx + 4) as u32, (by + 6) as u32, c_white(), btn_col, "Back");
    bx += 40;
    svc.gop_fill_rect(bx as u32, by as u32, 48, 20, btn_col);
    draw_string_fb(svc, (bx + 4) as u32, (by + 6) as u32, c_white(), btn_col, "Forward");
    bx += 56;
    let pw = (w - (bx - x) - 8).max(20);
    svc.gop_fill_rect(bx as u32, by as u32, pw as u32, 20, path_col);
    let path = core::str::from_utf8(&G_FM_PATH).unwrap_or("/");
    draw_string_fb(svc, (bx + 4) as u32, (by + 6) as u32, make_color(0xAA, 0xAA, 0xAA), path_col, path.trim_end_matches('\0'));
    cur_y += 30;
    let avail_h = h - 30;
    let row_h = 20;
    let visible = (avail_h / row_h).max(0);
    for i in 0..visible {
        let ry = cur_y + i * row_h;
        let sel = i == G_FM_SEL;
        if sel {
            svc.gop_fill_rect(x as u32, ry as u32, w as u32, row_h as u32, c_fm_sel());
        } else if i % 2 == 0 {
            svc.gop_fill_rect(x as u32, ry as u32, w as u32, row_h as u32, make_color(0x30, 0x30, 0x30));
        }
        svc.gop_fill_rect((x + 4) as u32, (ry + 4) as u32, 4, 3, make_color(0xC0, 0xC0, 0x40));
        svc.gop_fill_rect((x + 3) as u32, (ry + 7) as u32, 6, 8, make_color(0xC0, 0xC0, 0x40));
        let bg = if sel { c_fm_sel() } else if i % 2 == 0 { make_color(0x30, 0x30, 0x30) } else { c_win_bg() };
        let fg = if sel { c_white() } else { make_color(0xDD, 0xDD, 0xDD) };
        draw_string_fb(svc, (x + 14) as u32, (ry + 6) as u32, fg, bg, "File");
    }
    if G_FM_COUNT as i32 > visible {
        let sb_x = x + w - 6;
        let sb_h_all = avail_h;
        let thumb_h = (sb_h_all * visible / G_FM_COUNT as i32).max(10);
        let mut thumb_y = cur_y;
        if G_FM_COUNT > visible {
            thumb_y += (sb_h_all - thumb_h) * G_FM_SCROLL / (G_FM_COUNT as i32 - visible);
        }
        svc.gop_fill_rect(sb_x as u32, cur_y as u32, 6, sb_h_all as u32, make_color(0x2A, 0x2A, 0x2A));
        svc.gop_fill_rect(sb_x as u32, thumb_y as u32, 6, thumb_h as u32, make_color(0x60, 0x60, 0x60));
    }
}

unsafe fn fm_key(key: i32) -> i32 {
    if G_FM_OPEN == 0 { return 0; }
    match key {
        0xE1 => {
            if G_FM_SEL > 0 { G_FM_SEL -= 1; }
            if G_FM_SEL < G_FM_SCROLL { G_FM_SCROLL = G_FM_SEL; }
            1
        }
        0xE0 => {
            if G_FM_SEL < G_FM_COUNT - 1 { G_FM_SEL += 1; }
            if G_FM_SEL >= G_FM_SCROLL + 20 { G_FM_SCROLL = G_FM_SEL - 5; }
            1
        }
        0x0A | 0x0D => 1,
        _ => 0,
    }
}

unsafe fn fm_click(_cx: i32, cy: i32) -> i32 {
    if G_FM_OPEN == 0 { return 0; }
    if cy >= 0 && cy < 30 { return 1; }
    let list_y = cy - 30;
    if list_y >= 0 {
        let row_h = 20;
        let idx = G_FM_SCROLL + list_y / row_h;
        if idx >= 0 && idx < G_FM_COUNT {
            if G_FM_SEL == idx && G_FM_PENDING_DBL != 0 {
                G_FM_PENDING_DBL = 0;
            } else {
                if G_FM_SEL == idx {
                    G_FM_PENDING_DBL = 1;
                } else {
                    G_FM_PENDING_DBL = 0;
                }
                G_FM_SEL = idx;
            }
        }
    }
    1
}

unsafe fn edit_open_win(svc: &dyn DesktopServices, filename: &str) {
    if G_EDIT_OPEN != 0 && G_EDIT_WIN >= 0 && (G_EDIT_WIN as usize) < MAX_WINS && G_WINS[G_EDIT_WIN as usize].win.open {
        close_window(G_EDIT_WIN);
    }
    let len = filename.len().min(255);
    G_EDIT_FILENAME[..len].copy_from_slice(filename.as_bytes());
    G_EDIT_FILENAME[len] = 0;
    G_EDIT_OPEN = 1;
    G_EDIT_WIN = open_window(svc, "Notepad", WinContent::Editor, 600, 400);
}

unsafe fn edit_draw(svc: &dyn DesktopServices, x: i32, y: i32, w: i32, h: i32) {
    let filename = core::str::from_utf8(&G_EDIT_FILENAME).unwrap_or("/newfile.txt");
    let display = filename.trim_end_matches('\0');
    svc.gop_fill_rect(x as u32, y as u32, w as u32, h as u32, make_color(0x1E, 0x1E, 0x1E));
    let msg = "Notepad - ";
    let mut combined = [0u8; 300];
    let msg_bytes = msg.as_bytes();
    let name_bytes = display.as_bytes();
    let total = msg_bytes.len() + name_bytes.len();
    if total < 300 {
        combined[..msg_bytes.len()].copy_from_slice(msg_bytes);
        combined[msg_bytes.len()..total].copy_from_slice(name_bytes);
    }
    let combined_str = core::str::from_utf8(&combined[..total]).unwrap_or("Notepad");
    draw_string_fb(svc, (x + 10) as u32, (y + 10) as u32, make_color(0xCC, 0xCC, 0xCC), make_color(0x1E, 0x1E, 0x1E), combined_str);
    draw_string_fb(svc, (x + 10) as u32, (y + 30) as u32, make_color(0x88, 0x88, 0x88), make_color(0x1E, 0x1E, 0x1E), "Ctrl+S: Save  Ctrl+Q: Close");
}

unsafe fn edit_key(_key: i32) -> i32 {
    1
}

unsafe fn edit_click(_cx: i32, _cy: i32) -> i32 {
    1
}

pub unsafe fn desktop_init(svc: &dyn DesktopServices) {
    G_NUM_WINS = 0;
    G_ACTIVE = -1;
    G_TICKS = 0;
    G_MX = (svc.gop_get_width() / 2) as i32;
    G_MY = (svc.gop_get_height() / 2) as i32;
    G_PREV_BTN = 0;
    G_DRAG_WIN = -1;
    G_RESIZE_WIN = -1;
    G_CTX_OPEN = 0;
    G_FM_OPEN = 0;
    G_FM_WIN = -1;
    G_MSG_TICKS = 0;
    G_BTN_DOWN = 0;
    G_FM_PENDING_DBL = 0;
    G_CTX_HOVER = -1;
    G_EDIT_OPEN = 0;
    G_EDIT_WIN = -1;
    setup_icons(svc);
}

unsafe fn draw_message(svc: &dyn DesktopServices, sw: i32, sh: i32) {
    if G_MSG_TICKS <= 0 { return; }
    let msg = core::str::from_utf8(&G_MSG_TEXT).unwrap_or("");
    let msg = msg.trim_end_matches('\0');
    let mw = msg.len() as i32 * 8 + 20;
    let mx = (sw - mw) / 2;
    let my = sh - TASKBAR_H - 50;
    svc.gop_fill_rect(mx as u32, my as u32, mw as u32, 24, make_color(0x22, 0x22, 0x44));
    svc.gop_fill_rect(mx as u32, my as u32, mw as u32, 1, make_color(0x40, 0x60, 0x80));
    draw_string_fb(svc, (mx + 10) as u32, (my + 8) as u32, c_white(), make_color(0x22, 0x22, 0x44), msg);
    G_MSG_TICKS -= 1;
}

unsafe fn desktop_redraw(svc: &dyn DesktopServices) {
    let sw = svc.gop_get_width() as i32;
    let sh = svc.gop_get_height() as i32;
    draw_background(svc);
    draw_icons(svc);
    for i in 0..G_NUM_WINS {
        draw_window(svc, i);
    }
    draw_taskbar(svc, sw as u32, sh as u32, TASKBAR_H as u32);
    draw_context_menu(svc);
    draw_message(svc, sw, sh);
    draw_cursor(svc, G_MX, G_MY);
    G_TICKS += 1;
}

pub unsafe fn desktop_run(svc: &dyn DesktopServices) {
    desktop_init(svc);
    loop {
        let sw = svc.gop_get_width() as i32;
        let sh = svc.gop_get_height() as i32;
        let mut mouse_dx: i32 = 0;
        let mut mouse_dy: i32 = 0;
        let mut mouse_btns: u8 = 0;
        if svc.mouse_poll(&mut mouse_dx, &mut mouse_dy, &mut mouse_btns) != 0 {
            G_MX += mouse_dx;
            G_MY += mouse_dy;
            if G_MX < 0 { G_MX = 0; }
            if G_MY < 0 { G_MY = 0; }
            if G_MX >= sw { G_MX = sw - 1; }
            if G_MY >= sh { G_MY = sh - 1; }
            if G_CTX_OPEN != 0 {
                G_CTX_HOVER = ctx_hit_test(G_MX, G_MY);
            }
            if G_DRAG_WIN >= 0 && (G_DRAG_WIN as usize) < G_NUM_WINS && G_WINS[G_DRAG_WIN as usize].win.open {
                G_WINS[G_DRAG_WIN as usize].win.x = G_MX - G_DRAG_OX;
                G_WINS[G_DRAG_WIN as usize].win.y = G_MY - G_DRAG_OY;
            }
            if G_RESIZE_WIN >= 0 && (G_RESIZE_WIN as usize) < G_NUM_WINS && G_WINS[G_RESIZE_WIN as usize].win.open {
                let dw = G_MX - G_DRAG_OX;
                let dh = G_MY - G_DRAG_OY;
                let mut nw = G_RESIZE_OW + dw;
                let mut nh = G_RESIZE_OH + dh;
                if nw < WIN_MIN_W { nw = WIN_MIN_W; }
                if nh < WIN_MIN_H { nh = WIN_MIN_H; }
                G_WINS[G_RESIZE_WIN as usize].win.w = nw;
                G_WINS[G_RESIZE_WIN as usize].win.h = nh;
            }
            let left_down = (mouse_btns & 0x01) != 0;
            let left_prev_down = (G_PREV_BTN & 0x01) != 0;
            let left_click = left_down && !left_prev_down;
            let right_click = (mouse_btns & 0x02) != 0 && (G_PREV_BTN & 0x02) == 0;
            if left_click {
                G_BTN_DOWN = 1;
                let mut handled = false;
                if G_CTX_OPEN != 0 {
                    let item = ctx_hit_test(G_MX, G_MY);
                    if item >= 0 {
                        ctx_handle(svc, item);
                        handled = true;
                    } else {
                        G_CTX_OPEN = 0;
                    }
                }
                for i in (0..G_NUM_WINS).rev() {
                    if handled { break; }
                    if !G_WINS[i].win.open { continue; }
                    let x = G_WINS[i].win.x;
                    let w = G_WINS[i].win.w;
                    let ty = G_WINS[i].win.y - TITLE_H;
                    let cx = x + w - 18;
                    let cy = ty + 3;
                    if G_MX >= cx && G_MX < cx + 16 && G_MY >= cy && G_MY < cy + 16 {
                        close_window(i as i32);
                        handled = true;
                    }
                }
                for i in (0..G_NUM_WINS).rev() {
                    if handled { break; }
                    if !G_WINS[i].win.open { continue; }
                    let x = G_WINS[i].win.x;
                    let y = G_WINS[i].win.y;
                    let w = G_WINS[i].win.w;
                    let ty = y - TITLE_H;
                    if G_MX >= x && G_MX < x + w && G_MY >= ty && G_MY < ty + TITLE_H {
                        if G_ACTIVE != i as i32 {
                            if G_ACTIVE >= 0 && (G_ACTIVE as usize) < MAX_WINS {
                                G_WINS[G_ACTIVE as usize].win.active = false;
                            }
                            G_ACTIVE = i as i32;
                            G_WINS[i].win.active = true;
                        }
                        if win_resize_hit(G_MX, G_MY, i as i32) {
                            G_RESIZE_WIN = i as i32;
                            G_RESIZE_OW = G_WINS[i].win.w;
                            G_RESIZE_OH = G_WINS[i].win.h;
                            G_DRAG_OX = G_MX;
                            G_DRAG_OY = G_MY;
                        } else {
                            G_DRAG_WIN = i as i32;
                            G_DRAG_OX = G_MX - G_WINS[i].win.x;
                            G_DRAG_OY = G_MY - G_WINS[i].win.y;
                        }
                        handled = true;
                    }
                }
                for i in (0..G_NUM_WINS).rev() {
                    if handled { break; }
                    if !G_WINS[i].win.open { continue; }
                    let x = G_WINS[i].win.x;
                    let y = G_WINS[i].win.y;
                    let w = G_WINS[i].win.w;
                    let h = G_WINS[i].win.h;
                    if G_MX >= x && G_MX < x + w && G_MY >= y && G_MY < y + h {
                        if G_ACTIVE != i as i32 {
                            if G_ACTIVE >= 0 && (G_ACTIVE as usize) < MAX_WINS {
                                G_WINS[G_ACTIVE as usize].win.active = false;
                            }
                            G_ACTIVE = i as i32;
                            G_WINS[i].win.active = true;
                        }
                        match G_WINS[i].content {
                            WinContent::FileManager => { fm_click(G_MX - x - 2, G_MY - y - 2); }
                            WinContent::Editor => { edit_click(G_MX - x - 2, G_MY - y - 2); }
                            WinContent::None => {}
                        }
                        handled = true;
                    }
                }
                if !handled {
                    for ic in 0..NUM_ICONS {
                        if icon_hit(G_MX, G_MY, ic) {
                            match ic {
                                0 => svc.shell_run(),
                                1 => fm_open_win(svc),
                                2 => edit_open_win(svc, "/newfile.txt"),
                                3 => show_msg("Trash is empty"),
                                _ => {}
                            }
                        }
                    }
                }
            }
            if right_click {
                let over_win = win_from_pt(G_MX, G_MY, -1);
                let mut over_icon = false;
                for ic in 0..NUM_ICONS {
                    if icon_hit(G_MX, G_MY, ic) { over_icon = true; break; }
                }
                if over_win < 0 && !over_icon {
                    G_CTX_OPEN = 1;
                    G_CTX_X = G_MX;
                    G_CTX_Y = G_MY;
                    if G_CTX_X + CTX_W > sw { G_CTX_X = sw - CTX_W - 4; }
                    if G_CTX_Y + CTX_H > sh - TASKBAR_H { G_CTX_Y = sh - TASKBAR_H - CTX_H - 4; }
                    G_CTX_HOVER = -1;
                }
            }
            if !left_down && left_prev_down {
                G_DRAG_WIN = -1;
                G_RESIZE_WIN = -1;
                G_BTN_DOWN = 0;
            }
            G_PREV_BTN = mouse_btns;
        }
        if svc.kbd_kbhit() != 0 {
            let key = svc.kbd_getchar();
            if G_ACTIVE >= 0 && (G_ACTIVE as usize) < G_NUM_WINS && G_WINS[G_ACTIVE as usize].win.open {
                match G_WINS[G_ACTIVE as usize].content {
                    WinContent::FileManager => { fm_key(key); }
                    WinContent::Editor => { edit_key(key); }
                    WinContent::None => {}
                }
            }
        }
        if G_FM_PENDING_DBL != 0 {
            G_TICK_COUNT += 1;
            if G_TICK_COUNT > 15 { G_FM_PENDING_DBL = 0; G_TICK_COUNT = 0; }
        } else {
            G_TICK_COUNT = 0;
        }
        desktop_redraw(svc);
        svc.gop_flip();
    }
}

pub fn handle_desktop_click(_svc: &dyn DesktopServices, _x: i32, _y: i32, _w: u32, _h: u32, _th: u32) {
}

unsafe fn draw_string_fb(svc: &dyn DesktopServices, x: u32, y: u32, fg: u32, bg: u32, s: &str) {
    let fb_ptr = svc.gop_get_fb() as *mut u32;
    let w = svc.gop_get_width();
    let h = svc.gop_get_height();
    let pitch_px = svc.gop_get_pitch() / 4;
    let mut cx = x;
    for &byte in s.as_bytes() {
        if cx + 8 > w || y + 16 > h { break; }
        if byte < 32 || byte > 126 {
            cx += 8;
            continue;
        }
        let idx = (byte - 32) as usize;
        for row in 0usize..16 {
            let bits = render::FONT_8X16[idx][row];
            let offset = (y as u64 + row as u64) * pitch_px as u64 + cx as u64;
            let line = fb_ptr.add(offset as usize);
            if bits == 0xFF {
                for col in 0..8 {
                    core::ptr::write_volatile(line.add(col), fg);
                }
            } else if bits == 0x00 {
                for col in 0..8 {
                    core::ptr::write_volatile(line.add(col), bg);
                }
            } else {
                for col in 0..8 {
                    let color = if bits & (0x80 >> col) != 0 { fg } else { bg };
                    core::ptr::write_volatile(line.add(col), color);
                }
            }
        }
        cx += 8;
    }
}
