
#[repr(C)]
pub struct NvGpuApi {
    pub fill_rect: Option<unsafe fn(u32, u32, u32, u32, u32)>,
    pub put_pixel: Option<unsafe fn(u32, u32, u32)>,
    pub get_pixel: Option<unsafe fn(u32, u32) -> u32>,
    pub is_active: Option<unsafe fn() -> i32>,
    pub set_fb: Option<unsafe fn(u64, u32, u32, u32)>,
    pub init_3d: Option<unsafe fn() -> i32>,
}

pub static mut G_NV_GPU_API: Option<&'static NvGpuApi> = None;

#[repr(C)]
pub struct NvGpuState {
    pub found: i32,
    pub bus: u8,
    pub device: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub bar0_base: u64,
    pub bar1_base: u64,
    pub bar1_size: u64,
    pub fb_offset: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u32,
    pub fifo_ready: i32,
    pub channel_id: i32,
    pub push_base: u64,
    pub push_size: u32,
    pub push_pos: u32,
    pub double_buffer: i32,
    pub backbuffer_offset: u64,
    pub front_buf: i32,
}

pub static mut G_NV_STATE: NvGpuState = NvGpuState {
    found: 0,
    bus: 0,
    device: 0,
    func: 0,
    vendor_id: 0,
    device_id: 0,
    bar0_base: 0,
    bar1_base: 0,
    bar1_size: 0,
    fb_offset: 0,
    width: 0,
    height: 0,
    pitch: 0,
    bpp: 0,
    fifo_ready: 0,
    channel_id: 0,
    push_base: 0,
    push_size: 0,
    push_pos: 0,
    double_buffer: 0,
    backbuffer_offset: 0,
    front_buf: 0,
};

pub const NV_GPU_FILL_THRESHOLD: u32 = 64;

pub unsafe fn nv_gpu_init(_fb_base: u64, _width: u32, _height: u32, _pitch: u32) -> i32 {
    0
}

pub unsafe fn nv_gpu_fill_rect(_x: u32, _y: u32, _w: u32, _h: u32, _color: u32) {}
