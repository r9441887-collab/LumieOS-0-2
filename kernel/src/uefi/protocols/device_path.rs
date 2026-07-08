
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiDevicePathProtocol {
    pub type_: u8,
    pub sub_type: u8,
    pub length: u16,
}

pub const DEVICE_PATH_TYPE_MEDIA: u8 = 4;
pub const DEVICE_PATH_TYPE_END: u8 = 0x7F;
pub const MEDIA_FILEPATH_DP: u8 = 4;
pub const END_ENTIRE_DEVICE_PATH: u8 = 0xFF;
