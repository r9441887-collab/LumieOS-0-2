
use crate::uefi::types::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub pad1: u8,
    pub nanosecond: u32,
    pub time_zone: s16,
    pub daylight: u8,
    pub pad2: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiTimeCapabilities {
    pub resolution: u32,
    pub accuracy: u32,
    pub sets_to_zero: u8,
}
