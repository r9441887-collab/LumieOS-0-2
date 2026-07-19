use core::arch::asm;
pub const PS2_DATA: u16 = 0x60;
pub const PS2_STAT: u16 = 0x64;
pub const PS2_CMD: u16 = 0x64;
#[inline] pub fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe { asm!("in al, dx", out("al") val, in("dx") port, options(nostack, preserves_flags)); }
    val
}
#[inline] pub fn outb(port: u16, val: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags)); }
}
#[inline] pub fn pause() { unsafe { asm!("pause", options(nostack)); } }
pub fn wait_write() { for _ in 0..10000 { if inb(PS2_STAT) & 2 == 0 { return; } pause(); } }
pub fn wait_read() { for _ in 0..10000 { if inb(PS2_STAT) & 1 != 0 { return; } pause(); } }
pub fn read_data() -> u8 { wait_read(); inb(PS2_DATA) }
pub fn write_data(val: u8) { wait_write(); outb(PS2_DATA, val); }
pub fn write_cmd(cmd: u8) { wait_write(); outb(PS2_CMD, cmd); }
pub fn send_cmd_to_mouse(cmd: u8) -> i32 {
    write_cmd(0xD4);
    for _ in 0..5000 { pause(); }
    write_data(cmd);
    for _ in 0..100000 {
        if inb(PS2_STAT) & 1 != 0 {
            let resp = inb(PS2_DATA);
            if resp == 0xFA { return 0; }
            if resp == 0xFE { return -2; }
            return -1;
        }
        pause();
    }
    -1
}
pub fn flush_input() { while inb(PS2_STAT) & 1 != 0 { inb(PS2_DATA); } }
