#![no_std]

use lumie_std::LumieColor;

pub mod editor;
pub mod render;
pub mod input;
pub mod buffer;

pub trait EditorServices {
    fn term_write(&self, s: &str);
    fn term_writeln(&self, s: &str);
    fn term_clear(&self, bg: LumieColor);
    fn term_set_fg(&self, c: LumieColor);
    fn term_set_bg(&self, c: LumieColor);
    fn term_putchar(&self, c: u8);
    fn kbd_getchar(&self) -> i32;
    fn kbd_flush(&self);
    fn gop_draw_char(&self, x: u32, y: u32, fg: u32, bg: u32, c: u8);
    fn gop_fill_rect(&self, x: u32, y: u32, w: u32, h: u32, color: u32);
    fn gop_get_width(&self) -> u32;
    fn gop_get_height(&self) -> u32;
    fn gop_make_color(&self, r: u8, g: u8, b: u8) -> u32;
    fn fs_read(&self, path: &str, buf: &mut [u8]) -> i32;
    fn fs_write(&self, path: &str, data: &[u8]) -> i32;
    fn fs_get_size(&self, path: &str) -> i32;
    fn term_get_width(&self) -> i32;
    fn term_get_height(&self) -> i32;
    fn term_set_cursor(&self, visible: bool);
}

pub fn editor_run(services: &dyn EditorServices, filename: &str) {
    let mut ed = editor::EditorState::new();
    ed.init(filename);
    ed.windowed = false;
    ed.load_file(services, filename);

    buffer::status_msg(&ed, services, "Press Ctrl+Q to quit, Ctrl+S to save, arrows to navigate");

    services.term_set_cursor(false);
    services.kbd_flush();

    while !ed.done {
        render::render(&ed, services);

        let c = services.kbd_getchar();
        input::handle_key(&mut ed, c, services);
        input::update_scroll(&mut ed, services);
    }

    services.term_set_cursor(true);
    services.term_clear(LumieColor::Blue);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
