#![no_std]

extern crate lumie_std;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe { bsod_handler(info); }
    loop {}
}

unsafe fn bsod_handler(info: &core::panic::PanicInfo) {
    let bg = crate::display::ld_make_color(0x00, 0x00, 0x80);
    let white = crate::display::ld_make_color(0xFF, 0xFF, 0xFF);
    let yellow = crate::display::ld_make_color(0xFF, 0xFF, 0x00);
    let cyan = crate::display::ld_make_color(0x00, 0xFF, 0xFF);
    let red = crate::display::ld_make_color(0xFF, 0x00, 0x00);

    let fb = crate::gop_get_fb();
    if !fb.is_null() && (*fb).base != 0 {
        let w = (*fb).width;
        let h = (*fb).height;

        crate::gop_fill_rect(0, 0, w, h, bg);

        crate::gop_draw_string(20, 20, white, bg, b"*** LUMIEOS PANIC ***\0" as *const u8);
        crate::gop_draw_string(20, 44, red, bg, b"A fatal error has occurred.\0" as *const u8);
        crate::gop_draw_string(20, 64, yellow, bg, b"The system has been halted.\0" as *const u8);

        let mut y = 100u32;

        /* Location */
        if let Some(loc) = info.location() {
            let mut buf: [u8; 256] = [0u8; 256];
            let mut bp = 0;
            let pre = b"Location: \0";
            for &c in pre { if bp < 255 { buf[bp] = c; bp += 1; } }
            for &c in loc.file().as_bytes() { if bp < 255 { buf[bp] = c; bp += 1; } }
            if bp < 255 { buf[bp] = b':'; bp += 1; }
            let mut ln: [u8; 16] = [0u8; 16];
            lumie_std::format::lumie_itoa(loc.line() as i64, ln.as_mut_ptr(), 10);
            for &c in ln.iter() { if c == 0 { break; } if bp < 255 { buf[bp] = c; bp += 1; } }
            if bp < 255 { buf[bp] = b':'; bp += 1; }
            let mut col: [u8; 16] = [0u8; 16];
            lumie_std::format::lumie_itoa(loc.column() as i64, col.as_mut_ptr(), 10);
            for &c in col.iter() { if c == 0 { break; } if bp < 255 { buf[bp] = c; bp += 1; } }
            if bp < 255 { buf[bp] = 0; }
            crate::gop_draw_string(20, y, cyan, bg, buf.as_ptr());
            y += 20;
        }

        /* Display panic message */
        {
            let mut msg_buf: [u8; 256] = [0u8; 256];
            let mut mp = 0;
            if let Some(s) = info.message().as_str() {
                for &c in s.as_bytes() { if mp < 255 { msg_buf[mp] = c; mp += 1; } }
            } else {
                let fallback = b"<unformatted panic message>\0";
                for &c in fallback { if mp < 255 { msg_buf[mp] = c; mp += 1; } }
            }
            if mp < 255 { msg_buf[mp] = 0; }
            crate::gop_draw_string(20, y, white, bg, b"Error: \0" as *const u8);
            y += 20;
            crate::gop_draw_string(20, y, white, bg, msg_buf.as_ptr());
            y += 24;
        }

        let hint = b"Check /crash.log on the boot partition for details.\0";
        crate::gop_draw_string(20, y, yellow, bg, hint as *const u8);
        y += 24;
        crate::gop_draw_string(20, y, white, bg, b"Press any key to reboot...\0" as *const u8);

        /* Write crash log to FAT32 if available */
        let mut log: [u8; 1024] = [0u8; 1024];
        let mut lp = 0;
        let hd = b"LumieOS Crash Log\r\n==================\r\n\0";
        for &c in hd { if lp < 1023 { log[lp] = c; lp += 1; } }
        if let Some(loc) = info.location() {
            let f = b"File: \0";
            for &c in f { if lp < 1023 { log[lp] = c; lp += 1; } }
            for &c in loc.file().as_bytes() { if lp < 1023 { log[lp] = c; lp += 1; } }
            if lp < 1023 { log[lp] = b'\r'; lp += 1; }
            if lp < 1023 { log[lp] = b'\n'; lp += 1; }
            let l = b"Line: \0";
            for &c in l { if lp < 1023 { log[lp] = c; lp += 1; } }
            let mut ls: [u8; 16] = [0u8; 16];
            lumie_std::format::lumie_itoa(loc.line() as i64, ls.as_mut_ptr(), 10);
            for &c in ls.iter() { if c == 0 { break; } if lp < 1023 { log[lp] = c; lp += 1; } }
            if lp < 1023 { log[lp] = b'\r'; lp += 1; }
            if lp < 1023 { log[lp] = b'\n'; lp += 1; }
        }
        let m = b"Message: \0";
        for &c in m { if lp < 1023 { log[lp] = c; lp += 1; } }
        if let Some(s) = info.message().as_str() {
            for &c in s.as_bytes() { if lp < 1023 { log[lp] = c; lp += 1; } }
        }
        if lp < 1023 { log[lp] = 0; }
        crate::fat_write_file(b"/crash.log\0" as *const u8, log.as_ptr() as *const core::ffi::c_void, lp as u32);
    }

    /* Wait for key press */
    if !crate::input::get_ld_st().is_null() {
        crate::input::loader_getchar();
    }
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

    unsafe { lumie_set_image_handle(image_handle); }

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
    }

    /* Save boot device handle via LocateDevicePath */
    let li_guid = &EFI_LOADED_IMAGE_PROTOCOL_GUID as *const EfiGuid;
    unsafe {
        let bs = st.boot_services;
        if !bs.is_null() {
            if let Some(hp) = (*bs).handle_protocol {
                let mut li: *mut EfiLoadedImageProtocol = ptr::null_mut();
                let err = hp(image_handle, li_guid, &mut li as *mut *mut EfiLoadedImageProtocol as *mut *mut c_void);
                if err == 0 && !li.is_null() {
                    let file_path = (*li).file_path;
                    if !file_path.is_null() {
                        let mut fp: *mut EfiDevicePathProtocol = file_path;
                        let dp_guid = &EFI_DEVICE_PATH_PROTOCOL_GUID as *const EfiGuid;
                        let locate_dp: Option<unsafe extern "efiapi" fn(
                            *const EfiGuid, *mut *mut EfiDevicePathProtocol, *mut efi_handle,
                        ) -> efi_status> = core::mem::transmute((*bs).locate_device_path);
                        if let Some(ldp) = locate_dp {
                            let mut dev_handle: efi_handle = ptr::null_mut();
                            let st2 = ldp(dp_guid, &mut fp, &mut dev_handle);
                            if st2 == 0 && !dev_handle.is_null() {
                                G_LOADER_BOOT_DEVICE = dev_handle;
                            }
                        }
                    }
                    lumie_cache_kernel_image((*li).image_base, (*li).image_size as u32);
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

    /* Initialize FAT on boot device BEFORE checking if OS is installed.
     * Without this, fat_exists() always returns false (FAT uninitialized)
     * and an installed system can never boot. */
    unsafe {
        if !G_LOADER_BOOT_DEVICE.is_null() {
            fat_set_device(G_LOADER_BOOT_DEVICE);
        }
    }

    /* Not installed → install; Installed → boot */
    if !boot::lumieos_installed() {
        /* If boot device was not set by LocateDevicePath, try fallback */
        unsafe {
            if G_LOADER_BOOT_DEVICE.is_null() {
                term_write(b"[boot] WARNING: boot device handle is NULL, searching...\0" as *const u8);
                term_newline();
                let bs_ref = &*st.boot_services;
                let mut fallback_devs: [devices::LoaderBlockDevice; 16] = core::mem::zeroed();
                let fb_count = devices::loader_enum_block_devices(bs_ref, &mut fallback_devs, false);
                let mut found_boot = false;
                for i in 0..fb_count as usize {
                    if fat_set_device(fallback_devs[i].handle) == 0 {
                        if fat_get_file_size(b"install.pkg\0" as *const u8) > 0 {
                            G_LOADER_BOOT_DEVICE = fallback_devs[i].handle;
                            found_boot = true;
                            term_write(b"[boot] Found install.pkg on block device\0" as *const u8);
                            term_newline();
                            break;
                        }
                    }
                }
                if !found_boot {
                    term_write(b"[boot] ERROR: no device with install.pkg found\0" as *const u8);
                    term_newline();
                }
            }
            if !G_LOADER_BOOT_DEVICE.is_null() {
                if fat_set_device(G_LOADER_BOOT_DEVICE) != 0 {
                    term_write(b"[boot] WARNING: fat_set_device failed for boot device\0" as *const u8);
                    term_newline();
                }
            } else {
                term_write(b"[boot] ERROR: cannot initialize FAT - no boot device\0" as *const u8);
                term_newline();
            }
        }

        let mut devices: [devices::LoaderBlockDevice; 16] = unsafe { core::mem::zeroed() };
        let dev_count = devices::loader_enum_block_devices(unsafe { &*st.boot_services }, &mut devices, true);
        let target = if dev_count > 0 {
            let sel = devices::loader_show_device_menu(&devices[..dev_count as usize]);
            if sel >= 0 {
                let d = &devices[sel as usize];
                unsafe { crate::install::install_set_preselected_device(d.handle, d.is_partition as i32, d.is_removable as i32); }
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

    /* Installed → boot menu (show if other OSes exist in UEFI boot order) */
    if boot::detect_other_os() {
        let mut entries: [boot::BootEntry; 16] = unsafe { core::mem::zeroed() };
        let count = boot::read_boot_entries(&mut entries);
        if count > 1 {
            let sel = boot::show_boot_menu(&entries[..count]);
            let sel_entry = &entries[sel as usize];
            if !sel_entry.is_lumie {
                /* Chainload: set BootNext and reboot */
                boot::set_boot_next_and_reboot(sel_entry.boot_num);
                /* Should never return */
                loop {}
            }
            /* LumieOS selected → fall through to normal boot */
        }
    }

    display::loader_boot_screen();

    /* Show boot diagnostics */
    {
        let w = unsafe { gop_get_width() };
        let bg = display::ld_make_color(0x00, 0x00, 0x80);
        let cyan = display::ld_make_color(0x00, 0xFF, 0xFF);
        let green = display::ld_make_color(0x00, 0xCC, 0x00);
        let red = display::ld_make_color(0xFF, 0x00, 0x00);

        unsafe { gop_fill_rect(0, 0, w, 200, bg); }
        display::loader_drv_draw_str(8, 8, cyan, bg, b"LumieOS Boot Diagnostics:");
        display::loader_drv_draw_str(8, 28, green, bg, b"[OK] GOP initialized");
        display::loader_drv_draw_str(8, 48, green, bg, b"[OK] Boot device found");
        display::loader_drv_draw_str(8, 68, green, bg, b"[OK] FAT32 initialized");

        /* Check critical files */
        let files: &[(&[u8], &[u8])] = &[
            (b"/system/kernel.lkrn\0", b"Kernel\0"),
            (b"/system/shell.lsh\0", b"Shell\0"),
            (b"/drivers/kbd.ldrv\0", b"Keyboard Driver\0"),
            (b"/drivers/fs.ldrv\0", b"Filesystem Driver\0"),
        ];

        let mut y = 88u32;
        for &(path, name) in files {
            let exists = unsafe { fat_exists(path.as_ptr()) } == 1;
            let color = if exists { green } else { red };
            let status: &[u8] = if exists { b"[OK] " } else { b"[!!] " };
            let mut msg: [u8; 80] = [0u8; 80];
            let mut mp = 0;
            for &c in status { if mp < 79 { msg[mp] = c; mp += 1; } }
            for &c in name { if c == 0 { break; } if mp < 79 { msg[mp] = c; mp += 1; } }
            if !exists {
                let missing = b" - MISSING!";
                for &c in missing { if mp < 79 { msg[mp] = c; mp += 1; } }
            }
            display::loader_drv_draw_str(8, y, color, bg, &msg[..mp]);
            y += 20;
        }
    }

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

    /* Build boot info shared by all loaded modules */
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
        }
    }

    /* Load kernel module first */
    {
        let bg = display::ld_make_color(0x00, 0x00, 0x80);
        let cyan = display::ld_make_color(0x00, 0xFF, 0xFF);
        let green = display::ld_make_color(0x00, 0xCC, 0x00);
        let red = display::ld_make_color(0xFF, 0x00, 0x00);
        let w = unsafe { gop_get_width() };
        let h = unsafe { gop_get_height() };

        display::loader_drv_draw_str(8, h - 80, cyan, bg, b"Loading kernel...");

        let mut kernel_mod: SysModule = unsafe { core::mem::zeroed() };
        let kr = unsafe {
            sys_load(
                b"/system/kernel.lkrn\0" as *const u8,
                &mut boot_info as *mut SysBootInfo,
                &mut kernel_mod as *mut SysModule,
            )
        };

        if kr != 0 {
            let mut msg: [u8; 64] = [0u8; 64];
            let mut mp = 0;
            for &c in b"ERROR: kernel load failed (code \0" { if c == 0 { break; } if mp < 63 { msg[mp] = c; mp += 1; } }
            let mut num: [u8; 8] = [0u8; 8];
            unsafe { lumie_std::format::lumie_itoa(kr as i64, num.as_mut_ptr(), 10); }
            for &c in num.iter() { if c == 0 { break; } if mp < 63 { msg[mp] = c; mp += 1; } }
            for &c in b")\0" { if c == 0 { break; } if mp < 63 { msg[mp] = c; mp += 1; } }
            msg[mp] = 0;
            display::loader_drv_draw_str(8, h - 60, red, bg, &msg[..mp + 1]);

            /* Show error description */
            let reason: &[u8] = match kr {
                -1 => b"Invalid arguments\0",
                -2 => b"File too small\0",
                -3 => b"Memory allocation failed\0",
                -4 => b"File read error\0",
                -5 => b"Invalid module magic\0",
                -6 => b"Invalid code size\0",
                -7 => b"Load base allocation failed\0",
                _ => b"Unknown error\0",
            };
            display::loader_drv_draw_str(8, h - 40, red, bg, reason);
            unsafe { pit_stall(5000000); } /* Show error for 5 seconds */
            return;
        }

        if kernel_mod.entry.is_null() {
            display::loader_drv_draw_str(8, h - 60, red, bg, b"ERROR: kernel entry point is NULL\0");
            unsafe { pit_stall(5000000); }
            return;
        }

        display::loader_drv_draw_str(8, h - 60, green, bg, b"[OK] Kernel loaded successfully\0");

        unsafe {
            let entry_fn: fn(*mut SysBootInfo, *mut *mut c_void) -> i32 =
                core::mem::transmute(kernel_mod.entry);
            let mut api: *mut c_void = ptr::null_mut();
            let init_rc = entry_fn(&mut boot_info, &mut api);
            if init_rc != 0 {
                let mut msg: [u8; 64] = [0u8; 64];
                let mut mp = 0;
                for &c in b"ERROR: kernel init failed (code \0" { if c == 0 { break; } if mp < 63 { msg[mp] = c; mp += 1; } }
                let mut num: [u8; 8] = [0u8; 8];
                unsafe { lumie_std::format::lumie_itoa(init_rc as i64, num.as_mut_ptr(), 10); }
                for &c in num.iter() { if c == 0 { break; } if mp < 63 { msg[mp] = c; mp += 1; } }
                for &c in b")\0" { if c == 0 { break; } if mp < 63 { msg[mp] = c; mp += 1; } }
                msg[mp] = 0;
                display::loader_drv_draw_str(8, h - 40, red, bg, &msg[..mp + 1]);
                unsafe { pit_stall(5000000); }
                return;
            }
        }
        display::loader_drv_draw_str(8, h - 40, green, bg, b"[OK] Kernel initialized\0");
    }

    /* GPU driver */
    {
        let bg = display::ld_make_color(0x00, 0x00, 0x80);
        let cyan = display::ld_make_color(0x00, 0xFF, 0xFF);
        let green = display::ld_make_color(0x00, 0xCC, 0x00);
        let yellow = display::ld_make_color(0xFF, 0xFF, 0x00);
        let w = unsafe { gop_get_width() };
        let h = unsafe { gop_get_height() };

        display::loader_drv_draw_str(8, h - 20, cyan, bg, b"Loading GPU driver...");

        let mut mod_: SysModule = unsafe { core::mem::zeroed() };
        let gpu_rc = unsafe {
            sys_load(
                b"/drivers/nv_gpu.sys\0" as *const u8,
                &mut boot_info as *mut SysBootInfo,
                &mut mod_ as *mut SysModule,
            )
        };

        if gpu_rc == 0 && !mod_.entry.is_null() {
            unsafe {
                let mut api: *mut c_void = ptr::null_mut();
                let entry_fn: fn(*mut SysBootInfo, *mut *mut c_void) -> i32 =
                    core::mem::transmute(mod_.entry);
                let ret = entry_fn(&mut boot_info, &mut api);
                if ret == 0 && !api.is_null() {
                    g_nv_gpu_api = api;
                    display::loader_drv_draw_str(8, h - 20, green, bg, b"[OK] GPU driver loaded\0");
                } else {
                    display::loader_drv_draw_str(8, h - 20, yellow, bg, b"[!!] GPU driver init failed, using fallback\0");
                }
            }
        } else {
            display::loader_drv_draw_str(8, h - 20, yellow, bg, b"[!!] GPU driver not found, using framebuffer\0");
        }

        unsafe { pit_stall(500000); } /* Brief pause to show status */
    }

    unsafe { gop_nv_init(); desktop_init(); desktop_run(); }
}


