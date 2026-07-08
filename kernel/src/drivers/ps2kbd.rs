use core::arch::asm;

const PS2_DATA: u16 = 0x60;
const PS2_STAT: u16 = 0x64;
const PS2_CMD: u16 = 0x64;

pub const KBD_INS: i32 = 0xE8;
pub const KBD_HOME: i32 = 0xE4;
pub const KBD_PGUP: i32 = 0xE6;
pub const KBD_END: i32 = 0xE5;
pub const KBD_PGDN: i32 = 0xE7;
pub const KBD_DEL: i32 = 0x7F;
pub const KBD_UP: i32 = 0xE0;
pub const KBD_DOWN: i32 = 0xE1;
pub const KBD_LEFT: i32 = 0xE2;
pub const KBD_RIGHT: i32 = 0xE3;
#[allow(dead_code)]
pub const KBD_ESC: i32 = 0x1B;

static mut G_KBD_READY: i32 = 0;
static mut G_SHIFT: i32 = 0;
static mut G_CTRL: i32 = 0;
static mut G_ALT: i32 = 0;
static mut G_CAPS: i32 = 0;

static mut KEY_AVAILABLE: i32 = 0;
static mut LAST_CHAR: i32 = 0;
static mut EXT: i32 = 0;
static mut RELEASE: i32 = 0;

#[inline]
fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe { asm!("in al, dx", out("al") val, in("dx") port, options(nostack, preserves_flags)); }
    val
}

#[inline]
fn outb(port: u16, val: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags)); }
}

fn ps2_wait_write() {
    for _ in 0..200000 {
        if inb(PS2_STAT) & 2 == 0 {
            return;
        }
        unsafe { asm!("pause", options(nostack)); }
    }
}

fn ps2_wait_read() {
    for _ in 0..200000 {
        if inb(PS2_STAT) & 1 != 0 {
            return;
        }
        unsafe { asm!("pause", options(nostack)); }
    }
}

fn ps2_controller_exists() -> i32 {
    let status = inb(PS2_STAT);
    if status != 0xFF && status != 0 { 1 } else { 0 }
}

fn ps2_read_data() -> u8 {
    ps2_wait_read();
    inb(PS2_DATA)
}

fn ps2_write_data(val: u8) {
    ps2_wait_write();
    outb(PS2_DATA, val);
}

fn ps2_write_cmd(cmd: u8) {
    ps2_wait_write();
    outb(PS2_CMD, cmd);
}

fn ps2_cmd_with_data(cmd: u8, _data: u8) -> u8 {
    ps2_write_cmd(cmd);
    ps2_read_data()
}

pub fn ps2kbd_init() -> i32 {
    unsafe {
        if G_KBD_READY != 0 {
            return 0;
        }

        if ps2_controller_exists() == 0 {
            return -1;
        }

        for _ in 0..100 {
            if inb(PS2_STAT) & 1 == 0 {
                break;
            }
            inb(PS2_DATA);
        }

        ps2_write_cmd(0xAD);

        while inb(PS2_STAT) & 1 != 0 {
            inb(PS2_DATA);
        }

        let mut config = ps2_cmd_with_data(0x20, 0);
        config &= !0x47;
        config |= 0x01;
        ps2_write_cmd(0x60);
        ps2_write_data(config);

        ps2_write_cmd(0xAE);

        ps2_write_data(0xFF);
        let ack = ps2_read_data();
        if ack != 0xFA {
            return -1;
        }
        ps2_read_data();

        ps2_write_data(0xF4);
        let ack = ps2_read_data();
        if ack != 0xFA {
            return -1;
        }

        ps2_write_data(0xF0);
        ps2_read_data();
        ps2_write_data(0x02);
        ps2_read_data();

        G_KBD_READY = 1;
    }
    0
}

fn ps2_scancode_to_ascii(key: u8, shifted: i32) -> i32 {
    let s = shifted != 0;
    match key {
        0x16 => (if s { '!' } else { '1' }) as i32,
        0x1E => (if s { '@' } else { '2' }) as i32,
        0x26 => (if s { '#' } else { '3' }) as i32,
        0x25 => (if s { '$' } else { '4' }) as i32,
        0x2E => (if s { '%' } else { '5' }) as i32,
        0x36 => (if s { '^' } else { '6' }) as i32,
        0x3D => (if s { '&' } else { '7' }) as i32,
        0x3E => (if s { '*' } else { '8' }) as i32,
        0x46 => (if s { '(' } else { '9' }) as i32,
        0x45 => (if s { ')' } else { '0' }) as i32,
        0x4E => (if s { '_' } else { '-' }) as i32,
        0x55 => (if s { '+' } else { '=' }) as i32,
        0x15 => (if s { 'Q' } else { 'q' }) as i32,
        0x1D => (if s { 'W' } else { 'w' }) as i32,
        0x24 => (if s { 'E' } else { 'e' }) as i32,
        0x2D => (if s { 'R' } else { 'r' }) as i32,
        0x2C => (if s { 'T' } else { 't' }) as i32,
        0x35 => (if s { 'Y' } else { 'y' }) as i32,
        0x3C => (if s { 'U' } else { 'u' }) as i32,
        0x43 => (if s { 'I' } else { 'i' }) as i32,
        0x44 => (if s { 'O' } else { 'o' }) as i32,
        0x4D => (if s { 'P' } else { 'p' }) as i32,
        0x1C => (if s { 'A' } else { 'a' }) as i32,
        0x1B => (if s { 'S' } else { 's' }) as i32,
        0x23 => (if s { 'D' } else { 'd' }) as i32,
        0x2B => (if s { 'F' } else { 'f' }) as i32,
        0x34 => (if s { 'G' } else { 'g' }) as i32,
        0x33 => (if s { 'H' } else { 'h' }) as i32,
        0x3B => (if s { 'J' } else { 'j' }) as i32,
        0x42 => (if s { 'K' } else { 'k' }) as i32,
        0x4B => (if s { 'L' } else { 'l' }) as i32,
        0x1A => (if s { 'Z' } else { 'z' }) as i32,
        0x22 => (if s { 'X' } else { 'x' }) as i32,
        0x21 => (if s { 'C' } else { 'c' }) as i32,
        0x2A => (if s { 'V' } else { 'v' }) as i32,
        0x32 => (if s { 'B' } else { 'b' }) as i32,
        0x31 => (if s { 'N' } else { 'n' }) as i32,
        0x3A => (if s { 'M' } else { 'm' }) as i32,
        0x41 => (if s { '<' } else { ',' }) as i32,
        0x49 => (if s { '>' } else { '.' }) as i32,
        0x4A => (if s { '?' } else { '/' }) as i32,
        0x4C => (if s { ':' } else { ';' }) as i32,
        0x52 => (if s { '"' } else { '\'' }) as i32,
        0x54 => (if s { '{' } else { '[' }) as i32,
        0x5B => (if s { '}' } else { ']' }) as i32,
        0x5D => (if s { '|' } else { '\\' }) as i32,
        0x0E => (if s { '~' } else { '`' }) as i32,
        0x29 => ' ' as i32,
        0x66 => '\x08' as i32,
        0x5A => '\n' as i32,
        _ => 0,
    }
}

fn ps2_process_scancode(code: u8) -> i32 {
    unsafe {
        static mut E1_SKIP: i32 = 0;

        if E1_SKIP > 0 {
            E1_SKIP -= 1;
            return 0;
        }
        if code == 0xE0 {
            EXT = 1;
            return 0;
        }
        if code == 0xE1 {
            EXT = 0;
            E1_SKIP = 7;
            return 0;
        }
        if code == 0xF0 {
            RELEASE = 1;
            return 0;
        }

        let key = code;
        let pressed = RELEASE == 0;
        RELEASE = 0;

        if EXT == 1 {
            EXT = 0;
            if !pressed {
                return 0;
            }
            if key == 0x1C {
                return '\n' as i32;
            }
            if key == 0x4A {
                return '/' as i32;
            }
            if key == 0x70 {
                return KBD_INS;
            }
            if key == 0x6C {
                return KBD_HOME;
            }
            if key == 0x7D {
                return KBD_PGUP;
            }
            if key == 0x69 {
                return KBD_END;
            }
            if key == 0x7A {
                return KBD_PGDN;
            }
            if key == 0x71 {
                if G_CTRL != 0 && G_ALT != 0 {
                }
                return KBD_DEL;
            }
            if key == 0x75 {
                return KBD_UP;
            }
            if key == 0x6B {
                return KBD_LEFT;
            }
            if key == 0x72 {
                return KBD_DOWN;
            }
            if key == 0x74 {
                return KBD_RIGHT;
            }
            return 0;
        }
        if EXT == 2 {
            EXT -= 1;
            return 0;
        }

        if key == 0x12 {
            G_SHIFT = pressed as i32;
            return 0;
        }
        if key == 0x59 {
            G_SHIFT = pressed as i32;
            return 0;
        }
        if key == 0x14 {
            G_CTRL = pressed as i32;
            return 0;
        }
        if key == 0x11 {
            G_ALT = pressed as i32;
            return 0;
        }
        if key == 0x58 {
            if pressed {
                G_CAPS = if G_CAPS != 0 { 0 } else { 1 };
            }
            return 0;
        }

        if !pressed {
            return 0;
        }

        let mut c = ps2_scancode_to_ascii(key, G_SHIFT);
        if c >= 'a' as i32 && c <= 'z' as i32 && G_CAPS != 0 {
            c -= 32;
        }
        if c >= 'A' as i32 && c <= 'Z' as i32 && G_CAPS != 0 && G_SHIFT != 0 {
            c += 32;
        }
        c
    }
}

pub fn ps2kbd_getchar() -> i32 {
    loop {
        unsafe {
            if KEY_AVAILABLE != 0 {
                KEY_AVAILABLE = 0;
                return LAST_CHAR;
            }
            if inb(PS2_STAT) & 1 != 0 {
                let code = inb(PS2_DATA);
                let c = ps2_process_scancode(code);
                if c != 0 {
                    return c;
                }
            } else {
                asm!("pause", options(nostack));
            }
        }
    }
}

pub fn ps2kbd_kbhit() -> i32 {
    unsafe {
        if KEY_AVAILABLE != 0 {
            return 1;
        }
        if inb(PS2_STAT) & 1 != 0 {
            let code = inb(PS2_DATA);
            let c = ps2_process_scancode(code);
            if c != 0 {
                LAST_CHAR = c;
                KEY_AVAILABLE = 1;
                return 1;
            }
        }
    }
    0
}

pub fn ps2kbd_getch_noblock() -> i32 {
    unsafe {
        if KEY_AVAILABLE != 0 {
            KEY_AVAILABLE = 0;
            return LAST_CHAR;
        }
    }
    0
}

pub fn ps2kbd_flush() {
    unsafe {
        KEY_AVAILABLE = 0;
        LAST_CHAR = 0;
        EXT = 0;
        RELEASE = 0;
        G_SHIFT = 0;
        G_CTRL = 0;
        G_ALT = 0;
        G_CAPS = 0;
    }
    for _ in 0..100 {
        if inb(PS2_STAT) & 1 == 0 {
            break;
        }
        inb(PS2_DATA);
    }
}

pub fn ps2kbd_switch_to_ps2() -> i32 {
    ps2kbd_init()
}
