use crate::render;

#[derive(Clone, Copy)]
pub struct Button {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub label: [u8; 32],
    pub label_len: usize,
    pub color: u32,
    pub hover_color: u32,
}

impl Button {
    pub fn new(x: i32, y: i32, w: i32, h: i32, label: &str, color: u32) -> Self {
        let mut lbl = [0u8; 32];
        let len = label.len().min(31);
        lbl[..len].copy_from_slice(&label.as_bytes()[..len]);
        Button {
            x, y, w, h,
            label: lbl,
            label_len: len,
            color,
            hover_color: render::make_color(0x40, 0x70, 0xE0),
        }
    }

    pub fn hit_test(&self, mx: i32, my: i32) -> bool {
        mx >= self.x && mx < self.x + self.w && my >= self.y && my < self.y + self.h
    }

    pub fn label_str(&self) -> &str {
        core::str::from_utf8(&self.label[..self.label_len]).unwrap_or("")
    }
}

pub struct Label {
    pub x: i32,
    pub y: i32,
    pub text: [u8; 256],
    pub text_len: usize,
    pub fg: u32,
    pub bg: u32,
}

impl Label {
    pub fn new(x: i32, y: i32, text: &str, fg: u32, bg: u32) -> Self {
        let mut t = [0u8; 256];
        let len = text.len().min(255);
        t[..len].copy_from_slice(&text.as_bytes()[..len]);
        Label {
            x, y,
            text: t,
            text_len: len,
            fg, bg,
        }
    }

    pub fn text_str(&self) -> &str {
        core::str::from_utf8(&self.text[..self.text_len]).unwrap_or("")
    }
}

#[derive(Clone, Copy)]
pub struct Window {
    pub title: [u8; 32],
    pub title_len: usize,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub open: bool,
    pub active: bool,
}

impl Window {
    pub fn new(title: &str, x: i32, y: i32, w: i32, h: i32) -> Self {
        let mut t = [0u8; 32];
        let len = title.len().min(31);
        t[..len].copy_from_slice(&title.as_bytes()[..len]);
        Window {
            title: t,
            title_len: len,
            x, y, w, h,
            open: false,
            active: false,
        }
    }

    pub fn title_str(&self) -> &str {
        core::str::from_utf8(&self.title[..self.title_len]).unwrap_or("")
    }

    pub fn title_rect(&self) -> (i32, i32, i32) {
        let ty = self.y - crate::desktop::TITLE_H;
        (self.x, ty, self.w)
    }

    pub fn close_btn_rect(&self) -> (i32, i32, i32, i32) {
        let (tx, ty, tw) = self.title_rect();
        let cx = tx + tw - 18;
        let cy = ty + 3;
        (cx, cy, 16, 16)
    }

    pub fn resize_rect(&self) -> (i32, i32, i32, i32) {
        let rx = self.x + self.w - crate::desktop::RESIZE_W;
        let ry = self.y + self.h - crate::desktop::RESIZE_H;
        (rx, ry, crate::desktop::RESIZE_W + 4, crate::desktop::RESIZE_H + 4)
    }

    pub fn hit_test_title(&self, mx: i32, my: i32) -> bool {
        let (tx, ty, tw) = self.title_rect();
        mx >= tx && mx < tx + tw && my >= ty && my < ty + crate::desktop::TITLE_H
    }

    pub fn hit_test_close(&self, mx: i32, my: i32) -> bool {
        let (cx, cy, cw, ch) = self.close_btn_rect();
        mx >= cx && mx < cx + cw && my >= cy && my < cy + ch
    }

    pub fn hit_test_body(&self, mx: i32, my: i32) -> bool {
        mx >= self.x && mx < self.x + self.w && my >= self.y && my < self.y + self.h
    }

    pub fn hit_test_resize(&self, mx: i32, my: i32) -> bool {
        let (rx, ry, rw, rh) = self.resize_rect();
        mx >= rx && mx < rx + rw && my >= ry && my < ry + rh
    }
}
