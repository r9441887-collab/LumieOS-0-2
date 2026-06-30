#![no_std]

use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;

extern "C" {
    fn term_clear(bg: u32);
    fn term_set_fg(c: u32);
    fn term_set_bg(c: u32);
    fn term_write(s: *const u8);
    fn term_writeln(s: *const u8);
    fn fat_set_device(handle: efi_handle) -> i32;
    fn fat_write_file(path: *const u8, data: *const c_void, size: u32) -> i32;
    fn fat_exists(path: *const u8) -> i32;
    fn fat_format(total_sectors: u64) -> i32;
    fn fat_mkdir(path: *const u8) -> i32;
    fn fat_install_bootloader() -> i32;
    fn lumie_efi_register_boot_entry() -> i32;
    fn pit_stall(us: u32);
    fn pit_get_ticks() -> u64;
    fn install_pkg_open(path: *const u8, pkg: *mut c_void) -> i32;
    fn install_pkg_extract_all(pkg: *mut c_void, progress: *mut c_void) -> i32;
    fn install_pkg_close(pkg: *mut c_void);
}

fn loader_console_install(phase: *const u8, pct: i32) {
    unsafe {
        term_set_fg(0x55FFFF);
        term_write(b"[" as *const u8);
        let bar_w = 40;
        let filled = (pct * bar_w) / 100;
        for i in 0..bar_w {
            if i < filled { term_write(b"#" as *const u8); }
            else { term_write(b"." as *const u8); }
        }
        term_write(b"] " as *const u8);
        let mut pc: [u8; 8] = [0u8; 8];
        lumie_std::format::lumie_itoa(pct as i64, pc.as_mut_ptr(), 10);
        let mut pi = 0;
        while pi < 8 && pc[pi] != 0 {
            let c = [pc[pi], 0u8];
            term_write(c.as_ptr());
            pi += 1;
        }
        term_write(b"% " as *const u8);
        term_set_fg(0xFFFFFF);
        if !phase.is_null() { term_writeln(phase); }
    }
}

fn loader_text_confirm(msg: *const u8) -> bool {
    unsafe {
        term_write(msg);
        term_write(b" (y/n): " as *const u8);
        loop {
            let c = crate::input::loader_getchar();
            if c == b'y' as i32 || c == b'Y' as i32 {
                term_writeln(b"y" as *const u8);
                return true;
            }
            if c == b'n' as i32 || c == b'N' as i32 {
                term_writeln(b"n" as *const u8);
                return false;
            }
        }
    }
}

pub fn loader_install_screen() {
    unsafe {
        term_clear(0);
        term_set_bg(0);
        term_set_fg(0xFFFFFF);
        term_writeln(b"=== LumieOS Installer ===" as *const u8);
        term_writeln(b"" as *const u8);
    }

    let bs = match unsafe { crate::input::get_ld_st() } {
        st if !st.is_null() => unsafe { (*st).boot_services },
        _ => { unsafe { term_writeln(b"ERROR: No UEFI boot services." as *const u8); } return; }
    };

    let mut install_devices: [crate::devices::LoaderBlockDevice; 16] =
        unsafe { core::mem::zeroed() };
    let dev_count = crate::devices::loader_enum_block_devices(unsafe { &*bs }, &mut install_devices);
    let target_device: i32;

    if dev_count == 0 {
        unsafe {
            term_writeln(b"ERROR: No block devices found." as *const u8);
            term_writeln(b"Press any key to return..." as *const u8);
            crate::input::loader_getchar();
        }
        return;
    }

    if dev_count == 1 {
        target_device = 0;
        unsafe {
            term_write(b"Target device: " as *const u8);
            let mut lbl = [0u8; 65];
            let mut lp = 0;
            for &c in install_devices[0].label.iter() {
                if c == 0 { break; }
                lbl[lp] = c; lp += 1;
            }
            lbl[lp] = 0;
            term_writeln(lbl.as_ptr());
        }
    } else {
        unsafe {
            term_writeln(b"Available devices:" as *const u8);
            for i in 0..dev_count as usize {
                let mut buf: [u8; 128] = [0u8; 128];
                let mut bp = 0;
                buf[bp] = b' '; bp += 1; buf[bp] = b' '; bp += 1;
                let mut num: [u8; 8] = [0u8; 8];
                lumie_std::format::lumie_itoa((i + 1) as i64, num.as_mut_ptr(), 10);
                for &c in num.iter() { if c == 0 { break; } buf[bp] = c; bp += 1; }
                buf[bp] = b'.'; bp += 1; buf[bp] = b' '; bp += 1;
                for &c in install_devices[i].label.iter() {
                    if c == 0 { break; }
                    buf[bp] = c; bp += 1;
                }
                buf[bp] = 0;
                term_writeln(buf.as_ptr());
            }
            term_write(b"Select device (1-" as *const u8);
            let mut num: [u8; 8] = [0u8; 8];
            lumie_std::format::lumie_itoa(dev_count as i64, num.as_mut_ptr(), 10);
            let mut ni = 0;
            while ni < 8 && num[ni] != 0 {
                let c = [num[ni], 0u8];
                term_write(c.as_ptr());
                ni += 1;
            }
            term_write(b"): " as *const u8);
            loop {
                let c = crate::input::loader_getchar();
                if c >= b'1' as i32 && c <= b'0' as i32 + dev_count {
                    target_device = c - b'1' as i32;
                    term_writeln(b"" as *const u8);
                    break;
                }
            }
        }
    }

    if !loader_text_confirm(b"Format and install LumieOS on this device?" as *const u8) {
        unsafe { term_writeln(b"Installation cancelled." as *const u8); }
        return;
    }

    if unsafe { fat_set_device(install_devices[target_device as usize].handle) } != 0 {
        unsafe {
            term_writeln(b"ERROR: Failed to access target device." as *const u8);
            crate::input::loader_getchar();
        }
        return;
    }

    let mut pkg: [u8; 256] = [0u8; 256];
    let mut pkg_found = false;

    if !crate::get_boot_device().is_null() {
        unsafe { fat_set_device(crate::get_boot_device()); }
        if unsafe { install_pkg_open(b"install.pkg\0" as *const u8, pkg.as_mut_ptr() as *mut c_void) } == 0 {
            pkg_found = true;
        }
        unsafe { fat_set_device(install_devices[target_device as usize].handle); }
    }

    if !pkg_found {
        unsafe {
            term_writeln(b"ERROR: install.pkg not found on boot device." as *const u8);
            term_writeln(b"Press any key to return..." as *const u8);
            crate::input::loader_getchar();
        }
        return;
    }

    /* Format */
    unsafe { term_writeln(b"Formatting disk..." as *const u8); }
    loader_console_install(b"Formatting..." as *const u8, 10);

    let mut total_sectors: u64 = 0;
    let bio_guid = &EFI_BLOCK_IO_GUID as *const EfiGuid;
    let mut bio: *mut c_void = ptr::null_mut();
    let es = unsafe {
        if let Some(hp) = (*bs).handle_protocol {
            hp(install_devices[target_device as usize].handle, bio_guid, &mut bio)
        } else { 1 }
    };
    if es == 0 && !bio.is_null() {
        let media_ptr = unsafe { *(bio as *mut *mut c_void).add(1) };
        if !media_ptr.is_null() {
            let last_block = unsafe { *(media_ptr as *mut u64).add(2) };
            total_sectors = last_block + 1;
        }
    }
    if total_sectors == 0 { total_sectors = 1024 * 1024; }
    unsafe { fat_format(total_sectors); }
    loader_console_install(b"Format done" as *const u8, 20);

    /* Directories */
    loader_console_install(b"Creating directories..." as *const u8, 30);
    if unsafe { fat_exists(b"/system\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/system\0" as *const u8); } }
    if unsafe { fat_exists(b"/drivers\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/drivers\0" as *const u8); } }
    if unsafe { fat_exists(b"/EFI\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/EFI\0" as *const u8); } }
    if unsafe { fat_exists(b"/EFI/BOOT\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/EFI/BOOT\0" as *const u8); } }

    /* Extract */
    loader_console_install(b"Extracting system files..." as *const u8, 50);
    unsafe { install_pkg_extract_all(pkg.as_mut_ptr() as *mut c_void, ptr::null_mut()); }

    /* Bootloader */
    loader_console_install(b"Installing bootloader..." as *const u8, 80);
    unsafe { fat_install_bootloader(); lumie_efi_register_boot_entry(); }

    /* Timezone */
    loader_console_install(b"Setting timezone..." as *const u8, 90);
    unsafe {
        term_writeln(b"" as *const u8);
        term_writeln(b"Select timezone:" as *const u8);
        term_writeln(b"  1. Moscow (UTC+3)" as *const u8);
        term_writeln(b"  2. Krasnoyarsk (UTC+7)" as *const u8);
        term_write(b"Choice (1-2): " as *const u8);
    }
    let mut tz_sel: i32 = 0;
    loop {
        let c = unsafe { crate::input::loader_getchar() };
        if c == b'1' as i32 { tz_sel = 0; break; }
        if c == b'2' as i32 { tz_sel = 1; break; }
    }
    unsafe { term_writeln(b"" as *const u8); }

    let tz_offsets: [i32; 2] = [180, 420];
    let mut tz_buf: [u8; 16] = [0u8; 16];
    unsafe {
        lumie_std::format::lumie_itoa(tz_offsets[tz_sel as usize] as i64, tz_buf.as_mut_ptr(), 10);
        let mut len = 0;
        while len < 16 && tz_buf[len] != 0 { len += 1; }
        fat_write_file(b"/system/timezone.cfg\0" as *const u8, tz_buf.as_ptr() as *const c_void, (len + 1) as u32);
        install_pkg_close(pkg.as_mut_ptr() as *mut c_void);
    }

    loader_console_install(b"Installation complete!" as *const u8, 100);
    unsafe {
        term_writeln(b"" as *const u8);
        term_writeln(b"LumieOS installed successfully! Press any key to reboot..." as *const u8);
        crate::input::loader_getchar();
    }

    /* Reboot via LumieOS reboot function */
    unsafe { lumie_reboot(); }
}

extern "C" {
    fn lumie_reboot();
}
