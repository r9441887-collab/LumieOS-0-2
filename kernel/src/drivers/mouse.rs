use crate::uefi::tables::EfiSystemTable;
use crate::uefi::types::*;
use crate::uefi::guid::{EFI_SIMPLE_POINTER_PROTOCOL_GUID, EFI_ABSOLUTE_POINTER_PROTOCOL_GUID};
use crate::uefi::protocols::pointer::{EfiSimplePointerProtocol, EfiSimplePointerState};
use crate::console::gop::{self, get_width, get_height};
use crate::drivers::ps2mouse;

pub const MOUSE_LEFT_BUTTON: u8 = 0x01;
pub const MOUSE_RIGHT_BUTTON: u8 = 0x02;
pub const MOUSE_MIDDLE_BUTTON: u8 = 0x04;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub buttons: u8,
    pub dx: i32,
    pub dy: i32,
    pub present: i32,
}

static mut G_POINTER: *mut EfiSimplePointerProtocol = core::ptr::null_mut();
static mut G_ABS_POINTER: *mut core::ffi::c_void = core::ptr::null_mut();
static mut MOUSE_X: i32 = 0;
static mut MOUSE_Y: i32 = 0;
static mut MOUSE_PRESENT: i32 = 0;
static mut MOUSE_INIT_DONE: i32 = 0;
static mut CURSOR_BG: [[u32; 16]; 16] = [[0; 16]; 16];
static mut CURSOR_DRAWN: i32 = 0;
static mut CURSOR_LAST_X: i32 = 0;
static mut CURSOR_LAST_Y: i32 = 0;

static CURSOR_BITS: [[u8; 2]; 16] = [
    [0x80, 0x00],
    [0xC0, 0x00],
    [0xE0, 0x00],
    [0xF0, 0x00],
    [0xF8, 0x00],
    [0xFC, 0x00],
    [0xFE, 0x00],
    [0xFF, 0x00],
    [0xFF, 0x80],
    [0xFE, 0xC0],
    [0xFC, 0xE0],
    [0xF0, 0x70],
    [0xE0, 0x38],
    [0xC0, 0x1C],
    [0x80, 0x0E],
    [0x00, 0x04],
];

pub unsafe fn init(st: *mut EfiSystemTable) {
    if st.is_null() { return; }
    let st = &*st;
    let bs = st.boot_services;
    if bs.is_null() { return; }

    let locate = (*bs).locate_protocol;

    if let Some(f) = locate {
        let mut ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let guid: *const crate::uefi::guid::EfiGuid = &EFI_SIMPLE_POINTER_PROTOCOL_GUID;
        let status = f(guid, core::ptr::null_mut(), &mut ptr);
        if status == EFI_SUCCESS && !ptr.is_null() {
            G_POINTER = ptr as *mut EfiSimplePointerProtocol;
        } else {
            let mut abs: *mut core::ffi::c_void = core::ptr::null_mut();
            let aguid: *const crate::uefi::guid::EfiGuid = &EFI_ABSOLUTE_POINTER_PROTOCOL_GUID;
            let status = f(aguid, core::ptr::null_mut(), &mut abs);
            if status == EFI_SUCCESS && !abs.is_null() {
                G_ABS_POINTER = abs;
            }
        }
    }

    if !G_POINTER.is_null() {
        let reset = (*G_POINTER).reset;
        if let Some(f) = reset {
            f(G_POINTER as *mut core::ffi::c_void, FALSE);
        }
    }
    if !G_ABS_POINTER.is_null() {
        // skip ABS reset for now
    }

    let mut init_w = get_width();
    let mut init_h = get_height();
    if init_w == 0 || init_h == 0 { init_w = 640; init_h = 480; }
    MOUSE_X = (init_w / 2) as i32;
    MOUSE_Y = (init_h / 2) as i32;
    MOUSE_PRESENT = 1;
    MOUSE_INIT_DONE = 1;
}

pub unsafe fn reinit_ps2() {
    ps2mouse::init();
    if ps2mouse::is_ready() != 0 {
        ps2mouse::set_pos(MOUSE_X, MOUSE_Y);
    }
}

unsafe fn clamp_mouse() {
    let w = get_width() as i32;
    let h = get_height() as i32;
    if MOUSE_X < 0 { MOUSE_X = 0; }
    if MOUSE_Y < 0 { MOUSE_Y = 0; }
    if MOUSE_X >= w { MOUSE_X = w - 1; }
    if MOUSE_Y >= h { MOUSE_Y = h - 1; }
}

pub unsafe fn poll(state: *mut MouseState) -> i32 {
    if MOUSE_PRESENT == 0 {
        if !state.is_null() {
            (*state).x = 0;
            (*state).y = 0;
            (*state).dx = 0;
            (*state).dy = 0;
            (*state).buttons = 0;
            (*state).present = 0;
        }
        return 0;
    }

    if ps2mouse::is_ready() != 0 {
        let mut dx: i32 = 0;
        let mut dy: i32 = 0;
        let mut btns: u8 = 0;
        if ps2mouse::poll(&mut dx, &mut dy, &mut btns) != 0 {
            MOUSE_X += dx;
            MOUSE_Y += dy;
            clamp_mouse();
            if !state.is_null() {
                (*state).x = MOUSE_X;
                (*state).y = MOUSE_Y;
                (*state).dx = dx;
                (*state).dy = dy;
                (*state).buttons = btns;
                (*state).present = 1;
            }
            return 1;
        }
    } else {
        ps2mouse::init();
    }

    if MOUSE_INIT_DONE != 0 {
        MOUSE_INIT_DONE = 0;
        if !state.is_null() {
            (*state).x = MOUSE_X;
            (*state).y = MOUSE_Y;
            (*state).dx = 0;
            (*state).dy = 0;
            (*state).buttons = 0;
            (*state).present = 1;
        }
        return 1;
    }

    if let Some(_bs) = crate::globals::get_bs() {
        if !G_POINTER.is_null() {
            let mut ps = core::mem::zeroed::<EfiSimplePointerState>();
            let get_state = (*G_POINTER).get_state;
            if let Some(f) = get_state {
                let status = f(G_POINTER, &mut ps);
                if status == EFI_SUCCESS {
                    MOUSE_X += (ps.relative_movement_x >> 8) as i32;
                    MOUSE_Y += (ps.relative_movement_y >> 8) as i32;
                }
            }
            clamp_mouse();
            if !state.is_null() {
                (*state).x = MOUSE_X;
                (*state).y = MOUSE_Y;
                (*state).dx = 0;
                (*state).dy = 0;
                (*state).buttons = 0;
                (*state).present = 1;
            }
            return 1;
        }
    }

    if !state.is_null() {
        (*state).x = MOUSE_X;
        (*state).y = MOUSE_Y;
        (*state).dx = 0;
        (*state).dy = 0;
        (*state).buttons = 0;
        (*state).present = MOUSE_PRESENT;
    }
    MOUSE_PRESENT
}

fn cursor_bit(row: usize, col: usize) -> bool {
    let bits = ((CURSOR_BITS[row][0] as u16) << 8) | CURSOR_BITS[row][1] as u16;
    bits & (0x8000u16 >> col) != 0
}

pub unsafe fn draw(x: i32, y: i32) {
    if MOUSE_PRESENT == 0 { return; }
    let w = get_width();
    let h = get_height();

    for row in 0..16 {
        for col in 0..16 {
            let px = x + col;
            let py = y + row;
            if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 { continue; }

            CURSOR_BG[row as usize][col as usize] = gop::get_pixel(px as u32, py as u32);

            if cursor_bit(row as usize, col as usize) {
                let mut color = gop::gop_make_color(0x00, 0x55, 0xFF);
                if row == 0 || col == 0 { color = gop::gop_make_color(0xFF, 0xFF, 0xFF); }
                gop::put_pixel(px as u32, py as u32, color);
            }
        }
    }
    CURSOR_DRAWN = 1;
    CURSOR_LAST_X = x;
    CURSOR_LAST_Y = y;
}

pub unsafe fn restore(x: i32, y: i32) {
    if MOUSE_PRESENT == 0 || CURSOR_DRAWN == 0 { return; }
    let w = get_width();
    let h = get_height();

    for row in 0..16 {
        for col in 0..16 {
            let px = x + col;
            let py = y + row;
            if px < 0 || px >= w as i32 || py < 0 || py >= h as i32 { continue; }
            gop::put_pixel(px as u32, py as u32, CURSOR_BG[row as usize][col as usize]);
        }
    }
    CURSOR_DRAWN = 0;
}

pub fn get_pos(x: *mut i32, y: *mut i32) {
    unsafe {
        if !x.is_null() { *x = MOUSE_X; }
        if !y.is_null() { *y = MOUSE_Y; }
    }
}

pub fn set_pos(x: i32, y: i32) {
    unsafe {
        MOUSE_X = x;
        MOUSE_Y = y;
        clamp_mouse();
    }
}

pub fn cleanup_uefi() {
    unsafe {
        G_POINTER = core::ptr::null_mut();
        G_ABS_POINTER = core::ptr::null_mut();
    }
}
