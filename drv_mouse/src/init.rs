use crate::ps2;
static mut G_MOUSE_READY: i32 = 0;
pub unsafe fn mouse_init() -> i32 {
    if G_MOUSE_READY != 0 { return 0; }
    ps2::flush_input();
    ps2::write_cmd(0xA8);
    for _ in 0..50000 { ps2::pause(); }
    ps2::flush_input();
    let ret = ps2::send_cmd_to_mouse(0xFF);
    if ret == 0 {
        for _ in 0..100000 {
            if ps2::inb(ps2::PS2_STAT) & 1 != 0 && ps2::inb(ps2::PS2_DATA) == 0xAA { break; }
            ps2::pause();
        }
        for _ in 0..5000 {
            if ps2::inb(ps2::PS2_STAT) & 1 != 0 { ps2::inb(ps2::PS2_DATA); }
            else { break; }
            ps2::pause();
        }
    }
    for _ in 0..5000 { ps2::pause(); }
    if ps2::send_cmd_to_mouse(0xF4) == 0 {
        G_MOUSE_READY = 1;
        return 0;
    }
    G_MOUSE_READY = 0;
    -1
}
pub fn is_ready() -> bool { unsafe { G_MOUSE_READY != 0 } }
