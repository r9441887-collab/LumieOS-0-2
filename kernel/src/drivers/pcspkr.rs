
use core::arch::asm;

const PIT_DATA2: u16 = 0x42;
const PIT_CMD: u16 = 0x43;
const PIT_SPKR: u8 = 0xB6;
const SPKR_PORT: u16 = 0x61;

unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    asm!("in al, dx", out("al") val, in("dx") port, options(nostack, preserves_flags));
    val
}

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
}

pub unsafe fn init() {
    outb(PIT_CMD, PIT_SPKR);
    outb(PIT_DATA2, 0x31);
    outb(PIT_DATA2, 0x04);
    let tmp = inb(SPKR_PORT);
    outb(SPKR_PORT, tmp | 3);
}
