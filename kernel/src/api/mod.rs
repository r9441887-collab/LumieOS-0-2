use core::ffi::c_void;

pub const KAPI_VERSION: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
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

pub struct SysModule {
    pub entry: Option<fn(&SysBootInfo, &mut *mut u8) -> i32>,
    pub base: u64,
    pub size: u32,
}

impl Default for SysModule {
    fn default() -> Self {
        SysModule {
            entry: None,
            base: 0,
            size: 0,
        }
    }
}

pub struct LcCtx {
    pub errors: i32,
    pub code_pos: i32,
}

impl Default for LcCtx {
    fn default() -> Self {
        LcCtx {
            errors: 0,
            code_pos: 0,
        }
    }
}

pub const USER_ROLE_USER: i32 = 0;
pub const USER_ROLE_ADMIN: i32 = 1;

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

pub trait KernelApi {
    fn term_clear(&self, bg: u32);
    fn term_set_fg(&self, c: u32);
    fn term_set_bg(&self, c: u32);
    fn term_set_pos(&self, x: i32, y: i32);
    fn term_write(&self, s: &str);
    fn term_writeln(&self, s: &str);
    fn term_putchar(&self, c: u8);
    fn term_get_width(&self) -> i32;
    fn term_get_height(&self) -> i32;
    fn term_set_cursor(&self, visible: bool);

    fn kbd_getchar(&self) -> i32;
    fn kbd_kbhit(&self) -> i32;
    fn kbd_flush(&self);

    fn fs_read(&self, path: &str, buf: &mut [u8]) -> i32;
    fn fs_write(&self, path: &str, data: &[u8]) -> i32;
    fn fs_exists(&self, path: &str) -> bool;
    fn fs_list_dir(&self, path: &str, entries: &mut [LumieDirEnt]) -> i32;
    fn fs_mkdir(&self, path: &str) -> i32;
    fn fs_delete(&self, path: &str) -> i32;
    fn fs_get_size(&self, path: &str) -> i32;
    fn fs_format(&self, total_sectors: u64);
    fn fs_set_drive(&self, drive_letter: char);
    fn fs_get_current_drive(&self) -> char;
    fn fs_use_ahci(&self);
    fn fs_reinit(&self);
    fn fs_ramdisk_init(&self);
    fn fs_ramdisk_format_fat32(&self);

    fn gop_fill_rect(&self, x: u32, y: u32, w: u32, h: u32, color: u32);
    fn gop_put_pixel(&self, x: u32, y: u32, color: u32);
    fn gop_get_width(&self) -> u32;
    fn gop_get_height(&self) -> u32;
    fn gop_get_pitch(&self) -> u32;
    fn gop_fb_ptr(&self) -> *mut c_void;
    fn gop_fb_info(&self) -> FramebufferInfo;
    fn gop_flip(&self);
    fn gop_vsync(&self);
    fn gop_draw_char(&self, x: u32, y: u32, fg: u32, bg: u32, c: u8);
    fn gop_make_color(&self, r: u8, g: u8, b: u8) -> u32;

    fn mouse_poll(&self, ms: &mut MouseState) -> bool;
    fn mouse_restore(&self, x: i32, y: i32);
    fn mouse_draw(&self, x: i32, y: i32);
    fn mouse_get_pos(&self, x: &mut i32, y: &mut i32);
    fn mouse_set_visible(&self, v: bool);

    fn stall(&self, us: u64);
    fn reboot(&self);
    fn shutdown(&self);
    fn get_time(&self, buf: &mut [u8]) -> i32;

    fn sched_get_count(&self) -> i32;
    fn sched_get_name(&self, id: i32, buf: &mut [u8]) -> i32;
    fn sched_get_state(&self, id: i32) -> i32;
    fn sched_get_priority(&self, id: i32) -> u8;

    fn users_init(&self);
    fn users_current_name(&self, buf: &mut [u8]) -> i32;
    fn users_current_role(&self) -> i32;
    fn users_login(&self, name: &str, pass: Option<&str>) -> i32;
    fn users_is_logged_in(&self) -> bool;
    fn users_add(&self, name: &str, pass: &str, role: i32) -> i32;
    fn users_remove(&self, name: &str) -> i32;
    fn users_is_protected_path(&self, path: &str) -> bool;

    fn reg_init(&self);
    fn reg_get(&self, key: &str, val: &mut [u8]) -> i32;
    fn reg_set(&self, key: &str, val: &str) -> i32;
    fn reg_del(&self, key: &str) -> i32;
    fn reg_list(&self, buf: &mut [u8]) -> i32;
    fn reg_get_start(&self, buf: &mut [u8]) -> i32;

    fn disk_enum_all(&self) -> i32;
    fn disk_get_info(&self, id: i32, info: &mut DiskInfo) -> i32;
    fn disk_get_drive_letter(&self, id: i32) -> u8;

    fn ahci_is_ready(&self) -> bool;
    fn ahci_get_sector_count(&self) -> u64;

    fn net_init(&self) -> i32;
    fn net_renet_download(&self, name: Option<&str>);

    fn pcspkr_beep(&self, freq: u32, dur: u32);

    fn sys_load(&self, path: &str, bi: &SysBootInfo, mod_out: &mut SysModule) -> i32;
    fn lumie_get_kernel_image(&self, base: &mut *const u8, size: &mut u32) -> i32;
    fn lumie_pack_module(&self, data: &[u8], magic: u32, subtype: u32, name: &str, packed: &mut *mut u8, packed_sz: &mut u32) -> i32;

    fn pe_check(&self, buf: &[u8]) -> bool;
    fn pe_type(&self, buf: &[u8]) -> Option<&str>;
    fn pe_machine_str(&self, buf: &[u8]) -> Option<&str>;

    fn mm_get_free_mem(&self) -> u64;

    fn bootcache_clear(&self);
    fn bootcache_count(&self) -> i32;
    fn bootcache_load(&self, lines: &mut [[u8; 256]], max: i32) -> i32;

    fn drvcheck_run_scan(&self);

    fn lc_compile_file(&self, ctx: &mut LcCtx, path: &str) -> i32;
    fn lc_output_sys(&self, ctx: &LcCtx, path: &str, mod_name: &str) -> i32;

    fn extract_gzip_tar(&self, file: Option<&str>);
}
