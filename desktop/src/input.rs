pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
    pub prev_buttons: u8,
}

impl MouseState {
    pub fn new() -> Self {
        MouseState {
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            buttons: 0,
            prev_buttons: 0,
        }
    }

    pub fn left_clicked(&self) -> bool {
        (self.buttons & 0x01) != 0 && (self.prev_buttons & 0x01) == 0
    }

    pub fn right_clicked(&self) -> bool {
        (self.buttons & 0x02) != 0 && (self.prev_buttons & 0x02) == 0
    }

    pub fn left_released(&self) -> bool {
        (self.prev_buttons & 0x01) != 0 && (self.buttons & 0x01) == 0
    }

    pub fn clamp(&mut self, sw: i32, sh: i32) {
        if self.x < 0 { self.x = 0; }
        if self.y < 0 { self.y = 0; }
        if self.x >= sw { self.x = sw - 1; }
        if self.y >= sh { self.y = sh - 1; }
    }
}

pub enum KeyCode {
    Char(u8),
    Up,
    Down,
    Left,
    Right,
    Esc,
    Del,
    Home,
    End,
    PgUp,
    PgDn,
    Ins,
    Tab,
    Enter,
    Backspace,
    Unknown(i32),
}

pub fn translate_key(scancode: i32) -> KeyCode {
    match scancode {
        0xE0 => KeyCode::Up,
        0xE1 => KeyCode::Down,
        0xE2 => KeyCode::Left,
        0xE3 => KeyCode::Right,
        0x1B => KeyCode::Esc,
        0x7F => KeyCode::Del,
        0xE4 => KeyCode::Home,
        0xE5 => KeyCode::End,
        0xE6 => KeyCode::PgUp,
        0xE7 => KeyCode::PgDn,
        0xE8 => KeyCode::Ins,
        0x09 => KeyCode::Tab,
        0x0A | 0x0D => KeyCode::Enter,
        0x08 => KeyCode::Backspace,
        c if c >= 0x20 && c <= 0x7E => KeyCode::Char(c as u8),
        _ => KeyCode::Unknown(scancode),
    }
}
