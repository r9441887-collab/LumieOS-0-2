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
pub fn wait_write() { for _ in 0..200000 { if inb(PS2_STAT) & 2 == 0 { return; } pause(); } }
pub fn wait_read() { for _ in 0..200000 { if inb(PS2_STAT) & 1 != 0 { return; } pause(); } }
pub fn kbd_present() -> bool { let s = inb(PS2_STAT); s != 0xFF && s != 0 }
pub fn ps2_flush() { for _ in 0..100 { if inb(PS2_STAT) & 1 == 0 { break; } inb(PS2_DATA); } }
pub fn read_data() -> u8 { wait_read(); inb(PS2_DATA) }
pub fn write_data(val: u8) { wait_write(); outb(PS2_DATA, val); }
pub fn write_cmd(cmd: u8) { wait_write(); outb(PS2_CMD, cmd); }
pub fn cmd_with_data(cmd: u8) -> u8 { write_cmd(cmd); wait_read(); inb(PS2_DATA) }
