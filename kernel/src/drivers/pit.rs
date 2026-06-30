#![no_std]

extern "C" {
    fn pit_init(freq: u32);
    fn pit_stall(us: u32);
    fn pit_get_ticks() -> u64;
}

pub unsafe fn init(freq: u32) {
    pit_init(freq);
}

pub unsafe fn stall(us: u32) {
    pit_stall(us);
}

pub fn get_ticks() -> u64 {
    unsafe { pit_get_ticks() }
}
