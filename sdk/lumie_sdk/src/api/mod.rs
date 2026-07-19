use core::ffi::c_void;

pub const KAPI_VERSION: u32 = 2;

pub const ROLE_USER: i32 = 0;
pub const ROLE_ADMIN: i32 = 1;

pub const MOD_MAGIC_LKRN: u32 = 0x4E524B4C;
pub const MOD_MAGIC_LSH: u32 = 0x48534C4C;
pub const MOD_MAGIC_LDRV: u32 = 0x5652444C;
pub const MOD_MAGIC_SYS: u32 = 0x01535953;

pub const REL_ADDR64: u16 = 1;
pub const REL_ADDR32: u16 = 2;
pub const REL_REL32: u16 = 4;

pub type TermClearFn = Option<unsafe fn(u32)>;
pub type TermSetFgFn = Option<unsafe fn(u32)>;
pub type TermSetBgFn = Option<unsafe fn(u32)>;
pub type TermSetPosFn = Option<unsafe fn(i32, i32)>;
pub type TermWriteFn = Option<unsafe fn(*const u8)>;
pub type TermWritelnFn = Option<unsafe fn(*const u8)>;
pub type TermPutcharFn = Option<unsafe fn(u8)>;
pub type TermGetWidthFn = Option<unsafe fn() -> i32>;
pub type TermGetHeightFn = Option<unsafe fn() -> i32>;
pub type KbdGetcharFn = Option<unsafe fn() -> i32>;
pub type KbdKbhitFn = Option<unsafe fn() -> i32>;
pub type KmallocFn = Option<unsafe fn(u64) -> *mut c_void>;
pub type KfreeFn = Option<unsafe fn(*mut c_void)>;
pub type KcallocFn = Option<unsafe fn(u64, u64) -> *mut c_void>;
pub type KmemsetFn = Option<unsafe fn(*mut c_void, i32, u64)>;
pub type KmemcpyFn = Option<unsafe fn(*mut c_void, *const c_void, u64)>;
pub type FsReadFn = Option<unsafe fn(*const u8, *mut c_void, u32) -> i32>;
pub type FsWriteFn = Option<unsafe fn(*const u8, *const c_void, u32) -> i32>;
pub type FsExistsFn = Option<unsafe fn(*const u8) -> i32>;
pub type FsListFn = Option<unsafe fn(*const u8, *mut c_void, i32) -> i32>;
pub type FsMkdirFn = Option<unsafe fn(*const u8) -> i32>;
pub type PrintfFn = Option<unsafe fn(*const u8)>;
pub type StallFn = Option<unsafe fn(u64)>;
pub type RebootFn = Option<unsafe fn()>;
pub type ShutdownFn = Option<unsafe fn()>;
pub type GpuFillRectFn = Option<unsafe fn(u32, u32, u32, u32, u32) -> i32>;
pub type GpuPutPixelFn = Option<unsafe fn(u32, u32, u32) -> i32>;
pub type GpuGetPixelFn = Option<unsafe fn(u32, u32) -> u32>;
pub type GpuIsActiveFn = Option<unsafe fn() -> i32>;
pub type GpuFlipFn = Option<unsafe fn()>;
pub type GpuVsyncFn = Option<unsafe fn()>;
pub type ModLoadFn = Option<unsafe fn(*const u8, *mut *mut c_void) -> i32>;
pub type ModUnloadFn = Option<unsafe fn(i32)>;
pub type MemTotalFn = Option<unsafe fn() -> u64>;
pub type MemFreeFn = Option<unsafe fn() -> u64>;
pub type MemUsedFn = Option<unsafe fn() -> u64>;
pub type DiskReadFn = Option<unsafe fn(i32, u64, u32, *mut c_void) -> i32>;
pub type DiskWriteFn = Option<unsafe fn(i32, u64, u32, *const c_void) -> i32>;
pub type DiskCountFn = Option<unsafe fn() -> i32>;
pub type DiskNameFn = Option<unsafe fn(i32) -> *const u8>;
pub type DiskSectorsFn = Option<unsafe fn(i32) -> u64>;
pub type PciScanFn = Option<unsafe fn(i32, *mut u16, *mut u16, *mut u8) -> i32>;
pub type PciVendorStrFn = Option<unsafe fn(u16) -> *const u8>;
pub type PciDeviceStrFn = Option<unsafe fn(u16, u16) -> *const u8>;
pub type GetTimeFn = Option<unsafe fn(*mut u8, i32) -> i32>;
pub type SchedCountFn = Option<unsafe fn() -> i32>;
pub type SchedNameFn = Option<unsafe fn(i32) -> *const u8>;
pub type SchedStateFn = Option<unsafe fn(i32) -> i32>;

#[repr(C)]
pub struct KernelApiV1 {
    pub version: u32,
    pub term_clear: TermClearFn,
    pub term_set_fg: TermSetFgFn,
    pub term_set_bg: TermSetBgFn,
    pub term_set_pos: TermSetPosFn,
    pub term_write: TermWriteFn,
    pub term_writeln: TermWritelnFn,
    pub term_putchar: TermPutcharFn,
    pub term_get_width: TermGetWidthFn,
    pub term_get_height: TermGetHeightFn,
    pub kbd_getchar: KbdGetcharFn,
    pub kbd_kbhit: KbdKbhitFn,
    pub kmalloc: KmallocFn,
    pub kfree: KfreeFn,
    pub kcalloc: KcallocFn,
    pub kmemset: KmemsetFn,
    pub kmemcpy: KmemcpyFn,
    pub fs_read: FsReadFn,
    pub fs_write: FsWriteFn,
    pub fs_exists: FsExistsFn,
    pub fs_list: FsListFn,
    pub fs_mkdir: FsMkdirFn,
    pub printf: PrintfFn,
    pub stall: StallFn,
    pub reboot: RebootFn,
    pub shutdown: ShutdownFn,
    pub gpu_fill_rect: GpuFillRectFn,
    pub gpu_put_pixel: GpuPutPixelFn,
    pub gpu_get_pixel: GpuGetPixelFn,
    pub gpu_is_active: GpuIsActiveFn,
    pub gpu_flip: GpuFlipFn,
    pub gpu_vsync: GpuVsyncFn,
    pub desktop_ctx: *mut c_void,
    pub mod_load: ModLoadFn,
    pub mod_unload: ModUnloadFn,
    pub mem_total: MemTotalFn,
    pub mem_free: MemFreeFn,
    pub mem_used: MemUsedFn,
    pub disk_read: DiskReadFn,
    pub disk_write: DiskWriteFn,
    pub disk_count: DiskCountFn,
    pub disk_name: DiskNameFn,
    pub disk_sectors: DiskSectorsFn,
    pub pci_scan: PciScanFn,
    pub pci_vendor_str: PciVendorStrFn,
    pub pci_device_str: PciDeviceStrFn,
    pub get_time: GetTimeFn,
    pub sched_count: SchedCountFn,
    pub sched_name: SchedNameFn,
    pub sched_state: SchedStateFn,
    pub reserved: [u64; 8],
}

#[repr(C)]
pub struct LumieDirEnt {
    pub name: [u8; 256],
    pub is_dir: u8,
    pub size: u32,
}

#[derive(Clone, Copy, Default)]
pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
}

#[derive(Clone, Copy)]
pub struct FramebufferInfo {
    pub base: u64,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub size: u32,
}

#[derive(Clone, Copy)]
pub struct DiskInfo {
    pub name: [u8; 64],
    pub sector_count: u64,
    pub sector_size: u32,
    pub present: bool,
    pub is_ahci: bool,
}

impl Default for DiskInfo {
    fn default() -> Self {
        DiskInfo {
            name: [0u8; 64],
            sector_count: 0,
            sector_size: 0,
            present: false,
            is_ahci: false,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SysBootInfo {
    pub version: u32,
    pub gop_fb_base: u64,
    pub gop_width: i32,
    pub gop_height: i32,
    pub gop_pitch: i32,
}

pub type ModuleEntryFn = unsafe fn(*const c_void, *mut *mut c_void) -> i32;
