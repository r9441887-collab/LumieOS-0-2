#!/usr/bin/env python3
"""Generate separate driver crates for kbd, mouse, fs."""

import os, sys

BASE = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))

DRIVERS = {
    'kbd': [
        ('ps2.rs', '''
#![no_std]
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
'''),
        ('scancode.rs', '''
#![no_std]
pub fn scancode_to_ascii(key: u8, shifted: bool) -> i32 {
    use core::cmp::Ordering;
    match key {
        0x16 => (if shifted { '!' } else { '1' }) as i32,
        0x1E => (if shifted { '@' } else { '2' }) as i32,
        0x26 => (if shifted { '#' } else { '3' }) as i32,
        0x25 => (if shifted { '$' } else { '4' }) as i32,
        0x2E => (if shifted { '%' } else { '5' }) as i32,
        0x36 => (if shifted { '^' } else { '6' }) as i32,
        0x3D => (if shifted { '&' } else { '7' }) as i32,
        0x3E => (if shifted { '*' } else { '8' }) as i32,
        0x46 => (if shifted { '(' } else { '9' }) as i32,
        0x45 => (if shifted { ')' } else { '0' }) as i32,
        0x4E => (if shifted { '_' } else { '-' }) as i32,
        0x55 => (if shifted { '+' } else { '=' }) as i32,
        0x15 => (if shifted { 'Q' } else { 'q' }) as i32,
        0x1D => (if shifted { 'W' } else { 'w' }) as i32,
        0x24 => (if shifted { 'E' } else { 'e' }) as i32,
        0x2D => (if shifted { 'R' } else { 'r' }) as i32,
        0x2C => (if shifted { 'T' } else { 't' }) as i32,
        0x35 => (if shifted { 'Y' } else { 'y' }) as i32,
        0x3C => (if shifted { 'U' } else { 'u' }) as i32,
        0x43 => (if shifted { 'I' } else { 'i' }) as i32,
        0x44 => (if shifted { 'O' } else { 'o' }) as i32,
        0x4D => (if shifted { 'P' } else { 'p' }) as i32,
        0x1C => (if shifted { 'A' } else { 'a' }) as i32,
        0x1B => (if shifted { 'S' } else { 's' }) as i32,
        0x23 => (if shifted { 'D' } else { 'd' }) as i32,
        0x2B => (if shifted { 'F' } else { 'f' }) as i32,
        0x34 => (if shifted { 'G' } else { 'g' }) as i32,
        0x33 => (if shifted { 'H' } else { 'h' }) as i32,
        0x3B => (if shifted { 'J' } else { 'j' }) as i32,
        0x42 => (if shifted { 'K' } else { 'k' }) as i32,
        0x4B => (if shifted { 'L' } else { 'l' }) as i32,
        0x1A => (if shifted { 'Z' } else { 'z' }) as i32,
        0x22 => (if shifted { 'X' } else { 'x' }) as i32,
        0x21 => (if shifted { 'C' } else { 'c' }) as i32,
        0x2A => (if shifted { 'V' } else { 'v' }) as i32,
        0x32 => (if shifted { 'B' } else { 'b' }) as i32,
        0x31 => (if shifted { 'N' } else { 'n' }) as i32,
        0x3A => (if shifted { 'M' } else { 'm' }) as i32,
        0x41 => (if shifted { '<' } else { ',' }) as i32,
        0x49 => (if shifted { '>' } else { '.' }) as i32,
        0x4A => (if shifted { '?' } else { '/' }) as i32,
        0x4C => (if shifted { ':' } else { ';' }) as i32,
        0x52 => (if shifted { '\"' } else { '\\'' }) as i32,
        0x54 => (if shifted { '{' } else { '[' }) as i32,
        0x5B => (if shifted { '}' } else { ']' }) as i32,
        0x5D => (if shifted { '|' } else { '\\\\' }) as i32,
        0x0E => (if shifted { '~' } else { '`' }) as i32,
        0x29 => ' ' as i32,
        0x66 => 0x08,
        0x5A => 0x0A,
        _ => 0,
    }
}
pub const KBD_INS: i32 = 0xE8;
pub const KBD_HOME: i32 = 0xE4;
pub const KBD_PGUP: i32 = 0xC6;
pub const KBD_END: i32 = 0xE5;
pub const KBD_PGDN: i32 = 0xC7;
pub const KBD_DEL: i32 = 0x7F;
pub const KBD_UP: i32 = 0xE0;
pub const KBD_DOWN: i32 = 0xE1;
pub const KBD_LEFT: i32 = 0xE2;
pub const KBD_RIGHT: i32 = 0xE3;
'''),
        ('process.rs', '''
#![no_std]
use crate::ps2;
use crate::scancode;
static mut G_SHIFT: i32 = 0;
static mut G_CAPS: i32 = 0;
static mut KEY_AVAILABLE: i32 = 0;
pub static mut LAST_CHAR: i32 = 0;
static mut EXT: i32 = 0;
static mut RELEASE: i32 = 0;
pub fn process_scancode(code: u8) -> i32 {
    unsafe {
        static mut E1_SKIP: i32 = 0;
        if E1_SKIP > 0 { E1_SKIP -= 1; return 0; }
        if code == 0xE0 { EXT = 1; return 0; }
        if code == 0xE1 { EXT = 0; E1_SKIP = 7; return 0; }
        if code == 0xF0 { RELEASE = 1; return 0; }
        let key = code;
        let pressed = RELEASE == 0;
        RELEASE = 0;
        if EXT == 1 {
            EXT = 0;
            if !pressed { return 0; }
            return match key {
                0x1C => 0x0A, 0x4A => '/' as i32,
                0x70 => scancode::KBD_INS, 0x6C => scancode::KBD_HOME,
                0x7D => scancode::KBD_PGUP, 0x69 => scancode::KBD_END,
                0x7A => scancode::KBD_PGDN, 0x71 => scancode::KBD_DEL,
                0x75 => scancode::KBD_UP, 0x6B => scancode::KBD_LEFT,
                0x72 => scancode::KBD_DOWN, 0x74 => scancode::KBD_RIGHT,
                _ => 0,
            };
        }
        if key == 0x12 || key == 0x59 { G_SHIFT = pressed as i32; return 0; }
        if key == 0x14 || key == 0x11 { return 0; }
        if key == 0x58 { if pressed { G_CAPS ^= 1; } return 0; }
        if !pressed { return 0; }
        let mut c = scancode::scancode_to_ascii(key, G_SHIFT != 0);
        if c >= b'a' as i32 && c <= b'z' as i32 && G_CAPS != 0 { c -= 32; }
        if c >= b'A' as i32 && c <= b'Z' as i32 && G_CAPS != 0 && G_SHIFT != 0 { c += 32; }
        c
    }
}
pub fn poll_for_char() -> i32 {
    loop {
        unsafe {
            if KEY_AVAILABLE != 0 { KEY_AVAILABLE = 0; return LAST_CHAR; }
            if ps2::inb(ps2::PS2_STAT) & 1 != 0 {
                let c = process_scancode(ps2::inb(ps2::PS2_DATA));
                if c != 0 { return c; }
            } else { ps2::pause(); }
        }
    }
}
pub fn kbhit() -> bool {
    unsafe {
        if KEY_AVAILABLE != 0 { return true; }
        if ps2::inb(ps2::PS2_STAT) & 1 != 0 {
            let c = process_scancode(ps2::inb(ps2::PS2_DATA));
            if c != 0 { LAST_CHAR = c; KEY_AVAILABLE = 1; return true; }
        }
    }
    false
}
pub fn reset_state() {
    unsafe { KEY_AVAILABLE = 0; LAST_CHAR = 0; EXT = 0; RELEASE = 0; G_SHIFT = 0; G_CAPS = 0; }
    ps2::ps2_flush();
}
'''),
        ('init.rs', '''
#![no_std]
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
'''),
        ('lib.rs', '''
#![no_std]
mod ps2;
mod scancode;
mod process;
mod init;
use core::ffi::c_void;
static mut EXPORTS: [usize; 4] = [0; 4];
#[no_mangle]
pub unsafe extern "C" fn entry(_kapi: *const c_void, module_api: *mut *mut c_void) -> i32 {
    let ret = init::kbd_init();
    if ret != 0 { return ret; }
    EXPORTS[0] = process::poll_for_char as usize;
    EXPORTS[1] = process::kbhit as usize;
    EXPORTS[2] = init::is_ready as usize;
    EXPORTS[3] = 0;
    if !module_api.is_null() {
        *module_api = &mut EXPORTS as *mut [usize; 4] as *mut c_void;
    }
    0
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
'''),
    ],
    'mouse': [
        ('ps2.rs', '''#![no_std]
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
'''),
        ('packet.rs', '''
#![no_std]
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
'''),
        ('init.rs', '''
#![no_std]
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
'''),
        ('lib.rs', '''
#![no_std]
mod ps2;
mod packet;
mod init;
use core::ffi::c_void;
static mut EXPORTS: [usize; 4] = [0; 4];
#[no_mangle]
pub unsafe extern "C" fn entry(_kapi: *const c_void, module_api: *mut *mut c_void) -> i32 {
    let ret = init::mouse_init();
    if ret != 0 { return ret; }
    EXPORTS[0] = packet::poll as usize;
    EXPORTS[1] = packet::get_pos as usize;
    EXPORTS[2] = init::is_ready as usize;
    EXPORTS[3] = 0;
    if !module_api.is_null() {
        *module_api = &mut EXPORTS as *mut [usize; 4] as *mut c_void;
    }
    0
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
'''),
    ],
    'fs': [
        ('vfat.rs', '''
#![no_std]
use core::ffi::c_void;
type ReadFn = unsafe fn(*const u8, *mut c_void, u32) -> i32;
type WriteFn = unsafe fn(*const u8, *const c_void, u32) -> i32;
type ExistsFn = unsafe fn(*const u8) -> i32;
type MkdirFn = unsafe fn(*const u8) -> i32;
pub struct FsOps {
    pub read: ReadFn,
    pub write: WriteFn,
    pub exists: ExistsFn,
    pub mkdir: MkdirFn,
}
static mut OPS: Option<FsOps> = None;
pub fn set_ops(ops: FsOps) { unsafe { OPS = Some(ops); } }
pub fn get_ops() -> Option<&'static FsOps> { unsafe { OPS.as_ref() } }
pub fn read(path: &[u8], buf: &mut [u8]) -> i32 {
    unsafe {
        if let Some(ref ops) = OPS {
            let mut p = [0u8; 256];
            let n = path.len().min(254);
            p[..n].copy_from_slice(&path[..n]);
            p[n] = 0;
            (ops.read)(p.as_ptr(), buf.as_mut_ptr() as *mut c_void, buf.len() as u32)
        } else { -1 }
    }
}
pub fn write(path: &[u8], data: &[u8]) -> i32 {
    unsafe {
        if let Some(ref ops) = OPS {
            let mut p = [0u8; 256];
            let n = path.len().min(254);
            p[..n].copy_from_slice(&path[..n]);
            p[n] = 0;
            (ops.write)(p.as_ptr(), data.as_ptr() as *const c_void, data.len() as u32)
        } else { -1 }
    }
}
pub fn exists(path: &[u8]) -> bool {
    unsafe {
        if let Some(ref ops) = OPS {
            let mut p = [0u8; 256];
            let n = path.len().min(254);
            p[..n].copy_from_slice(&path[..n]);
            p[n] = 0;
            (ops.exists)(p.as_ptr()) == 1
        } else { false }
    }
}
'''),
        ('lib.rs', '''
#![no_std]
mod vfat;
use core::ffi::c_void;
static mut EXPORTS: [usize; 4] = [0; 4];
#[no_mangle]
pub unsafe extern "C" fn entry(kapi: *const c_void, module_api: *mut *mut c_void) -> i32 {
    if kapi.is_null() { return -1; }
    let vtable = kapi as *const usize;
    let read_fn = core::mem::transmute::<usize, unsafe fn(*const u8, *mut c_void, u32) -> i32>(*vtable.add(8));
    let write_fn = core::mem::transmute::<usize, unsafe fn(*const u8, *const c_void, u32) -> i32>(*vtable.add(9));
    let exists_fn = core::mem::transmute::<usize, unsafe fn(*const u8) -> i32>(*vtable.add(10));
    let mkdir_fn = core::mem::transmute::<usize, unsafe fn(*const u8) -> i32>(*vtable.add(11));
    vfat::set_ops(vfat::FsOps { read: read_fn, write: write_fn, exists: exists_fn, mkdir: mkdir_fn });
    EXPORTS[0] = vfat::read as usize;
    EXPORTS[1] = vfat::write as usize;
    EXPORTS[2] = vfat::exists as usize;
    EXPORTS[3] = 0;
    if !module_api.is_null() {
        *module_api = &mut EXPORTS as *mut [usize; 4] as *mut c_void;
    }
    0
}
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
'''),
    ],
}

def main():
    for name, files in DRIVERS.items():
        d = os.path.join(BASE, f'drv_{name}', 'src')
        os.makedirs(d, exist_ok=True)
        for fname, content in files:
            path = os.path.join(d, fname)
            with open(path, 'w', encoding='utf-8') as f:
                f.write(content.lstrip('\n'))
            print(f'  {path}')
        print(f'Created drv_{name} ({len(files)} files)')

if __name__ == '__main__':
    main()
