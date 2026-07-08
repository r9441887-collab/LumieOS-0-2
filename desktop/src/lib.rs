#![no_std]

extern crate lumie_std;

pub mod desktop;
pub mod render;
pub mod input;
pub mod widgets;
pub mod tasks;

use core::ffi::c_void;

pub trait DesktopServices {
    fn term_write(&self, s: &str);
    fn term_writeln(&self, s: &str);
    fn term_clear(&self, bg: u32);
    fn term_set_fg(&self, c: u32);
    fn term_set_bg(&self, c: u32);
    fn term_set_pos(&self, x: i32, y: i32);
    fn term_get_width(&self) -> i32;
    fn term_get_height(&self) -> i32;

    fn kbd_getchar(&self) -> i32;
    fn kbd_kbhit(&self) -> i32;

    fn gop_fill_rect(&self, x: u32, y: u32, w: u32, h: u32, color: u32);
    fn gop_put_pixel(&self, x: u32, y: u32, color: u32);
    fn gop_get_width(&self) -> u32;
    fn gop_get_height(&self) -> u32;
    fn gop_get_pitch(&self) -> u32;
    fn gop_get_fb(&self) -> *mut c_void;
    fn gop_flip(&self);
    fn gop_vsync(&self);

    fn mouse_poll(&self, dx: &mut i32, dy: &mut i32, btns: &mut u8) -> i32;

    fn fs_read(&self, path: &str, buf: &mut [u8]) -> i32;
    fn fs_write(&self, path: &str, data: &[u8]) -> i32;
    fn fs_exists(&self, path: &str) -> bool;

    fn shell_run(&self);
    fn taskmgr_run(&self);
    fn setup_run(&self);
}

pub struct Desktop<'a> {
    pub svc: &'a dyn DesktopServices,
    pub width: u32,
    pub height: u32,
    pub bg_color: u32,
    pub taskbar_height: u32,
    pub running: bool,
}

impl<'a> Desktop<'a> {
    pub fn new(svc: &'a dyn DesktopServices) -> Self {
        let w = svc.gop_get_width();
        let h = svc.gop_get_height();
        Desktop {
            svc,
            width: w,
            height: h,
            bg_color: 0x000060,
            taskbar_height: 40,
            running: false,
        }
    }

    pub fn run(&mut self) {
        unsafe { desktop::desktop_run(self.svc); }
    }

    pub fn draw_background(&self) {
        unsafe { desktop::draw_background(self.svc); }
    }

    pub fn draw_taskbar(&self) {
        unsafe { desktop::draw_taskbar(self.svc, self.width, self.height, self.taskbar_height); }
    }

    pub fn handle_click(&mut self, x: i32, y: i32) {
        desktop::handle_desktop_click(self.svc, x, y, self.width, self.height, self.taskbar_height);
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
