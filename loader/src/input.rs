
use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;
use crate::display;
use crate::ffi::*;

static mut LD_ST: *mut EfiSystemTable = ptr::null_mut();
static mut LD_CON_IN: *mut core::ffi::c_void = ptr::null_mut();
static mut LD_BS: *mut EfiBootServices = ptr::null_mut();
static mut LD_POINTER: *mut core::ffi::c_void = ptr::null_mut();
static mut LD_PREV_BUTTONS: i32 = 0;
static mut LD_CLICK_X: i32 = -1;
static mut LD_CLICK_Y: i32 = -1;

#[repr(C)]
#[derive(Copy, Clone)]
struct EfiInputKey {
    scan_code: u16,
    unicode_char: u16,
}

type EfiInputReadKey = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *mut EfiInputKey) -> u64>;
type EfiInputReset = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u8)>;

#[repr(C)]
struct EfiSimpleTextInputProtocol {
    reset: EfiInputReset,
    read_key_stroke: EfiInputReadKey,
    wait_for_key: *mut core::ffi::c_void,
}

#[repr(C)]
struct EfiSimplePointerState {
    relative_movement_x: i64,
    relative_movement_y: i64,
    relative_movement_z: i64,
    attributes: u32,
    buttons: u32,
}

type EfiPointerGetState = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *mut EfiSimplePointerState) -> u64>;

#[repr(C)]
struct EfiSimplePointerProtocol {
    reset: EfiInputReset,
    get_state: EfiPointerGetState,
    wait_for_input: *mut core::ffi::c_void,
    mode: *mut core::ffi::c_void,
}

pub fn loader_kbd_init(st: *mut EfiSystemTable) {
    if st.is_null() { return; }
    unsafe {
        LD_ST = st;
        let st_ref = &*st;
        LD_CON_IN = st_ref.con_in as *mut c_void;
        LD_BS = st_ref.boot_services;
    }
}

static mut G_SAVED_KEY: EfiInputKey = EfiInputKey { scan_code: 0, unicode_char: 0 };
static mut G_HAS_SAVED_KEY: bool = false;

pub fn loader_kbhit() -> bool {
    unsafe {
        if LD_CON_IN.is_null() { return false; }
        if G_HAS_SAVED_KEY { return true; }
        let con_in = &*(LD_CON_IN as *mut EfiSimpleTextInputProtocol);
        if let Some(rks) = con_in.read_key_stroke {
            let mut key: EfiInputKey = core::mem::zeroed();
            if rks(LD_CON_IN, &mut key) == 0 {
                G_SAVED_KEY = key;
                G_HAS_SAVED_KEY = true;
                return true;
            }
        }
        false
    }
}

pub fn loader_getchar() -> i32 {
    unsafe {
        if LD_CON_IN.is_null() || LD_BS.is_null() { return 0; }
        let con_in = &*(LD_CON_IN as *mut EfiSimpleTextInputProtocol);
        loop {
            let mut key: EfiInputKey = core::mem::zeroed();
            if G_HAS_SAVED_KEY {
                key = G_SAVED_KEY;
                G_HAS_SAVED_KEY = false;
            } else {
                let st = con_in.read_key_stroke.map(|rks| rks(LD_CON_IN, &mut key));
                match st {
                    Some(0) => {}
                    _ => {
                        let bs = &*LD_BS;
                        if !con_in.wait_for_key.is_null() {
                            type WfeFn = unsafe extern "efiapi" fn(u64, *mut efi_event, *mut u64) -> u64;
                            type StallFn = unsafe extern "efiapi" fn(u64) -> u64;
                            let wfe: Option<WfeFn> = core::mem::transmute(bs.wait_for_event);
                            let stall: Option<StallFn> = core::mem::transmute(bs.stall);
                            if let Some(wf) = wfe {
                                let mut idx: u64 = 0;
                                wf(1, &con_in.wait_for_key as *const *mut c_void as *mut efi_event, &mut idx);
                            } else if let Some(s) = stall {
                                s(1000);
                            }
                        } else {
                            type StallFn = unsafe extern "efiapi" fn(u64) -> u64;
                            let stall: Option<StallFn> = core::mem::transmute(bs.stall);
                            if let Some(s) = stall {
                                s(1000);
                            }
                        }
                        continue;
                    }
                }
            }
            if key.unicode_char == 0x0D { return b'\n' as i32; }
            if key.unicode_char == 0x08 { return 0x08; }
            if key.unicode_char >= 0x01 && key.unicode_char <= 0x7E {
                return key.unicode_char as i32;
            }
            if key.scan_code >= 0x01 && key.scan_code <= 0x0B {
                return 0xE0 + key.scan_code as i32;
            }
        }
    }
}

pub fn loader_mouse_init(st: *mut EfiSystemTable) {
    if st.is_null() { return; }
    let bs = unsafe { (*st).boot_services };
    if bs.is_null() { return; }

    let pointer_guid = &EFI_SIMPLE_POINTER_PROTOCOL_GUID as *const EfiGuid;
    let abs_guid = &EFI_ABSOLUTE_POINTER_PROTOCOL_GUID as *const EfiGuid;

    unsafe {
        if let Some(lp) = (*bs).locate_protocol {
            let mut ptr: *mut c_void = ptr::null_mut();
            let st = lp(pointer_guid, ptr::null_mut(), &mut ptr);
            if st == 0 && !ptr.is_null() {
                LD_POINTER = ptr;
                if let Some(reset) = (*(ptr as *mut EfiSimplePointerProtocol)).reset {
                    reset(ptr, 0);
                }
            } else {
                let mut abs_ptr: *mut c_void = ptr::null_mut();
                let st = lp(abs_guid, ptr::null_mut(), &mut abs_ptr);
                if st == 0 && !abs_ptr.is_null() {
                    let abs_reset: EfiInputReset = core::mem::transmute(
                        (*(abs_ptr as *mut core::ffi::c_void as *mut [*mut c_void; 3]))[0]
                    );
                    if let Some(f) = abs_reset { f(abs_ptr, 0); }
                }
                LD_POINTER = ptr::null_mut();
            }
        }
    }

    unsafe {
        if !LD_POINTER.is_null() {
            let reset: EfiInputReset = core::mem::transmute(
                (*(LD_POINTER as *mut [*mut c_void; 3]))[0]
            );
            if let Some(f) = reset { f(LD_POINTER, 0); }
        } else {
            ps2mouse_init();
            if ps2mouse_is_ready() == 0 {
                xhci_init();
            }
        }
    }
}

pub fn loader_mouse_poll(dx: &mut i32, dy: &mut i32, buttons: &mut u8) -> bool {
    unsafe {
        if ps2mouse_is_ready() != 0 {
            let r = ps2mouse_poll(dx as *mut i32, dy as *mut i32, buttons as *mut u8);
            if r != 0 { return true; }
        }
        if xhci_mouse_present() != 0 {
            let r = xhci_poll_mouse(dx as *mut i32, dy as *mut i32, buttons as *mut u8);
            if r != 0 { return true; }
        }
    }

    unsafe {
        if LD_POINTER.is_null() { return false; }
        let mut ps: EfiSimplePointerState = core::mem::zeroed();
        let get_state: EfiPointerGetState = core::mem::transmute(
            (*(LD_POINTER as *mut [*mut c_void; 3]))[1]
        );
        let st = if let Some(f) = get_state { f(LD_POINTER, &mut ps) } else { 1u64 };
        if st != 0 { return false; }
        if ps.relative_movement_x == 0 && ps.relative_movement_y == 0 && ps.buttons == 0 {
            return false;
        }
        *dx = (ps.relative_movement_x >> 8) as i32;
        *dy = (ps.relative_movement_y >> 8) as i32;
        *buttons = 0;
        if ps.buttons & 0x01 != 0 { *buttons |= 1; }
        if ps.buttons & 0x02 != 0 { *buttons |= 2; }
        true
    }
}

pub fn loader_poll_mouse() {
    let mut dx: i32 = 0;
    let mut dy: i32 = 0;
    let mut btns: u8 = 0;
    if loader_mouse_poll(&mut dx, &mut dy, &mut btns) {
        unsafe {
            display::loader_cursor_restore();
            display::CURSOR_X += dx;
            display::CURSOR_Y += dy;
            if display::CURSOR_X < 0 { display::CURSOR_X = 0; }
            if display::CURSOR_Y < 0 { display::CURSOR_Y = 0; }
            let w = crate::gop_get_width() as i32;
            let h = crate::gop_get_height() as i32;
            if display::CURSOR_X >= w { display::CURSOR_X = w - 1; }
            if display::CURSOR_Y >= h { display::CURSOR_Y = h - 1; }
            display::loader_cursor_draw();

            if (btns & 1) != 0 && (LD_PREV_BUTTONS & 1) == 0 {
                LD_CLICK_X = display::CURSOR_X;
                LD_CLICK_Y = display::CURSOR_Y;
            } else {
                LD_CLICK_X = -1;
                LD_CLICK_Y = -1;
            }
            LD_PREV_BUTTONS = btns as i32;
        }
    }
}

pub fn get_ld_st() -> *mut EfiSystemTable {
    unsafe { LD_ST }
}

pub fn loader_get_click(x: &mut i32, y: &mut i32) -> bool {
    unsafe {
        if LD_CLICK_X >= 0 && LD_CLICK_Y >= 0 {
            *x = LD_CLICK_X;
            *y = LD_CLICK_Y;
            LD_CLICK_X = -1;
            LD_CLICK_Y = -1;
            true
        } else {
            false
        }
    }
}
