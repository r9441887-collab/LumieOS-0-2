use core::arch::asm;

const PS2_DATA: u16 = 0x60;
const PS2_STAT: u16 = 0x64;
const PS2_CMD: u16 = 0x64;

static mut G_MOUSE_READY: i32 = 0;
static mut G_MOUSE_X: i32 = 0;
static mut G_MOUSE_Y: i32 = 0;
static mut G_MOUSE_BUTTONS: u8 = 0;
static mut G_PACKET_INDEX: i32 = 0;
static mut G_PACKET: [u8; 3] = [0; 3];

#[inline]
fn inb(port: u16) -> u8 {
    let val: u8;
    unsafe { asm!("in al, dx", out("al") val, in("dx") port, options(nostack, preserves_flags)); }
    val
}

#[inline]
fn outb(port: u16, val: u8) {
    unsafe { asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags)); }
}

fn ps2_wait_write() {
    for _ in 0..10000 {
        if inb(PS2_STAT) & 2 == 0 { return; }
        unsafe { asm!("pause", options(nostack)); }
    }
}

fn ps2_write_data(val: u8) {
    ps2_wait_write();
    outb(PS2_DATA, val);
}

fn ps2_write_cmd(cmd: u8) {
    ps2_wait_write();
    outb(PS2_CMD, cmd);
}

fn ps2_send_cmd_to_mouse(cmd: u8) -> i32 {
    ps2_write_cmd(0xD4);
    for _ in 0..5000 { unsafe { asm!("pause", options(nostack)); } }
    ps2_write_data(cmd);
    for _ in 0..100000 {
        if inb(PS2_STAT) & 1 != 0 {
            let resp = inb(PS2_DATA);
            if resp == 0xFA { return 0; }
            if resp == 0xFE { return -2; }
            return -1;
        }
        unsafe { asm!("pause", options(nostack)); }
    }
    -1
}

pub unsafe fn init() {
    while inb(PS2_STAT) & 1 != 0 { inb(PS2_DATA); }
    ps2_write_cmd(0xA8);
    for _ in 0..50000 { asm!("pause", options(nostack)); }
    while inb(PS2_STAT) & 1 != 0 { inb(PS2_DATA); }

    let ret = ps2_send_cmd_to_mouse(0xFF);
    if ret == 0 {
        for _ in 0..100000 {
            if inb(PS2_STAT) & 1 != 0 {
                let resp = inb(PS2_DATA);
                if resp == 0xAA { break; }
            }
            asm!("pause", options(nostack));
        }
        for _ in 0..5000 {
            if inb(PS2_STAT) & 1 != 0 { inb(PS2_DATA); }
            else { break; }
            asm!("pause", options(nostack));
        }
    }

    for _ in 0..5000 {
        if inb(PS2_STAT) & 1 != 0 { inb(PS2_DATA); }
        else { break; }
        asm!("pause", options(nostack));
    }

    for _ in 0..5000 { asm!("pause", options(nostack)); }

    let ret = ps2_send_cmd_to_mouse(0xF4);
    if ret == 0 {
        G_MOUSE_READY = 1;
        G_PACKET_INDEX = 0;
        return;
    }

    G_MOUSE_READY = 0;
    G_PACKET_INDEX = 0;
}

pub fn is_ready() -> i32 {
    unsafe { G_MOUSE_READY }
}

pub unsafe fn poll(dx: *mut i32, dy: *mut i32, btns: *mut u8) -> i32 {
    if G_MOUSE_READY == 0 { return 0; }

    let mut moved = 0;
    while inb(PS2_STAT) & 1 != 0 {
        let data = inb(PS2_DATA);
        if G_PACKET_INDEX == 0 {
            G_PACKET[0] = data;
            G_PACKET_INDEX = 1;
        } else {
            G_PACKET[G_PACKET_INDEX as usize] = data;
            G_PACKET_INDEX += 1;
        }
        if G_PACKET_INDEX >= 3 {
            G_PACKET_INDEX = 0;
            let x = (G_PACKET[1] as i8) as i32;
            let y = -((G_PACKET[2] as i8) as i32);
            G_MOUSE_BUTTONS = G_PACKET[0] & 0x07;
            if x != 0 || y != 0 || G_MOUSE_BUTTONS != 0 {
                G_MOUSE_X += x;
                G_MOUSE_Y += y;
                if !dx.is_null() { *dx = x; }
                if !dy.is_null() { *dy = y; }
                if !btns.is_null() { *btns = G_MOUSE_BUTTONS; }
                moved = 1;
            }
        }
    }
    moved
}

pub fn get_pos(x: *mut i32, y: *mut i32) {
    unsafe {
        if !x.is_null() { *x = G_MOUSE_X; }
        if !y.is_null() { *y = G_MOUSE_Y; }
    }
}

pub fn set_pos(x: i32, y: i32) {
    unsafe {
        G_MOUSE_X = x;
        G_MOUSE_Y = y;
    }
}
