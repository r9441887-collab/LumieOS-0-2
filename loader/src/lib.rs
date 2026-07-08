#![no_std]

extern crate lumie_std;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub mod uefi;
mod ffi;
pub use crate::ffi::*;
pub mod display;
pub mod input;
pub mod devices;
pub mod install;
pub mod boot;
pub mod gpt;
pub mod font;

use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;

/* ------------------------------------------------------------------ */
/*  C-compatible structs                                              */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FbInfo {
    pub base: u64,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u32,
    pub pixel_format: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SysBootInfo {
    pub version: u32,
    pub alloc: Option<unsafe fn(u32) -> *mut c_void>,
    pub free: Option<unsafe fn(*mut c_void)>,
    pub log: Option<unsafe fn(*const u8)>,
    pub log_hex: Option<unsafe fn(u64)>,
    pub gop_fb_base: u64,
    pub gop_width: u32,
    pub gop_height: u32,
    pub gop_pitch: u32,
}

pub const SYS_BOOT_INFO_VERSION: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SysModule {
    pub base: *mut c_void,
    pub size: u32,
    pub entry: *mut c_void,
}

/* Boot device handle (saved during loader init) */
static mut G_LOADER_BOOT_DEVICE: efi_handle = core::ptr::null_mut();

pub fn get_boot_device() -> efi_handle {
    unsafe { G_LOADER_BOOT_DEVICE }
}

/* ------------------------------------------------------------------ */
/*  Entry point                                                       */
/* ------------------------------------------------------------------ */

#[no_mangle]
pub extern "efiapi" fn lumie_loader_start(
    image_handle: efi_handle,
    system_table: *mut EfiSystemTable,
) {
    let st = unsafe { &*system_table };

    /* Clear screen, disable cursor */
    let con_out = st.con_out;
    if !con_out.is_null() {
        unsafe {
            if let Some(cs) = (*con_out).clear_screen {
                cs(con_out as *mut c_void);
            }
            if let Some(ec) = (*con_out).enable_cursor {
                ec(con_out as *mut c_void, 0);
            }
        }
    }

    /* Initialize GOP */
    let status = unsafe { gop_init(image_handle, system_table) };
    if status != 0 {
        if !con_out.is_null() {
            let msg: [u16; 17] = [
                b'G' as u16, b'O' as u16, b'P' as u16, b' ' as u16, b'i' as u16,
                b'n' as u16, b'i' as u16, b't' as u16, b' ' as u16, b'f' as u16,
                b'a' as u16, b'i' as u16, b'l' as u16, b'e' as u16, b'd' as u16,
                b'!' as u16, 0,
            ];
            unsafe {
                if let Some(os) = (*con_out).output_string {
                    os(con_out as *mut c_void, msg.as_ptr() as *mut u16);
                }
            }
        }
        return;
    }

    /* Init subsystems */
    unsafe {
        mm_init(st.boot_services, image_handle);
        ahci_init();
        pit_init(1000);
        pcspkr_init();
    }

    input::loader_kbd_init(system_table);
    input::loader_mouse_init(system_table);

    unsafe {
        kbd_init(system_table);
        term_init();
        fat_set_bs(st.boot_services, image_handle, system_table);
        fat_init();
    }

    /* Save boot device handle */
    let li_guid = &EFI_LOADED_IMAGE_PROTOCOL_GUID as *const EfiGuid;
    unsafe {
        let bs = st.boot_services;
        if !bs.is_null() {
            if let Some(hp) = (*bs).handle_protocol {
                let mut li: *mut core::ffi::c_void = ptr::null_mut();
                let err = hp(image_handle, li_guid, &mut li);
                if err == 0 && !li.is_null() {
                    let li_bytes = li as *mut u8;
                    let dh_ptr = li_bytes.add(24) as *mut efi_handle;
                    G_LOADER_BOOT_DEVICE = *dh_ptr;
                }
            }
        }
    }

    unsafe { mouse_init(system_table); }

    /* Cache kernel image */
    unsafe {
        if !st.boot_services.is_null() {
            if let Some(hp) = (*st.boot_services).handle_protocol {
                let mut li: *mut core::ffi::c_void = ptr::null_mut();
                let err2 = hp(image_handle, li_guid, &mut li);
                if err2 == 0 && !li.is_null() {
                    let li_bytes = li as *mut u8;
                    let base_ptr = li_bytes.add(40) as *mut *const c_void;  // image_base
                    let size_ptr = li_bytes.add(48) as *mut u64;            // image_size
                    let ib = *base_ptr;
                    let isz = *size_ptr;
                    if !ib.is_null() && isz > 0 {
                        lumie_cache_kernel_image(ib, isz as u32);
                    }
                }
            }
        }
    }

    /* Transition to own drivers */
    unsafe {
        kbd_switch_to_ps2();
        mouse_reinit_ps2();
        if ahci_is_ready() != 0 { fat_use_ahci(); }
    }

    unsafe { lumie_load_shell_module(); }

    /* Not installed → shell */
    if !boot::lumieos_installed() {
        /* Pre-load install.pkg into RAM disk */
        unsafe { if !G_LOADER_BOOT_DEVICE.is_null() {
                fat_set_device(G_LOADER_BOOT_DEVICE);
                let pkg_size = fat_get_file_size(b"install.pkg\0" as *const u8);
                if pkg_size > 0 {
                    let sz = pkg_size as u32;
                    let mut buf: *mut u8 = ptr::null_mut();
                    if let Some(ap) = (*st.boot_services).allocate_pool {
                        let err3 = ap(EFI_BOOT_SERVICES_DATA, sz as u64, &mut buf as *mut *mut u8 as *mut *mut c_void);
                        if err3 == 0 && !buf.is_null() {
                            let r = fat_read_file(
                                b"install.pkg\0" as *const u8, buf as *mut c_void, sz,
                            );
                            if r == sz as i32 {
                                ramdisk_init();
                                ramdisk_format_fat32();
                                fat_set_drive(
                                    ramdisk_read_sector_cb as *const () as usize,
                                    ramdisk_write_sector_cb as *const () as usize,
                                    ptr::null_mut(),
                                );
                                fat_reinit();
                                fat_write_file(
                                    b"install.pkg\0" as *const u8, buf as *const c_void, sz,
                                );
                            }
                            if let Some(fp) = (*st.boot_services).free_pool {
                                fp(buf as *mut c_void);
                            }
                        }
                    }
                }
            }
        }

        let mut devices: [devices::LoaderBlockDevice; 16] = unsafe { core::mem::zeroed() };
        let dev_count = devices::loader_enum_block_devices(unsafe { &*st.boot_services }, &mut devices);
        let target = if dev_count > 0 {
            let sel = devices::loader_show_device_menu(&devices[..dev_count as usize]);
            if sel >= 0 {
                let d = &devices[sel as usize];
                let mut cfg: [u8; 512] = [0u8; 512];
                let mut pos = 0;
                let prefix = b"blkio\n";
                for &c in prefix { if pos < 511 { cfg[pos] = c; pos += 1; } }
                let mut sb: [u8; 32] = [0u8; 32];
                unsafe { lumie_std::format::lumie_itoa(d.block_count as i64, sb.as_mut_ptr(), 10); }
                for &c in sb.iter() { if c == 0 { break; } if pos < 511 { cfg[pos] = c; pos += 1; } }
                if pos < 511 { cfg[pos] = b'\n'; pos += 1; }
                for &c in d.label.iter() { if c == 0 { break; } if pos < 511 { cfg[pos] = c; pos += 1; } }
                unsafe { fat_write_file(b"/system/target.cfg\0" as *const u8, cfg.as_ptr() as *const c_void, pos as u32); }
                sel
            } else { -1 }
        } else { -1 };
        if target < 0 {
            unsafe {
                exit_boot_services();
                lumie_sched_init();
                gop_nv_init();
                shell_run();
            }
            return;
        }

        crate::install::loader_install_screen();
        return;
    }

    /* Installed → boot screen */
    display::loader_boot_screen();
    unsafe { drvcheck_run_scan(); }

    /* 3 boot attempts */
    let mut boot_ok = false;
    for attempt in 1..=3 {
        if attempt > 1 { unsafe { pit_stall(250000); } }
        if !boot::lumieos_installed() {
            let w = unsafe { gop_get_width() };
            let bg = display::ld_make_color(0x00, 0x00, 0x80);
            unsafe { gop_fill_rect(0, 0, w, 24, bg); }
            boot::boot_display_msg(attempt, b"not on disk");
            continue;
        }
        if boot::loader_check_files() != 0 {
            boot::boot_display_msg(attempt, b"files missing");
            continue;
        }
        boot_ok = true;
        unsafe { bootcache_save(b"boot_ok\0" as *const u8); }
        break;
    }

    /* Auto-repair */
    if !boot_ok && boot::lumieos_installed() {
        let w = unsafe { gop_get_width() };
        let h = unsafe { gop_get_height() };
        let bg = display::ld_make_color(0x00, 0x00, 0x80);
        let yellow = display::ld_make_color(0xFF, 0xFF, 0x00);
        let green = display::ld_make_color(0x00, 0xCC, 0x00);

        unsafe {
            display::loader_drv_clear(bg);
            gop_draw_string(
                w / 2 - 12 * 8, h / 3, yellow, bg,
                b"Auto-repair: reinstalling system files...\0" as *const u8,
            );
            if fat_exists(b"/system\0" as *const u8) == 0 { fat_mkdir(b"/system\0" as *const u8); }
            if fat_exists(b"/drivers\0" as *const u8) == 0 { fat_mkdir(b"/drivers\0" as *const u8); }
            let mut rpkg: [u8; 256] = [0u8; 256];
            if install_pkg_open(
                    b"install.pkg\0" as *const u8, rpkg.as_mut_ptr() as *mut c_void,
                ) == 0 {
                    install_pkg_set_write_fn(Some(fat_write_file));
                    install_pkg_extract_all(rpkg.as_mut_ptr() as *mut c_void, ptr::null_mut());
                    install_pkg_close(rpkg.as_mut_ptr() as *mut c_void);
                }
            gop_draw_string(
                w / 2 - 8 * 8, h / 3 + 24, green, bg,
                b"Repair complete. Booting...\0" as *const u8,
            );
            pit_stall(1000000);
        }
    }

    unsafe { exit_boot_services(); lumie_sched_init(); }

    /* GPU driver */
    {
        let mut boot_info: SysBootInfo = unsafe { core::mem::zeroed() };
        unsafe {
            let fb = gop_get_fb();
            if !fb.is_null() {
                let fb_ref = &*fb;
                boot_info.version = SYS_BOOT_INFO_VERSION;
                boot_info.gop_fb_base = fb_ref.base;
                boot_info.gop_width = fb_ref.width;
                boot_info.gop_height = fb_ref.height;
                boot_info.gop_pitch = fb_ref.pitch;

                let mut mod_: SysModule = core::mem::zeroed();
                if sys_load(
                    b"\\drivers\\nv_gpu.sys\0" as *const u8,
                    &mut boot_info as *mut SysBootInfo,
                    &mut mod_ as *mut SysModule,
                ) == 0 && !mod_.entry.is_null()
                {
                    let mut api: *mut c_void = ptr::null_mut();
                    let entry_fn: fn(*mut SysBootInfo, *mut *mut c_void) -> i32 =
                        core::mem::transmute(mod_.entry);
                    let ret = entry_fn(&mut boot_info, &mut api);
                    if ret == 0 && !api.is_null() {
                        g_nv_gpu_api = api;
                    }
                }
            }
        }
    }

    unsafe { gop_nv_init(); desktop_init(); desktop_run(); }
}


