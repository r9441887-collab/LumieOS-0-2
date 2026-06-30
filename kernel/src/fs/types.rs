#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LumieDirEnt {
    pub name: [u8; 256],
    pub is_dir: u8,
    pub size: u32,
}
