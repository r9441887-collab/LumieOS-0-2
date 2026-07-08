
use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;
use crate::ffi::*;

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

fn read_number() -> u64 {
    let mut val: u64 = 0;
    loop {
        let c = crate::input::loader_getchar();
        if c >= b'0' as i32 && c <= b'9' as i32 {
            let digit = (c - b'0' as i32) as u64;
            val = val * 10 + digit;
            let ch = [c as u8, 0u8];
            unsafe { term_write(ch.as_ptr()); }
        } else if c == b'\n' as i32 || c == b'\r' as i32 {
            break;
        } else {
            break;
        }
    }
    val
}

fn detect_other_os_in_boot_order() -> bool {
    let st_ptr = crate::input::get_ld_st();
    if st_ptr.is_null() { return false; }
    let rt = unsafe { (*st_ptr).runtime_services };
    if rt.is_null() { return false; }

    let global_guid = &EFI_GLOBAL_VARIABLE_GUID as *const EfiGuid;
    let boot_order_name: [u16; 10] = [
        b'B' as u16, b'o' as u16, b'o' as u16, b't' as u16,
        b'O' as u16, b'r' as u16, b'd' as u16, b'e' as u16,
        b'r' as u16, 0,
    ];
    let mut boot_order_buf: [u16; 128] = [0u16; 128];
    let mut boot_order_size: u64 = 256;
    let mut attrs: u32 = 0;

    let gv = unsafe { (*rt).get_variable };
    let st = match gv {
        Some(g) => unsafe {
            g(boot_order_name.as_ptr() as *mut u16, global_guid, &mut attrs, &mut boot_order_size, boot_order_buf.as_mut_ptr() as *mut c_void)
        },
        None => return false,
    };
    if st != 0 { return false; }

    let count = (boot_order_size / 2) as usize;
    for i in 0..count {
        let boot_num = boot_order_buf[i];
        let hex_digits = b"0123456789ABCDEF";
        let mut name_buf: [u16; 9] = [0u16; 9];
        name_buf[0] = b'B' as u16;
        name_buf[1] = b'o' as u16;
        name_buf[2] = b'o' as u16;
        name_buf[3] = b't' as u16;
        name_buf[4] = hex_digits[((boot_num >> 12) & 0xF) as usize] as u16;
        name_buf[5] = hex_digits[((boot_num >> 8) & 0xF) as usize] as u16;
        name_buf[6] = hex_digits[((boot_num >> 4) & 0xF) as usize] as u16;
        name_buf[7] = hex_digits[(boot_num & 0xF) as usize] as u16;
        name_buf[8] = 0;

        let mut desc_buf: [u16; 128] = [0u16; 128];
        let mut desc_size: u64 = 256;
        let st2 = unsafe {
            gv.unwrap()(name_buf.as_mut_ptr(), global_guid, ptr::null_mut(), &mut desc_size, desc_buf.as_mut_ptr() as *mut c_void)
        };
        if st2 == 0 && desc_size > 4 {
            let desc_slice = &desc_buf[..64];
            let mut is_lumie = true;
            let lumie_name: [u16; 8] = [b'L' as u16, b'u' as u16, b'm' as u16, b'i' as u16, b'e' as u16, b'O' as u16, b'S' as u16, 0];
            for j in 0..8 {
                if desc_slice[j] != lumie_name[j] { is_lumie = false; break; }
            }
            if !is_lumie && desc_buf[0] != 0 {
                return true;
            }
        }
    }
    false
}

pub fn loader_install_screen() {
    unsafe {
        term_clear(0);
        term_set_bg(0);
        term_set_fg(0xFFFFFF);
        term_writeln(b"=== LumieOS Installer ===" as *const u8);
        term_writeln(b"" as *const u8);
    }

    let st_ptr = crate::input::get_ld_st();
    if st_ptr.is_null() {
        unsafe { term_writeln(b"ERROR: No UEFI system table." as *const u8); }
        return;
    }
    let st = unsafe { &*st_ptr };

    let mut install_devices: [crate::devices::LoaderBlockDevice; 16] =
        unsafe { core::mem::zeroed() };
    let dev_count = crate::devices::loader_enum_block_devices(unsafe { &*st.boot_services }, &mut install_devices);
    let target_device: i32;

    if dev_count == 0 {
        unsafe {
            term_writeln(b"ERROR: No block devices found." as *const u8);
            term_writeln(b"Press any key to return..." as *const u8);
            crate::input::loader_getchar();
        }
        return;
    }

    /* Show devices */
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
            let total_mb = (install_devices[i].block_count * install_devices[i].block_size as u64) / (1024 * 1024);
            let tag = b" (";
            for &c in tag { if bp < 127 { buf[bp] = c; bp += 1; } }
            let mut sz: [u8; 16] = [0u8; 16];
            lumie_std::format::lumie_itoa(total_mb as i64, sz.as_mut_ptr(), 10);
            for &c in sz.iter() { if c == 0 { break; } if bp < 127 { buf[bp] = c; bp += 1; } }
            let tag2 = b" MB)";
            for &c in tag2 { if bp < 127 { buf[bp] = c; bp += 1; } }
            buf[bp] = 0;
            term_writeln(buf.as_ptr());
        }
    }

    /* Select device (multi-digit support) */
    unsafe {
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
    }
    loop {
        let sel = read_number();
        if sel >= 1 && sel <= dev_count as u64 {
            target_device = (sel - 1) as i32;
            break;
        }
    }
    unsafe { term_writeln(b"" as *const u8); }

    /* Get partition size in GB */
    unsafe {
        term_write(b"Enter size in GB for LumieOS (default 50): " as *const u8);
    }
    let size_gb = read_number();
    let size_gb = if size_gb == 0 { 50 } else { size_gb };
    unsafe { term_writeln(b"" as *const u8); }

    let dev_handle = install_devices[target_device as usize].handle;
    let is_partition = install_devices[target_device as usize].is_partition != 0;

    /* Check if this is a whole disk (not a partition) */
    if !is_partition {
        unsafe {
            let mut msg: [u8; 128] = [0u8; 128];
            let mut mp = 0;
            let pre = b"Will create a ";
            for &c in pre { if mp < 127 { msg[mp] = c; mp += 1; } }
            let mut gb_str: [u8; 8] = [0u8; 8];
            lumie_std::format::lumie_itoa(size_gb as i64, gb_str.as_mut_ptr(), 10);
            for &c in gb_str.iter() { if c == 0 { break; } if mp < 127 { msg[mp] = c; mp += 1; } }
            let post = b" GB GPT partition on this disk.";
            for &c in post { if mp < 127 { msg[mp] = c; mp += 1; } }
            msg[mp] = 0;
            term_writeln(msg.as_ptr());
        }
    } else {
        unsafe { term_writeln(b"Selected device is already a partition." as *const u8); }
    }

    /* Check for other OS */
    let has_other_os = detect_other_os_in_boot_order();
    let register_uefi;
    if has_other_os {
        unsafe { term_writeln(b"Detected another OS in UEFI boot order." as *const u8); }
        register_uefi = loader_text_confirm(b"Register LumieOS in UEFI boot menu?" as *const u8);
    } else {
        register_uefi = loader_text_confirm(b"Register LumieOS in UEFI boot menu?" as *const u8);
    }

    if !loader_text_confirm(b"Format and install LumieOS on this device?" as *const u8) {
        unsafe { term_writeln(b"Installation cancelled." as *const u8); }
        return;
    }

    /* Prepare install.pkg from boot device */
    let mut pkg: [u8; 256] = [0u8; 256];
    let mut pkg_found = false;
    let boot_dev = crate::get_boot_device();

    if !boot_dev.is_null() {
        unsafe { fat_set_device(boot_dev); }
        if unsafe { install_pkg_open(b"install.pkg\0" as *const u8, pkg.as_mut_ptr() as *mut c_void) } == 0 {
            pkg_found = true;
        }
    }

    if !pkg_found {
        unsafe {
            term_writeln(b"ERROR: install.pkg not found on boot device." as *const u8);
            term_writeln(b"Press any key to return..." as *const u8);
            crate::input::loader_getchar();
        }
        return;
    }

    let part_start: u64;
    let part_sectors: u64;
    let total_disk_sectors: u64;

    /* Auto-detect filesystem */
    let mut use_ntfs;
    if is_partition {
        /* Try NTFS first, then FAT32 */
        let mut ok = false;
        use_ntfs = true;
        if unsafe { ntfs_set_device(dev_handle) } == 0 {
            unsafe { term_writeln(b"Detected NTFS filesystem." as *const u8); }
            ok = true;
        } else {
            use_ntfs = false;
            if unsafe { fat_set_device(dev_handle) } == 0 {
                unsafe { term_writeln(b"Detected FAT32 filesystem." as *const u8); }
                ok = true;
            }
        }
        if !ok {
            unsafe {
                term_writeln(b"ERROR: Unsupported filesystem on partition (not NTFS or FAT32)." as *const u8);
                term_writeln(b"Press any key to return..." as *const u8);
                crate::input::loader_getchar();
            }
            return;
        }
        let bio_guid = &EFI_BLOCK_IO_GUID as *const EfiGuid;
        let mut bio: *mut c_void = ptr::null_mut();
        let es2 = unsafe {
            if let Some(hp) = (*st.boot_services).handle_protocol {
                hp(dev_handle, bio_guid, &mut bio)
            } else { 1 }
        };
        if es2 == 0 && !bio.is_null() {
            let media_ptr = unsafe { *(bio as *mut *mut c_void).add(1) };
            if !media_ptr.is_null() {
                let last_block = unsafe { *(media_ptr as *mut u64).add(2) };
                total_disk_sectors = last_block + 1;
            } else { total_disk_sectors = 1024 * 1024; }
        } else { total_disk_sectors = 1024 * 1024; }
        part_start = 0;
        part_sectors = total_disk_sectors;
    } else {
        /* Whole disk — create GPT + FAT32 */
        use_ntfs = false;
        unsafe {
            term_writeln(b"Creating GPT partition table..." as *const u8);
        }
        match crate::gpt::create_gpt_partition(dev_handle, size_gb, false) {
            Some((start, sectors)) => {
                part_start = start;
                part_sectors = sectors;
                /* Signal the filesystem to use the partition directly by its handle */
                /* For now, we set up the device and format at offset */
                unsafe { fat_set_device(dev_handle); }
            }
            None => {
                unsafe {
                    term_writeln(b"ERROR: Failed to create GPT partition." as *const u8);
                    term_writeln(b"Press any key to return..." as *const u8);
                    crate::input::loader_getchar();
                }
                return;
            }
        }
    }

    /* Format */
    unsafe { term_writeln(b"Formatting..." as *const u8); }
    loader_console_install(b"Formatting..." as *const u8, 10);

    if use_ntfs {
        unsafe { term_writeln(b"NTFS already formatted, skipping format." as *const u8); }
    } else if is_partition || part_start == 0 {
        unsafe { fat_format(part_sectors); }
    } else {
        /* Format at partition offset — need to use raw block I/O */
        unsafe { fat_format_at(part_start, part_sectors); }
    }
    loader_console_install(b"Format done" as *const u8, 20);

    /* If we created a GPT partition, set up filesystem on the partition area */
    if !is_partition && part_start > 0 {
        if use_ntfs {
            unsafe { ntfs_set_device(dev_handle); }
        } else {
            unsafe { fat_set_device(dev_handle); }
        }
    }

    /* Directories */
    loader_console_install(b"Creating directories..." as *const u8, 30);
    if use_ntfs {
        if unsafe { ntfs_exists(b"/\0" as *const u8) } == 0 { /* Root exists */ }
        if unsafe { ntfs_exists(b"/system\0" as *const u8) } == 0 { unsafe { ntfs_mkdir(b"/system\0" as *const u8); } }
        if unsafe { ntfs_exists(b"/drivers\0" as *const u8) } == 0 { unsafe { ntfs_mkdir(b"/drivers\0" as *const u8); } }
    } else {
        if unsafe { fat_exists(b"/\0" as *const u8) } != 0 { /* Root exists */ }
        if unsafe { fat_exists(b"/system\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/system\0" as *const u8); } }
        if unsafe { fat_exists(b"/drivers\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/drivers\0" as *const u8); } }
        if unsafe { fat_exists(b"/EFI\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/EFI\0" as *const u8); } }
        if unsafe { fat_exists(b"/EFI/BOOT\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/EFI/BOOT\0" as *const u8); } }
    }

    /* Extract */
    loader_console_install(b"Extracting system files..." as *const u8, 50);
    unsafe {
        if use_ntfs {
            install_pkg_set_write_fn(Some(ntfs_write_file));
        } else {
            install_pkg_set_write_fn(Some(fat_write_file));
        }
        install_pkg_extract_all(pkg.as_mut_ptr() as *mut c_void, ptr::null_mut());
    }

    /* Bootloader */
    loader_console_install(b"Installing bootloader..." as *const u8, 75);
    unsafe { fat_install_bootloader(); }

    if register_uefi {
        loader_console_install(b"Registering UEFI boot entry..." as *const u8, 80);
        unsafe { lumie_efi_register_boot_entry(); }
    } else {
        loader_console_install(b"Skipping UEFI boot entry..." as *const u8, 80);
    }

    /* Timezone */
    loader_console_install(b"Setting timezone..." as *const u8, 90);
    unsafe {
        term_writeln(b"" as *const u8);
        term_writeln(b"Select timezone:" as *const u8);
        term_writeln(b"  1. Moscow (UTC+3)" as *const u8);
        term_writeln(b"  2. Krasnoyarsk (UTC+7)" as *const u8);
        term_write(b"Choice (1-2): " as *const u8);
    }
    let mut _tz_sel: i32 = 0;
    loop {
        let c = crate::input::loader_getchar();
        if c == b'1' as i32 { _tz_sel = 0; break; }
        if c == b'2' as i32 { _tz_sel = 1; break; }
    }
    unsafe { term_writeln(b"" as *const u8); }

    let tz_offsets: [i32; 2] = [180, 420];
    let mut tz_buf: [u8; 16] = [0u8; 16];
    unsafe {
        lumie_std::format::lumie_itoa(tz_offsets[_tz_sel as usize] as i64, tz_buf.as_mut_ptr(), 10);
        let mut len = 0;
        while len < 16 && tz_buf[len] != 0 { len += 1; }
        if use_ntfs {
            ntfs_write_file(b"/system/timezone.cfg\0" as *const u8, tz_buf.as_ptr() as *const c_void, (len + 1) as u32);
        } else {
            fat_write_file(b"/system/timezone.cfg\0" as *const u8, tz_buf.as_ptr() as *const c_void, (len + 1) as u32);
        }
        install_pkg_close(pkg.as_mut_ptr() as *mut c_void);
    }

    /* Write target config for the kernel */
    unsafe {
        let mut cfg: [u8; 64] = [0u8; 64];
        let mut cp = 0;
        let prefix = b"alloc_gb=";
        for &c in prefix { if cp < 63 { cfg[cp] = c; cp += 1; } }
        let mut gb_str: [u8; 8] = [0u8; 8];
        lumie_std::format::lumie_itoa(size_gb as i64, gb_str.as_mut_ptr(), 10);
        for &c in gb_str.iter() { if c == 0 { break; } if cp < 63 { cfg[cp] = c; cp += 1; } }
        cfg[cp] = 0;
        if use_ntfs {
            ntfs_write_file(b"/system/install.cfg\0" as *const u8, cfg.as_ptr() as *const c_void, cp as u32);
        } else {
            fat_write_file(b"/system/install.cfg\0" as *const u8, cfg.as_ptr() as *const c_void, cp as u32);
        }
    }

    /* Self-deletion: remove install.pkg from boot device */
    if !boot_dev.is_null() {
        unsafe { fat_set_device(boot_dev); }
        unsafe { fat_delete(b"install.pkg\0" as *const u8); }
    }

    loader_console_install(b"Installation complete!" as *const u8, 100);
    unsafe {
        term_writeln(b"" as *const u8);
        term_writeln(b"LumieOS installed successfully!" as *const u8);
        term_writeln(b"install.pkg has been removed." as *const u8);
        term_writeln(b"" as *const u8);
        term_writeln(b"Remove the installation media and press any key to reboot..." as *const u8);
        crate::input::loader_getchar();
    }

    unsafe { lumie_reboot(); }
}
