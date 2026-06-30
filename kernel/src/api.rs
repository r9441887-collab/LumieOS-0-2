#![no_std]

use crate::uefi::types::*;

pub const KAPI_VERSION: u32 = 2;

pub type TermClearFn = Option<unsafe extern "C" fn(u32)>;
pub type TermSetFgFn = Option<unsafe extern "C" fn(u32)>;
pub type TermSetBgFn = Option<unsafe extern "C" fn(u32)>;
pub type TermSetPosFn = Option<unsafe extern "C" fn(i32, i32)>;
pub type TermWriteFn = Option<unsafe extern "C" fn(*const u8)>;
pub type TermWritelnFn = Option<unsafe extern "C" fn(*const u8)>;
pub type TermPutcharFn = Option<unsafe extern "C" fn(u8)>;
pub type TermGetWidthFn = Option<unsafe extern "C" fn() -> i32>;
pub type TermGetHeightFn = Option<unsafe extern "C" fn() -> i32>;
pub type KbdGetcharFn = Option<unsafe extern "C" fn() -> i32>;
pub type KbdKbhitFn = Option<unsafe extern "C" fn() -> i32>;
pub type KmallocFn = Option<unsafe extern "C" fn(u64) -> *mut core::ffi::c_void>;
pub type KfreeFn = Option<unsafe extern "C" fn(*mut core::ffi::c_void)>;
pub type KcallocFn = Option<unsafe extern "C" fn(u64, u64) -> *mut core::ffi::c_void>;
pub type KmemsetFn = Option<unsafe extern "C" fn(*mut core::ffi::c_void, i32, u64)>;
pub type KmemcpyFn = Option<unsafe extern "C" fn(*mut core::ffi::c_void, *const core::ffi::c_void, u64)>;
pub type FsReadFn = Option<unsafe extern "C" fn(*const u8, *mut core::ffi::c_void, u32) -> i32>;
pub type FsWriteFn = Option<unsafe extern "C" fn(*const u8, *const core::ffi::c_void, u32) -> i32>;
pub type FsExistsFn = Option<unsafe extern "C" fn(*const u8) -> i32>;
pub type FsListFn = Option<unsafe extern "C" fn(*const u8, *mut core::ffi::c_void, i32) -> i32>;
pub type FsMkdirFn = Option<unsafe extern "C" fn(*const u8) -> i32>;
pub type PrintfFn = Option<unsafe extern "C" fn(*const u8, ...)>;
pub type StallFn = Option<unsafe extern "C" fn(u64)>;
pub type RebootFn = Option<unsafe extern "C" fn()>;
pub type ShutdownFn = Option<unsafe extern "C" fn()>;
pub type GpuFillRectFn = Option<unsafe extern "C" fn(u32, u32, u32, u32, u32) -> i32>;
pub type GpuPutPixelFn = Option<unsafe extern "C" fn(u32, u32, u32) -> i32>;
pub type GpuGetPixelFn = Option<unsafe extern "C" fn(u32, u32) -> u32>;
pub type GpuIsActiveFn = Option<unsafe extern "C" fn() -> i32>;
pub type GpuFlipFn = Option<unsafe extern "C" fn()>;
pub type GpuVsyncFn = Option<unsafe extern "C" fn()>;
pub type ModLoadFn = Option<unsafe extern "C" fn(*const u8, *mut *mut core::ffi::c_void) -> i32>;
pub type ModUnloadFn = Option<unsafe extern "C" fn(i32)>;
pub type MemTotalFn = Option<unsafe extern "C" fn() -> u64>;
pub type MemFreeFn = Option<unsafe extern "C" fn() -> u64>;
pub type MemUsedFn = Option<unsafe extern "C" fn() -> u64>;
pub type DiskReadFn = Option<unsafe extern "C" fn(i32, u64, u32, *mut core::ffi::c_void) -> i32>;
pub type DiskWriteFn = Option<unsafe extern "C" fn(i32, u64, u32, *const core::ffi::c_void) -> i32>;
pub type DiskCountFn = Option<unsafe extern "C" fn() -> i32>;
pub type DiskNameFn = Option<unsafe extern "C" fn(i32) -> *const u8>;
pub type DiskSectorsFn = Option<unsafe extern "C" fn(i32) -> u64>;
pub type PciScanFn = Option<unsafe extern "C" fn(i32, *mut u16, *mut u16, *mut u8) -> i32>;
pub type PciVendorStrFn = Option<unsafe extern "C" fn(u16) -> *const u8>;
pub type PciDeviceStrFn = Option<unsafe extern "C" fn(u16, u16) -> *const u8>;
pub type GetTimeFn = Option<unsafe extern "C" fn(*mut u8, i32) -> i32>;
pub type SchedCountFn = Option<unsafe extern "C" fn() -> i32>;
pub type SchedNameFn = Option<unsafe extern "C" fn(i32) -> *const u8>;
pub type SchedStateFn = Option<unsafe extern "C" fn(i32) -> i32>;

#[repr(C)]
pub struct KernelApi {
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

    pub desktop_ctx: *mut core::ffi::c_void,

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
