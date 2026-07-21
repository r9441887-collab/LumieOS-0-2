
use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;
use crate::ffi::*;

static mut G_PRESELECT_DEVICE: efi_handle = core::ptr::null_mut();
static mut G_PRESELECT_IS_PARTITION: i32 = -1;
static mut G_PRESELECT_IS_REMOVABLE: i32 = -1;

pub unsafe fn install_set_preselected_device(handle: efi_handle, is_partition: i32, is_removable: i32) {
    G_PRESELECT_DEVICE = handle;
    G_PRESELECT_IS_PARTITION = is_partition;
    G_PRESELECT_IS_REMOVABLE = is_removable;
}

unsafe extern "efiapi" fn gpt_progress_cb(msg: *const u8, pct: i32) {
    loader_console_install(msg, pct);
}

fn loader_console_install(phase: *const u8, pct: i32) {
    unsafe {
        term_set_fg(0x55FFFF);
        term_write(b"[\0" as *const u8);
        let bar_w = 40;
        let filled = (pct * bar_w) / 100;
        for i in 0..bar_w {
            if i < filled { term_write(b"#\0" as *const u8); }
            else { term_write(b".\0" as *const u8); }
        }
        term_write(b"] \0" as *const u8);
        let mut pc: [u8; 8] = [0u8; 8];
        lumie_std::format::lumie_itoa(pct as i64, pc.as_mut_ptr(), 10);
        let mut pi = 0;
        while pi < 8 && pc[pi] != 0 {
            let c = [pc[pi], 0u8];
            term_write(c.as_ptr());
            pi += 1;
        }
        term_write(b"% \0" as *const u8);
        term_set_fg(0xFFFFFF);
        if !phase.is_null() { term_writeln(phase); }
    }
}

fn loader_text_confirm(msg: *const u8) -> bool {
    unsafe {
        term_write(msg);
        term_write(b" (y/n): \0" as *const u8);
        loop {
            let c = crate::input::loader_getchar();
            if c == b'y' as i32 || c == b'Y' as i32 {
                term_writeln(b"y\0" as *const u8);
                return true;
            }
            if c == b'n' as i32 || c == b'N' as i32 {
                term_writeln(b"n\0" as *const u8);
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
        term_writeln(b"=== LumieOS Installer ===\0" as *const u8);
        term_writeln(b"\0" as *const u8);
    }

    let st_ptr = crate::input::get_ld_st();
    if st_ptr.is_null() {
        unsafe { term_writeln(b"ERROR: No UEFI system table.\0" as *const u8); }
        return;
    }
    let st = unsafe { &*st_ptr };

    #[allow(unused_assignments)]
    let mut dev_handle: efi_handle = core::ptr::null_mut();
    #[allow(unused_assignments)]
    let mut is_partition: bool = false;
    #[allow(unused_assignments)]
    let mut is_removable: bool = false;

    unsafe {
        if G_PRESELECT_IS_PARTITION >= 0 {
            dev_handle = G_PRESELECT_DEVICE;
            is_partition = G_PRESELECT_IS_PARTITION != 0;
            is_removable = G_PRESELECT_IS_REMOVABLE == 1;
            term_writeln(b"Using pre-selected device.\0" as *const u8);
            term_writeln(b"\0" as *const u8);
        } else {
            let mut install_devices: [crate::devices::LoaderBlockDevice; 16] =
                core::mem::zeroed();
            let dev_count = crate::devices::loader_enum_block_devices(&*st.boot_services, &mut install_devices, true);

            if dev_count == 0 {
                term_writeln(b"ERROR: No block devices found.\0" as *const u8);
                return;
            }

            term_writeln(b"Available devices:\0" as *const u8);
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
                let total_mb = install_devices[i].block_count / ((1024 * 1024) / install_devices[i].block_size as u64);
                let tag = b" (\0";
                for &c in tag { if bp < 127 { buf[bp] = c; bp += 1; } }
                let mut sz: [u8; 16] = [0u8; 16];
                lumie_std::format::lumie_itoa(total_mb as i64, sz.as_mut_ptr(), 10);
                for &c in sz.iter() { if c == 0 { break; } if bp < 127 { buf[bp] = c; bp += 1; } }
                let tag2 = b" MB)\0";
                for &c in tag2 { if bp < 127 { buf[bp] = c; bp += 1; } }
                buf[bp] = 0;
                term_writeln(buf.as_ptr());
            }

            term_write(b"Select device (1-\0" as *const u8);
            let mut num: [u8; 8] = [0u8; 8];
            lumie_std::format::lumie_itoa(dev_count as i64, num.as_mut_ptr(), 10);
            let mut ni = 0;
            while ni < 8 && num[ni] != 0 {
                let c = [num[ni], 0u8];
                term_write(c.as_ptr());
                ni += 1;
            }
            term_write(b"): \0" as *const u8);

            let target_device;
            loop {
                let sel = read_number();
                if sel >= 1 && sel <= dev_count as u64 {
                    target_device = (sel - 1) as i32;
                    break;
                }
            }
            term_writeln(b"\0" as *const u8);

            dev_handle = install_devices[target_device as usize].handle;
            is_partition = install_devices[target_device as usize].is_partition != 0;
            is_removable = install_devices[target_device as usize].is_removable != 0;
        }
    }

    /* Get partition size in GB */
    unsafe {
        term_write(b"Enter size in GB for LumieOS (default 50): \0" as *const u8);
    }
    let size_gb = read_number();
    let size_gb = if size_gb == 0 { 50 } else { size_gb };
    unsafe { term_writeln(b"\0" as *const u8); }

    /* Check if this is a whole disk (not a partition) */
    if !is_partition {
        unsafe {
            let mut msg: [u8; 128] = [0u8; 128];
            let mut mp = 0;
            let pre = b"Will create: ESP (FAT32, 260 MB) + LumFS (\0";
            for &c in pre { if c == 0 { break; } if mp < 127 { msg[mp] = c; mp += 1; } }
            let mut gb_str: [u8; 8] = [0u8; 8];
            lumie_std::format::lumie_itoa(size_gb as i64, gb_str.as_mut_ptr(), 10);
            for &c in gb_str.iter() { if c == 0 { break; } if mp < 127 { msg[mp] = c; mp += 1; } }
            let post = b" GB) on this disk.\0";
            for &c in post { if c == 0 { break; } if mp < 127 { msg[mp] = c; mp += 1; } }
            msg[mp] = 0;
            term_writeln(msg.as_ptr());
        }
    } else {
        unsafe { term_writeln(b"Selected device is already a partition.\0" as *const u8); }
    }

    /* Check for other OS */
    let has_other_os = crate::boot::detect_other_os();
    let register_uefi;
    if has_other_os {
        unsafe {
            term_writeln(b"\0" as *const u8);
            term_writeln(b"=== UEFI Boot Menu ===\0" as *const u8);
            term_writeln(b"Detected another OS in UEFI boot order.\0" as *const u8);
            term_writeln(b"LumieOS will be added AFTER existing entries (not at position 1).\0" as *const u8);
        }
        register_uefi = loader_text_confirm(b"Register LumieOS in UEFI boot menu?\0" as *const u8);
    } else {
        register_uefi = loader_text_confirm(b"Register LumieOS in UEFI boot menu?\0" as *const u8);
    }

    unsafe {
        term_writeln(b"\0" as *const u8);
        term_writeln(b"=== Installation ===\0" as *const u8);
    }
    if !loader_text_confirm(b"Format and install LumieOS on this device?\0" as *const u8) {
        unsafe { term_writeln(b"Installation cancelled.\0" as *const u8); }
        return;
    }

    /* Prepare install.pkg from boot device */
    let mut pkg: [u8; 256] = [0u8; 256];
    let mut pkg_found = false;
    let boot_dev = crate::get_boot_device();

    if !boot_dev.is_null() {
        unsafe { fat_set_partition_offset(0); }
        let set_dev_rc = unsafe { fat_set_device(boot_dev) };
        if set_dev_rc != 0 {
            unsafe {
                term_set_fg(0xFF0000);
                let mut msg: [u8; 80] = [0u8; 80];
                let pfx = b"ERROR: fat_set_device returned \0";
                let mut mp = 0;
                for &c in pfx { if mp < 79 { msg[mp] = c; mp += 1; } }
                let mut num: [u8; 8] = [0u8; 8];
                lumie_std::format::lumie_itoa(set_dev_rc as i64, num.as_mut_ptr(), 10);
                for &c in num.iter() { if c == 0 { break; } if mp < 79 { msg[mp] = c; mp += 1; } }
                msg[mp] = 0;
                term_writeln(msg.as_ptr());
                term_set_fg(0xFFFFFF);
            }
            return;
        }
        let open_rc = unsafe { install_pkg_open(b"install.pkg\0" as *const u8, pkg.as_mut_ptr() as *mut c_void) };
        if open_rc == 0 {
            pkg_found = true;
        } else {
            unsafe {
                term_set_fg(0xFF0000);
                let mut msg: [u8; 80] = [0u8; 80];
                let pfx = b"ERROR: install_pkg_open returned \0";
                let mut mp = 0;
                for &c in pfx { if mp < 79 { msg[mp] = c; mp += 1; } }
                let mut num: [u8; 8] = [0u8; 8];
                lumie_std::format::lumie_itoa(open_rc as i64, num.as_mut_ptr(), 10);
                for &c in num.iter() { if c == 0 { break; } if mp < 79 { msg[mp] = c; mp += 1; } }
                msg[mp] = 0;
                term_writeln(msg.as_ptr());
                term_set_fg(0xFFFFFF);
            }
        }
    } else {
        unsafe {
            term_set_fg(0xFF0000);
            term_writeln(b"ERROR: boot device handle is NULL.\0" as *const u8);
            term_writeln(b"Cannot find installation media. Make sure install.pkg exists.\0" as *const u8);
            term_set_fg(0xFFFFFF);
        }
    }

    if !pkg_found {
        unsafe {
            term_set_fg(0xFF0000);
            term_writeln(b"ERROR: install.pkg not found on boot device.\0" as *const u8);
            term_writeln(b"Make sure install.pkg is in the root of the boot partition.\0" as *const u8);
            term_set_fg(0xFFFFFF);
        }
        return;
    }

    let part_start: u64;
    let part_sectors: u64;
    let esp_part_start: u64;
    let esp_part_sectors: u64;
    let total_disk_sectors: u64;

    /* Auto-detect filesystem */
    let mut use_ntfs;
    let mut use_lumfs;
    if is_partition {
        /* Try LumFS first, then NTFS, then FAT32 */
        use_lumfs = true;
        use_ntfs = false;
        if unsafe { crate::ffi::lumfs_set_device(dev_handle) } == 0 {
            unsafe { term_writeln(b"Detected LumFS filesystem.\0" as *const u8); }
        } else {
            use_lumfs = false;
            use_ntfs = true;
            if unsafe { ntfs_set_device(dev_handle) } == 0 {
                unsafe { term_writeln(b"Detected NTFS filesystem.\0" as *const u8); }
            } else {
                use_ntfs = false;
                if unsafe { fat_set_device(dev_handle) } == 0 {
                    unsafe { term_writeln(b"Detected FAT32 filesystem.\0" as *const u8); }
                } else {
                    unsafe { term_writeln(b"No filesystem detected, will format as LumFS.\0" as *const u8); }
                }
            }
        }
        let bio_guid = &EFI_BLOCK_IO_GUID as *const EfiGuid;
        let mut bio: *mut crate::uefi::EfiBlockIoProtocol = ptr::null_mut();
        let es2 = unsafe {
            if let Some(hp) = (*st.boot_services).handle_protocol {
                hp(dev_handle, bio_guid, &mut bio as *mut *mut crate::uefi::EfiBlockIoProtocol as *mut *mut c_void)
            } else { 1 }
        };
        if es2 == 0 && !bio.is_null() {
            let media = unsafe { (*bio).media };
            if !media.is_null() {
                let last_block = unsafe { (*media).last_block };
                total_disk_sectors = last_block + 1;
            } else { total_disk_sectors = 1024 * 1024; }
        } else { total_disk_sectors = 1024 * 1024; }
        part_start = 0;
        part_sectors = total_disk_sectors;
        esp_part_start = 0;
        esp_part_sectors = 0;
    } else {
        /* Whole disk — create GPT with dual partitions (ESP + LumFS) */
        use_ntfs = false;
        use_lumfs = true;

        if is_removable {
            unsafe {
                term_set_fg(0xFFFF00);
                term_writeln(b"WARNING: Selected device is removable media (USB flash drive).\0" as *const u8);
                term_writeln(b"All data on this device will be PERMANENTLY erased!\0" as *const u8);
                term_set_fg(0xFFFFFF);
            }
            if !loader_text_confirm(b"Continue with removable device?\0" as *const u8) {
                unsafe { term_writeln(b"Installation cancelled.\0" as *const u8); }
                unsafe { install_pkg_close(pkg.as_mut_ptr() as *mut c_void); }
                return;
            }
        }

        unsafe {
            term_writeln(b"Checking write access...\0" as *const u8);
        }
        let write_check = crate::gpt::check_writable(dev_handle);
        if let Err(e) = write_check {
            unsafe {
                term_set_fg(0xFF0000);
                let mut msg: [u8; 96] = [0u8; 96];
                let pfx = b"ERROR: \0";
                let mut mp = 0;
                for &c in pfx { if mp < 95 { msg[mp] = c; mp += 1; } }
                for &c in e.as_bytes() { if mp < 95 { msg[mp] = c; mp += 1; } }
                msg[mp] = 0;
                term_writeln(msg.as_ptr());
                term_writeln(b"Cannot write to this device. Check write-protect switch or permissions.\0" as *const u8);
                term_set_fg(0xFFFFFF);
            }
            unsafe { install_pkg_close(pkg.as_mut_ptr() as *mut c_void); }
            return;
        }
        unsafe {
            term_writeln(b"Write access OK.\0" as *const u8);
            term_writeln(b"\0" as *const u8);
            term_writeln(b"Creating GPT with ESP (FAT32) + LumFS partitions...\0" as *const u8);
        }

        /* Create dual partitions: ESP (260 MB FAT32) + LumFS (system) */
        let dual_result = crate::gpt::create_gpt_dual_partitions(dev_handle, size_gb, 260, Some(gpt_progress_cb));
        match dual_result {
            Some(ref result) => {
                part_start = result.lumfs_start;
                part_sectors = result.lumfs_sectors;
                esp_part_start = result.esp_start;
                esp_part_sectors = result.esp_sectors;
                unsafe {
                    term_set_fg(0x00CC00);
                    term_writeln(b"GPT partition table created successfully.\0" as *const u8);
                    let mut msg: [u8; 80] = [0u8; 80];
                    let mut mp = 0;
                    for &c in b"  ESP (FAT32): LBA \0" { if c == 0 { break; } if mp < 79 { msg[mp] = c; mp += 1; } }
                    let mut num: [u8; 16] = [0u8; 16];
                    lumie_std::format::lumie_itoa(esp_part_start as i64, num.as_mut_ptr(), 10);
                    for &c in num.iter() { if c == 0 { break; } if mp < 79 { msg[mp] = c; mp += 1; } }
                    for &c in b", \0" { if c == 0 { break; } if mp < 79 { msg[mp] = c; mp += 1; } }
                    let mut mb: [u8; 8] = [0u8; 8];
                    lumie_std::format::lumie_itoa((esp_part_sectors * 512 / 1048576) as i64, mb.as_mut_ptr(), 10);
                    for &c in mb.iter() { if c == 0 { break; } if mp < 79 { msg[mp] = c; mp += 1; } }
                    for &c in b" MB\0" { if c == 0 { break; } if mp < 79 { msg[mp] = c; mp += 1; } }
                    msg[mp] = 0;
                    term_writeln(msg.as_ptr());

                    let mut msg2: [u8; 80] = [0u8; 80];
                    let mut mp2 = 0;
                    for &c in b"  LumFS: LBA \0" { if c == 0 { break; } if mp2 < 79 { msg2[mp2] = c; mp2 += 1; } }
                    let mut num2: [u8; 16] = [0u8; 16];
                    lumie_std::format::lumie_itoa(part_start as i64, num2.as_mut_ptr(), 10);
                    for &c in num2.iter() { if c == 0 { break; } if mp2 < 79 { msg2[mp2] = c; mp2 += 1; } }
                    for &c in b", \0" { if c == 0 { break; } if mp2 < 79 { msg2[mp2] = c; mp2 += 1; } }
                    let mut gb2: [u8; 8] = [0u8; 8];
                    lumie_std::format::lumie_itoa(size_gb as i64, gb2.as_mut_ptr(), 10);
                    for &c in gb2.iter() { if c == 0 { break; } if mp2 < 79 { msg2[mp2] = c; mp2 += 1; } }
                    for &c in b" GB\0" { if c == 0 { break; } if mp2 < 79 { msg2[mp2] = c; mp2 += 1; } }
                    msg2[mp2] = 0;
                    term_writeln(msg2.as_ptr());
                    term_set_fg(0xFFFFFF);
                }
            }
            None => {
                unsafe {
                    term_set_fg(0xFF0000);
                    term_writeln(b"ERROR: Failed to create GPT partitions.\0" as *const u8);
                    term_writeln(b"The device may be too small, slow, or write-protected.\0" as *const u8);
                    term_writeln(b"Minimum required: ~300 MB for ESP + space for LumFS.\0" as *const u8);
                    term_set_fg(0xFFFFFF);
                }
                unsafe { install_pkg_close(pkg.as_mut_ptr() as *mut c_void); }
                return;
            }
        }

        /* Format ESP as FAT32 */
        unsafe {
            term_writeln(b"\0" as *const u8);
            term_writeln(b"Formatting ESP as FAT32...\0" as *const u8);
            fat_set_partition_offset(esp_part_start);
            fat_format_at(0, esp_part_sectors);
            if fat_exists(b"/\0" as *const u8) == 0 {
                term_set_fg(0xFF0000);
                term_writeln(b"ERROR: ESP format failed - FAT32 not accessible.\0" as *const u8);
                term_writeln(b"The partition may be corrupted or too small.\0" as *const u8);
                term_set_fg(0xFFFFFF);
                install_pkg_close(pkg.as_mut_ptr() as *mut c_void);
                return;
            }
            fat_mkdir(b"/EFI\0" as *const u8);
            fat_mkdir(b"/EFI/BOOT\0" as *const u8);
            term_set_fg(0x00CC00);
            term_writeln(b"ESP formatted successfully.\0" as *const u8);
            term_set_fg(0xFFFFFF);
        }

        /* Copy bootloader to ESP */
        unsafe {
            term_writeln(b"Installing bootloader to ESP...\0" as *const u8);
            fat_install_bootloader(dev_handle, esp_part_start);
        }

        /* Initialize LumFS on system partition */
        unsafe {
            term_writeln(b"Initializing LumFS on system partition...\0" as *const u8);
            fat_set_partition_offset(part_start);
            if crate::ffi::lumfs_set_device(dev_handle) != 0 {
                term_set_fg(0xFF0000);
                term_writeln(b"ERROR: LumFS initialization failed.\0" as *const u8);
                term_writeln(b"The system partition may be corrupted.\0" as *const u8);
                term_set_fg(0xFFFFFF);
                install_pkg_close(pkg.as_mut_ptr() as *mut c_void);
                return;
            }
        }
    }

    /* Format */
    unsafe { term_writeln(b"Formatting system partition...\0" as *const u8); }
    loader_console_install(b"Formatting...\0" as *const u8, 10);

    if use_ntfs {
        unsafe { term_writeln(b"NTFS already formatted, skipping format.\0" as *const u8); }
    } else if use_lumfs && (is_partition || part_start != 0) {
        unsafe { crate::ffi::lumfs_format_at(0, part_sectors); }
    } else if use_lumfs {
        unsafe { crate::ffi::lumfs_format_at(part_start, part_sectors); }
    } else if is_partition || part_start == 0 {
        unsafe { fat_format(part_sectors); }
    } else {
        unsafe { fat_format_at(0, part_sectors); }
    }
    loader_console_install(b"Format done\0" as *const u8, 20);

    /* Directories */
    loader_console_install(b"Creating directories...\0" as *const u8, 30);
    if use_ntfs {
        if unsafe { ntfs_exists(b"/\0" as *const u8) } == 0 { /* Root exists */ }
        if unsafe { ntfs_exists(b"/system\0" as *const u8) } == 0 { unsafe { ntfs_mkdir(b"/system\0" as *const u8); } }
        if unsafe { ntfs_exists(b"/drivers\0" as *const u8) } == 0 { unsafe { ntfs_mkdir(b"/drivers\0" as *const u8); } }
    } else {
        if unsafe { fat_exists(b"/\0" as *const u8) } != 0 { /* Root exists */ }
        if unsafe { fat_exists(b"/system\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/system\0" as *const u8); } }
        if unsafe { fat_exists(b"/drivers\0" as *const u8) } == 0 { unsafe { fat_mkdir(b"/drivers\0" as *const u8); } }
    }

    /* Extract */
    loader_console_install(b"Extracting system files...\0" as *const u8, 50);
    let mut install_ok = true;
    unsafe {
        if use_ntfs {
            install_pkg_set_write_fn(Some(ntfs_write_file));
        } else {
            install_pkg_set_write_fn(Some(fat_write_file));
        }
        if install_pkg_extract_all(pkg.as_mut_ptr() as *mut c_void, ptr::null_mut()) != 0 {
            install_ok = false;
        }
    }

    if !install_ok {
        unsafe {
            term_set_fg(0xFF0000);
            term_writeln(b"ERROR: Failed to extract system files.\0" as *const u8);
            term_writeln(b"The install.pkg may be corrupted.\0" as *const u8);
            term_set_fg(0xFFFFFF);
        }
    }

    /* Register UEFI boot entry */
    if register_uefi {
        loader_console_install(b"Registering UEFI boot entry...\0" as *const u8, 80);
        let reg_rc = if !is_partition && esp_part_start != 0 {
            /* Dual partition: register boot entry pointing to ESP */
            unsafe { lumie_efi_register_boot_entry_for_target(dev_handle, esp_part_start, esp_part_sectors) }
        } else {
            unsafe { lumie_efi_register_boot_entry_for_target(dev_handle, part_start, part_sectors) }
        };
        if reg_rc != 0 {
            unsafe {
                term_set_fg(0xFF0000);
                term_set_bg(0);
                term_writeln(b"\0" as *const u8);
                term_writeln(b"WARNING: UEFI boot registration failed.\0" as *const u8);
                term_writeln(b"Press F11 (or Esc/F12) during boot and select LumieOS manually.\0" as *const u8);
                term_set_fg(0xFFFFFF);
            }
        } else {
            unsafe {
                term_set_fg(0x00CC00);
                term_writeln(b"UEFI boot entry registered successfully.\0" as *const u8);
                if has_other_os {
                    term_writeln(b"LumieOS added after existing boot entries.\0" as *const u8);
                } else {
                    term_writeln(b"LumieOS set as default boot option.\0" as *const u8);
                }
                term_set_fg(0xFFFFFF);
            }
        }
    } else {
        loader_console_install(b"Skipping UEFI boot entry...\0" as *const u8, 80);
        unsafe {
            term_writeln(b"\0" as *const u8);
            term_set_fg(0xFFFF00);
            term_writeln(b"LumieOS NOT registered in UEFI boot menu.\0" as *const u8);
            term_writeln(b"Press F11 (or Esc/F12) during boot and select LumieOS manually.\0" as *const u8);
            term_set_fg(0xFFFFFF);
        }
    }

    /* Timezone */
    loader_console_install(b"Setting timezone...\0" as *const u8, 90);
    unsafe {
        term_writeln(b"\0" as *const u8);
        term_writeln(b"Select timezone:\0" as *const u8);
        term_writeln(b"  1. Moscow (UTC+3)\0" as *const u8);
        term_writeln(b"  2. Krasnoyarsk (UTC+7)\0" as *const u8);
        term_write(b"Choice (1-2): \0" as *const u8);
    }
    let mut _tz_sel: i32 = 0;
    loop {
        let c = crate::input::loader_getchar();
        if c == b'1' as i32 { _tz_sel = 0; break; }
        if c == b'2' as i32 { _tz_sel = 1; break; }
    }
    unsafe { term_writeln(b"\0" as *const u8); }

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
        let prefix = b"alloc_gb=\0";
        for &c in prefix { if c == 0 { break; } if cp < 63 { cfg[cp] = c; cp += 1; } }
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

    /* Self-deletion: remove install.pkg from boot device (only on success) */
    if install_ok && !boot_dev.is_null() {
        unsafe { fat_set_partition_offset(0); }
        unsafe { fat_set_device(boot_dev); }
        unsafe { fat_delete(b"install.pkg\0" as *const u8); }
    }

    loader_console_install(b"Installation complete!\0" as *const u8, 100);
    unsafe {
        term_writeln(b"\0" as *const u8);
        term_set_fg(0x00CC00);
        term_writeln(b"LumieOS installed successfully!\0" as *const u8);
        term_set_fg(0xFFFFFF);
        if !is_partition && esp_part_start != 0 {
            term_writeln(b"  ESP (FAT32): Bootloader installed\0" as *const u8);
            term_writeln(b"  LumFS: System files installed\0" as *const u8);
        }
        term_writeln(b"install.pkg has been removed.\0" as *const u8);
        term_writeln(b"\0" as *const u8);
        term_writeln(b"Remove the installation media and press any key to reboot...\0" as *const u8);
        crate::input::loader_getchar();
    }

    unsafe { lumie_reboot(); }
}
