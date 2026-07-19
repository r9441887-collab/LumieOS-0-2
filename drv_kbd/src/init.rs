use crate::ps2;
use crate::process;
static mut G_KBD_READY: i32 = 0;
pub unsafe fn kbd_init() -> i32 {
    if G_KBD_READY != 0 { return 0; }
    if !ps2::kbd_present() { return -1; }
    ps2::ps2_flush();
    ps2::write_cmd(0xAD);
    while ps2::inb(ps2::PS2_STAT) & 1 != 0 { ps2::inb(ps2::PS2_DATA); }
    let mut config = ps2::cmd_with_data(0x20);
    config &= !0x47; config |= 0x01;
    ps2::write_cmd(0x60);
    ps2::write_data(config);
    ps2::write_cmd(0xAE);
    ps2::write_data(0xFF);
    if ps2::read_data() != 0xFA { return -2; }
    ps2::read_data();
    ps2::write_data(0xF4);
    if ps2::read_data() != 0xFA { return -3; }
    ps2::write_data(0xF0);
    ps2::read_data();
    ps2::write_data(0x02);
    ps2::read_data();
    G_KBD_READY = 1;
    process::reset_state();
    0
}
pub fn is_ready() -> bool { unsafe { G_KBD_READY != 0 } }
