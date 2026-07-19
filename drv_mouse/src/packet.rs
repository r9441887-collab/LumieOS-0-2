use crate::ps2;
static mut G_MOUSE_X: i32 = 0;
static mut G_MOUSE_Y: i32 = 0;
static mut G_BUTTONS: u8 = 0;
static mut PACKET_INDEX: i32 = 0;
static mut PACKET: [u8; 3] = [0; 3];
pub fn poll(dx: &mut i32, dy: &mut i32, btns: &mut u8) -> bool {
    let mut moved = false;
    unsafe {
        while ps2::inb(ps2::PS2_STAT) & 1 != 0 {
            let data = ps2::inb(ps2::PS2_DATA);
            if PACKET_INDEX == 0 { PACKET[0] = data; PACKET_INDEX = 1; }
            else {
                let idx = PACKET_INDEX as usize;
                if idx < 3 { PACKET[idx] = data; }
                PACKET_INDEX += 1;
            }
            if PACKET_INDEX >= 3 {
                PACKET_INDEX = 0;
                let x = (PACKET[1] as i8) as i32;
                let y = -((PACKET[2] as i8) as i32);
                G_BUTTONS = PACKET[0] & 0x07;
                if x != 0 || y != 0 || G_BUTTONS != 0 {
                    G_MOUSE_X += x; G_MOUSE_Y += y;
                    *dx = x; *dy = y; *btns = G_BUTTONS;
                    moved = true;
                }
            }
        }
    }
    moved
}
pub fn get_pos(x: &mut i32, y: &mut i32) { unsafe { *x = G_MOUSE_X; *y = G_MOUSE_Y; } }
pub fn set_pos(x: i32, y: i32) { unsafe { G_MOUSE_X = x; G_MOUSE_Y = y; } }
