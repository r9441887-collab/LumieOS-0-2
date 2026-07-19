use crate::ps2;
use crate::scancode;
static mut G_SHIFT: i32 = 0;
static mut G_CAPS: i32 = 0;
static mut KEY_AVAILABLE: i32 = 0;
pub static mut LAST_CHAR: i32 = 0;
static mut EXT: i32 = 0;
static mut RELEASE: i32 = 0;
pub fn process_scancode(code: u8) -> i32 {
    unsafe {
        static mut E1_SKIP: i32 = 0;
        if E1_SKIP > 0 { E1_SKIP -= 1; return 0; }
        if code == 0xE0 { EXT = 1; return 0; }
        if code == 0xE1 { EXT = 0; E1_SKIP = 7; return 0; }
        if code == 0xF0 { RELEASE = 1; return 0; }
        let key = code;
        let pressed = RELEASE == 0;
        RELEASE = 0;
        if EXT == 1 {
            EXT = 0;
            if !pressed { return 0; }
            return match key {
                0x1C => 0x0A, 0x4A => '/' as i32,
                0x70 => scancode::KBD_INS, 0x6C => scancode::KBD_HOME,
                0x7D => scancode::KBD_PGUP, 0x69 => scancode::KBD_END,
                0x7A => scancode::KBD_PGDN, 0x71 => scancode::KBD_DEL,
                0x75 => scancode::KBD_UP, 0x6B => scancode::KBD_LEFT,
                0x72 => scancode::KBD_DOWN, 0x74 => scancode::KBD_RIGHT,
                _ => 0,
            };
        }
        if key == 0x12 || key == 0x59 { G_SHIFT = pressed as i32; return 0; }
        if key == 0x14 || key == 0x11 { return 0; }
        if key == 0x58 { if pressed { G_CAPS ^= 1; } return 0; }
        if !pressed { return 0; }
        let mut c = scancode::scancode_to_ascii(key, G_SHIFT != 0);
        if c >= b'a' as i32 && c <= b'z' as i32 && G_CAPS != 0 { c -= 32; }
        if c >= b'A' as i32 && c <= b'Z' as i32 && G_CAPS != 0 && G_SHIFT != 0 { c += 32; }
        c
    }
}
pub fn poll_for_char() -> i32 {
    loop {
        unsafe {
            if KEY_AVAILABLE != 0 { KEY_AVAILABLE = 0; return LAST_CHAR; }
            if ps2::inb(ps2::PS2_STAT) & 1 != 0 {
                let c = process_scancode(ps2::inb(ps2::PS2_DATA));
                if c != 0 { return c; }
            } else { ps2::pause(); }
        }
    }
}
pub fn kbhit() -> bool {
    unsafe {
        if KEY_AVAILABLE != 0 { return true; }
        if ps2::inb(ps2::PS2_STAT) & 1 != 0 {
            let c = process_scancode(ps2::inb(ps2::PS2_DATA));
            if c != 0 { LAST_CHAR = c; KEY_AVAILABLE = 1; return true; }
        }
    }
    false
}
pub fn reset_state() {
    unsafe { KEY_AVAILABLE = 0; LAST_CHAR = 0; EXT = 0; RELEASE = 0; G_SHIFT = 0; G_CAPS = 0; }
    ps2::ps2_flush();
}
