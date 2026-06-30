pub const MAX_LINES: usize = 1024;
pub const MAX_LINE_LEN: usize = 256;
pub const TAB_STOP: i32 = 4;

#[repr(C)]
pub struct EditorState {
    pub lines: [[u8; MAX_LINE_LEN]; MAX_LINES],
    pub num_lines: i32,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub filename: [u8; MAX_LINE_LEN],
    pub modified: bool,
    pub done: bool,
    pub pending_quit: bool,
    pub windowed: bool,
    pub win_w: i32,
    pub win_h: i32,
    pub render_offx: i32,
    pub render_offy: i32,
}

impl EditorState {
    pub fn new() -> Self {
        unsafe { core::mem::zeroed() }
    }

    pub fn init(&mut self, filename: &str) {
        let len = filename.len().min(MAX_LINE_LEN - 1);
        let bytes = filename.as_bytes();
        let mut i = 0;
        while i < len {
            self.filename[i] = bytes[i];
            i += 1;
        }
        self.filename[i] = 0;

        self.num_lines = 1;
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.offset_x = 0;
        self.offset_y = 0;
        self.modified = false;
        self.done = false;
        self.pending_quit = false;
        self.windowed = false;
        self.win_w = 0;
        self.win_h = 0;
        self.render_offx = 0;
        self.render_offy = 0;
    }

    pub fn init_windowed(&mut self, filename: &str, win_w: i32, win_h: i32) {
        self.init(filename);
        self.windowed = true;
        self.win_w = win_w;
        self.win_h = win_h;
        self.done = false;
        self.offset_x = 0;
        self.offset_y = 0;
        self.render_offx = 0;
        self.render_offy = 0;
    }

    pub fn load_file(&mut self, services: &dyn crate::EditorServices, filename: &str) {
        let size = services.fs_get_size(filename);
        if size <= 0 || size >= 64 * 1024 {
            return;
        }

        let mut buf = [0u8; 64 * 1024];
        let read = services.fs_read(filename, &mut buf);
        if read <= 0 {
            return;
        }

        let read = read as usize;
        self.num_lines = 0;
        let mut line_start = 0usize;
        let mut i = 0;
        while i < read && (self.num_lines as usize) < MAX_LINES {
            if buf[i] == b'\n' {
                let mut len = i - line_start;
                if len > 0 && buf[line_start + len - 1] == b'\r' {
                    len -= 1;
                }
                if len > MAX_LINE_LEN - 1 {
                    len = MAX_LINE_LEN - 1;
                }
                let mut j = 0;
                while j < len {
                    self.lines[self.num_lines as usize][j] = buf[line_start + j];
                    j += 1;
                }
                self.lines[self.num_lines as usize][len] = 0;
                self.num_lines += 1;
                line_start = i + 1;
            }
            i += 1;
        }

        if line_start < read && (self.num_lines as usize) < MAX_LINES {
            let mut len = read - line_start;
            if len > 0 && buf[line_start + len - 1] == b'\r' {
                len -= 1;
            }
            if len > MAX_LINE_LEN - 1 {
                len = MAX_LINE_LEN - 1;
            }
            let mut j = 0;
            while j < len {
                self.lines[self.num_lines as usize][j] = buf[line_start + j];
                j += 1;
            }
            self.lines[self.num_lines as usize][len] = 0;
            self.num_lines += 1;
        }

        if self.num_lines == 0 {
            self.num_lines = 1;
        }
    }
}
