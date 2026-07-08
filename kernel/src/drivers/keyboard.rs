use crate::globals::get_bs;
use crate::uefi::tables::EfiSystemTable;
use crate::uefi::protocols::input::{EfiSimpleTextInputProtocol, EfiInputKey};
use crate::uefi::protocols::input_ex::{EfiSimpleTextInputExProtocol, EfiInputKeyEx, EFI_SHIFT_STATE_VALID, EFI_LEFT_CONTROL_PRESSED, EFI_RIGHT_CONTROL_PRESSED};
use crate::uefi::guid::EFI_SIMPLE_TEXT_INPUT_EX_GUID;
use crate::uefi::types::*;
use crate::drivers::ps2kbd;

const KBD_MODE_UEFI: i32 = 0;
const KBD_MODE_PS2: i32 = 1;
const KBD_MODE_DEAD: i32 = 2;

static mut KBD_MODE: i32 = KBD_MODE_DEAD;
static mut CON_IN: *mut EfiSimpleTextInputProtocol = core::ptr::null_mut();
static mut CON_IN_EX: *mut EfiSimpleTextInputExProtocol = core::ptr::null_mut();
static mut G_BS_KBD: *mut crate::uefi::tables::EfiBootServices = core::ptr::null_mut();
static mut KEY_AVAILABLE: i32 = 0;
static mut LAST_CHAR: i32 = 0;

fn map_unicode_to_ascii(c: char16) -> u8 {
    match c {
        0x08 => b'\x08',
        0x09 => b'\x09',
        0x0D => b'\x0A',
        0x1B => b'\x1B',
        0x01..=0x7E => c as u8,
        _ => 0,
    }
}

fn process_key(unicode: char16, scan: u16) -> i32 {
    let c = map_unicode_to_ascii(unicode);
    if c != 0 { return c as i32; }
    if scan != 0 {
        match scan {
            0x01 => return ps2kbd::KBD_UP,
            0x02 => return ps2kbd::KBD_DOWN,
            0x03 => return ps2kbd::KBD_LEFT,
            0x04 => return ps2kbd::KBD_RIGHT,
            0x05 => return ps2kbd::KBD_ESC as i32,
            0x06 => return ps2kbd::KBD_DEL,
            0x07 => return ps2kbd::KBD_HOME,
            0x08 => return ps2kbd::KBD_END,
            0x09 => return ps2kbd::KBD_PGUP,
            0x0A => return ps2kbd::KBD_PGDN,
            0x0B => return ps2kbd::KBD_INS,
            _ => {}
        }
    }
    0
}

unsafe fn read_key(key: &mut EfiInputKey) -> i32 {
    let con_in = CON_IN;
    if con_in.is_null() { return 0; }

    if !CON_IN_EX.is_null() {
        let mut key_ex = core::mem::zeroed::<EfiInputKeyEx>();
        let read = (*CON_IN_EX).read_key_stroke_ex;
        if let Some(f) = read {
            let status = f(CON_IN_EX, &mut key_ex);
            if status == EFI_SUCCESS {
                key.unicode_char = key_ex.unicode_char;
                key.scan_code = key_ex.scan_code;
                if (key_ex.shift_state & EFI_SHIFT_STATE_VALID) != 0
                    && (key_ex.shift_state & (EFI_LEFT_CONTROL_PRESSED | EFI_RIGHT_CONTROL_PRESSED)) != 0
                {
                    if key.unicode_char >= 'a' as u16 && key.unicode_char <= 'z' as u16 {
                        key.unicode_char = key.unicode_char - 'a' as u16 + 1;
                    } else if key.unicode_char >= 'A' as u16 && key.unicode_char <= 'Z' as u16 {
                        key.unicode_char = key.unicode_char - 'A' as u16 + 1;
                    }
                }
                return 1;
            }
        }
    }

    let read = (*con_in).read_key_stroke;
    if let Some(f) = read {
        if f(con_in as *mut core::ffi::c_void, key) == EFI_SUCCESS { return 1; }
    }
    0
}

pub unsafe fn init(st: *mut EfiSystemTable) {
    if st.is_null() { return; }
    let st = &*st;
    let con_in = st.con_in;
    if con_in.is_null() { return; }
    let bs = st.boot_services;
    if bs.is_null() { return; }

    CON_IN = con_in;
    G_BS_KBD = bs;
    CON_IN_EX = core::ptr::null_mut();
    KBD_MODE = KBD_MODE_UEFI;

    let locate = (*bs).locate_protocol;
    if let Some(f) = locate {
        let mut ex_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let guid: *const crate::uefi::guid::EfiGuid = &EFI_SIMPLE_TEXT_INPUT_EX_GUID;
        let status = f(guid, core::ptr::null_mut(), &mut ex_ptr);
        if status == EFI_SUCCESS && !ex_ptr.is_null() {
            CON_IN_EX = ex_ptr as *mut EfiSimpleTextInputExProtocol;
        }
    }

    ps2kbd::ps2kbd_init();
}

pub fn switch_to_ps2() {
    unsafe {
        if ps2kbd::ps2kbd_init() == 0 {
            KBD_MODE = KBD_MODE_PS2;
            KEY_AVAILABLE = 0;
            LAST_CHAR = 0;
        } else {
            KBD_MODE = KBD_MODE_DEAD;
        }
    }
}

pub fn getchar() -> i32 {
    unsafe {
        if KBD_MODE == KBD_MODE_PS2 {
            return ps2kbd::ps2kbd_getchar();
        }
        if KBD_MODE == KBD_MODE_UEFI {
            if CON_IN.is_null() || G_BS_KBD.is_null() {
                KBD_MODE = KBD_MODE_DEAD;
                return -1;
            }
            loop {
                let mut key = core::mem::zeroed::<EfiInputKey>();
                if read_key(&mut key) != 0 {
                    let c = process_key(key.unicode_char, key.scan_code);
                    if c != 0 { return c; }
                } else {
                    let wev = (*CON_IN).wait_for_key;
                    if !wev.is_null() {
                        let wfe = (*(G_BS_KBD as *mut crate::uefi::tables::EfiBootServices)).wait_for_event;
                        if let Some(f) = wfe {
                            let mut idx: u64 = 0;
                            let mut evt: efi_event = wev;
                            f(1, &mut evt, &mut idx);
                        }
                    } else {
                        let stall = (*(G_BS_KBD as *mut crate::uefi::tables::EfiBootServices)).stall;
                        if let Some(f) = stall {
                            f(1000);
                        }
                    }
                }
            }
        }
        if ps2kbd::ps2kbd_init() == 0 {
            KBD_MODE = KBD_MODE_PS2;
            return ps2kbd::ps2kbd_getchar();
        }
        let bs = get_bs();
        if let Some(bs) = bs {
            if let Some(f) = bs.stall { f(50000); }
        }
        -1
    }
}

pub fn kbhit() -> i32 {
    unsafe {
        if KEY_AVAILABLE != 0 { return 1; }

        if KBD_MODE == KBD_MODE_PS2 {
            return ps2kbd::ps2kbd_kbhit();
        }
        if KBD_MODE == KBD_MODE_UEFI {
            if CON_IN.is_null() { return 0; }
            let mut key = core::mem::zeroed::<EfiInputKey>();
            if read_key(&mut key) != 0 {
                let c = process_key(key.unicode_char, key.scan_code);
                if c != 0 {
                    LAST_CHAR = c;
                    KEY_AVAILABLE = 1;
                    return 1;
                }
            }
            return 0;
        }
        if ps2kbd::ps2kbd_init() == 0 {
            KBD_MODE = KBD_MODE_PS2;
            return ps2kbd::ps2kbd_kbhit();
        }
        0
    }
}

pub fn getch_noblock() -> i32 {
    unsafe {
        if KEY_AVAILABLE != 0 {
            KEY_AVAILABLE = 0;
            return LAST_CHAR;
        }
        if KBD_MODE == KBD_MODE_PS2 {
            return ps2kbd::ps2kbd_getch_noblock();
        }
        if KBD_MODE != KBD_MODE_UEFI { return 0; }
        0
    }
}

pub fn flush() {
    unsafe {
        KEY_AVAILABLE = 0;
        LAST_CHAR = 0;

        if KBD_MODE == KBD_MODE_UEFI && !CON_IN.is_null() {
            let mut key = core::mem::zeroed::<EfiInputKey>();
            let read = (*CON_IN).read_key_stroke;
            if let Some(f) = read {
                while f(CON_IN as *mut core::ffi::c_void, &mut key) == EFI_SUCCESS {}
            }
            if !CON_IN_EX.is_null() {
                let read_ex = (*CON_IN_EX).read_key_stroke_ex;
                if let Some(f) = read_ex {
                    let mut key_ex = core::mem::zeroed::<EfiInputKeyEx>();
                    while f(CON_IN_EX, &mut key_ex) == EFI_SUCCESS {}
                }
            }
        }

        if KBD_MODE == KBD_MODE_PS2 {
            while ps2kbd::ps2kbd_kbhit() != 0 { ps2kbd::ps2kbd_getchar(); }
            ps2kbd::ps2kbd_flush();
        }
    }
}
