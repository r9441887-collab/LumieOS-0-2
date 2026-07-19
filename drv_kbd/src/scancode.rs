pub fn scancode_to_ascii(key: u8, shifted: bool) -> i32 {
    use core::cmp::Ordering;
    match key {
        0x16 => (if shifted { '!' } else { '1' }) as i32,
        0x1E => (if shifted { '@' } else { '2' }) as i32,
        0x26 => (if shifted { '#' } else { '3' }) as i32,
        0x25 => (if shifted { '$' } else { '4' }) as i32,
        0x2E => (if shifted { '%' } else { '5' }) as i32,
        0x36 => (if shifted { '^' } else { '6' }) as i32,
        0x3D => (if shifted { '&' } else { '7' }) as i32,
        0x3E => (if shifted { '*' } else { '8' }) as i32,
        0x46 => (if shifted { '(' } else { '9' }) as i32,
        0x45 => (if shifted { ')' } else { '0' }) as i32,
        0x4E => (if shifted { '_' } else { '-' }) as i32,
        0x55 => (if shifted { '+' } else { '=' }) as i32,
        0x15 => (if shifted { 'Q' } else { 'q' }) as i32,
        0x1D => (if shifted { 'W' } else { 'w' }) as i32,
        0x24 => (if shifted { 'E' } else { 'e' }) as i32,
        0x2D => (if shifted { 'R' } else { 'r' }) as i32,
        0x2C => (if shifted { 'T' } else { 't' }) as i32,
        0x35 => (if shifted { 'Y' } else { 'y' }) as i32,
        0x3C => (if shifted { 'U' } else { 'u' }) as i32,
        0x43 => (if shifted { 'I' } else { 'i' }) as i32,
        0x44 => (if shifted { 'O' } else { 'o' }) as i32,
        0x4D => (if shifted { 'P' } else { 'p' }) as i32,
        0x1C => (if shifted { 'A' } else { 'a' }) as i32,
        0x1B => (if shifted { 'S' } else { 's' }) as i32,
        0x23 => (if shifted { 'D' } else { 'd' }) as i32,
        0x2B => (if shifted { 'F' } else { 'f' }) as i32,
        0x34 => (if shifted { 'G' } else { 'g' }) as i32,
        0x33 => (if shifted { 'H' } else { 'h' }) as i32,
        0x3B => (if shifted { 'J' } else { 'j' }) as i32,
        0x42 => (if shifted { 'K' } else { 'k' }) as i32,
        0x4B => (if shifted { 'L' } else { 'l' }) as i32,
        0x1A => (if shifted { 'Z' } else { 'z' }) as i32,
        0x22 => (if shifted { 'X' } else { 'x' }) as i32,
        0x21 => (if shifted { 'C' } else { 'c' }) as i32,
        0x2A => (if shifted { 'V' } else { 'v' }) as i32,
        0x32 => (if shifted { 'B' } else { 'b' }) as i32,
        0x31 => (if shifted { 'N' } else { 'n' }) as i32,
        0x3A => (if shifted { 'M' } else { 'm' }) as i32,
        0x41 => (if shifted { '<' } else { ',' }) as i32,
        0x49 => (if shifted { '>' } else { '.' }) as i32,
        0x4A => (if shifted { '?' } else { '/' }) as i32,
        0x4C => (if shifted { ':' } else { ';' }) as i32,
        0x52 => (if shifted { '"' } else { '\'' }) as i32,
        0x54 => (if shifted { '{' } else { '[' }) as i32,
        0x5B => (if shifted { '}' } else { ']' }) as i32,
        0x5D => (if shifted { '|' } else { '\\' }) as i32,
        0x0E => (if shifted { '~' } else { '`' }) as i32,
        0x29 => ' ' as i32,
        0x66 => 0x08,
        0x5A => 0x0A,
        _ => 0,
    }
}
pub const KBD_INS: i32 = 0xE8;
pub const KBD_HOME: i32 = 0xE4;
pub const KBD_PGUP: i32 = 0xC6;
pub const KBD_END: i32 = 0xE5;
pub const KBD_PGDN: i32 = 0xC7;
pub const KBD_DEL: i32 = 0x7F;
pub const KBD_UP: i32 = 0xE0;
pub const KBD_DOWN: i32 = 0xE1;
pub const KBD_LEFT: i32 = 0xE2;
pub const KBD_RIGHT: i32 = 0xE3;
