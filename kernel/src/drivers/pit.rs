use core::arch::asm;

const PIT_DATA: u16 = 0x40;
const PIT_CMD: u16 = 0x43;

static mut G_SAVED_DIVISOR: u32 = 0;

unsafe fn pit_outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
}

pub unsafe fn init(freq: u32) {
    let hz = if freq == 0 { 1000 } else { freq };
    let mut divisor = 1193182u32 / hz;
    if divisor < 2 { divisor = 2; }
    if divisor > 65535 { divisor = 65535; }
    G_SAVED_DIVISOR = divisor;
    pit_outb(PIT_CMD, 0x36);
    pit_outb(PIT_DATA, (divisor & 0xFF) as u8);
    pit_outb(PIT_DATA, ((divisor >> 8) & 0xFF) as u8);
}

pub unsafe fn stall(us: u32) {
    if us == 0 { return; }
    let mut ticks = (us as u64) * 1193182 / 1000000;
    if ticks > 65535 { ticks = 65535; }
    if ticks < 2 { ticks = 2; }

    pit_outb(PIT_CMD, 0x30);
    pit_outb(PIT_DATA, (ticks & 0xFF) as u8);
    pit_outb(PIT_DATA, ((ticks >> 8) & 0xFF) as u8);

    let timeout = (ticks * 2 + 10000) as u64;
    for _ in 0..timeout {
        pit_outb(PIT_CMD, 0xE2);
        let lo = {
            let mut v: u8 = 0;
            asm!("in al, dx", out("al") v, in("dx") PIT_DATA, options(nostack, preserves_flags));
            v
        };
        let hi = {
            let mut v: u8 = 0;
            asm!("in al, dx", out("al") v, in("dx") PIT_DATA, options(nostack, preserves_flags));
            v
        };
        let count = (lo as u16) | ((hi as u16) << 8);
        if count == 0 || count > ticks as u16 { break; }
        asm!("pause", options(nostack));
    }

    let restore = if G_SAVED_DIVISOR != 0 { G_SAVED_DIVISOR } else { 0 };
    if restore >= 2 {
        pit_outb(PIT_CMD, 0x34);
        pit_outb(PIT_DATA, (restore & 0xFF) as u8);
        pit_outb(PIT_DATA, ((restore >> 8) & 0xFF) as u8);
    }
}

pub fn get_ticks() -> u64 { 0 }
