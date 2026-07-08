use crate::console::terminal;
use crate::drivers::keyboard;
use crate::fs;
use crate::system::disk_io;
use crate::system::install_pkg;
use crate::drivers::ahci;

unsafe fn setup_progress(phase: *const u8, pct: i32) {
    terminal::term_set_fg(0x55FFFF);
    terminal::term_write(b"[\0" as *const u8);
    let bar_w = 40;
    let filled = (pct * bar_w) / 100;
    for i in 0..bar_w {
        if i < filled {
            terminal::term_putchar(b'#');
        } else {
            terminal::term_putchar(b'.');
        }
    }
    terminal::term_write(b"] \0" as *const u8);
    let mut pc: [u8; 8] = [0u8; 8];
    crate::system::util::lumie_itoa(pct as i64, pc.as_mut_ptr(), 10);
    terminal::term_write(pc.as_ptr());
    terminal::term_write(b"% \0" as *const u8);
    terminal::term_set_fg(0xFFFFFF);
    if !phase.is_null() {
        terminal::term_write(phase);
        terminal::term_putchar(b'\n');
    }
}

unsafe fn confirm(msg: *const u8) -> bool {
    terminal::term_write(msg);
    terminal::term_write(b" (y/n): \0" as *const u8);
    loop {
        let c = keyboard::getchar();
        if c == b'y' as i32 || c == b'Y' as i32 {
            terminal::term_writeln(b"y\0" as *const u8);
            return true;
        }
        if c == b'n' as i32 || c == b'N' as i32 {
            terminal::term_writeln(b"n\0" as *const u8);
            return false;
        }
    }
}

pub unsafe fn setup_run() -> i32 {
    let bg: u32 = 0x000000;
    terminal::term_clear(bg);
    terminal::term_set_bg(bg);
    terminal::term_set_fg(0xFFFFFF);

    terminal::term_writeln(b"=== LumieOS Console Installer ===\0" as *const u8);
    terminal::term_putchar(b'\n');

    let disk_count = disk_io::disk_enum_all();
    if disk_count == 0 {
        terminal::term_writeln(b"ERROR: No disks found.\0" as *const u8);
        terminal::term_writeln(b"Press any key...\0" as *const u8);
        keyboard::getchar();
        return -1;
    }

    terminal::term_writeln(b"Available disks:\0" as *const u8);
    for i in 0..disk_count {
        let d = disk_io::disk_get_info(i);
        if d.is_null() || !(*d).present {
            continue;
        }
        let info = &*d;
        let mut buf: [u8; 128] = [0u8; 128];
        let mut pos: usize = 0;
        crate::system::util::lumie_itoa((i + 1) as i64, buf[pos..].as_mut_ptr(), 10);
        while buf[pos] != 0 {
            pos += 1;
        }
        buf[pos] = b'.';
        pos += 1;
        buf[pos] = b' ';
        pos += 1;
        let name_len = crate::system::util::lumie_strlen_raw(&info.name);
        buf[pos..pos + name_len].copy_from_slice(&info.name[..name_len]);
        pos += name_len;
        let total_b = info.sector_count * (info.sector_size as u64);
        buf[pos] = b' ';
        pos += 1;
        buf[pos] = b'(';
        pos += 1;
        if total_b >= (1024 * 1024 * 1024) {
            crate::system::util::lumie_itoa((total_b / (1024 * 1024 * 1024)) as i64, buf[pos..].as_mut_ptr(), 10);
            while buf[pos] != 0 { pos += 1; }
            let gb = b" GB)";
            buf[pos..pos + 4].copy_from_slice(gb);
            pos += 4;
        } else {
            crate::system::util::lumie_itoa((total_b / (1024 * 1024)) as i64, buf[pos..].as_mut_ptr(), 10);
            while buf[pos] != 0 { pos += 1; }
            let mb = b" MB)";
            buf[pos..pos + 5].copy_from_slice(mb);
            pos += 5;
        }
        if info.is_removable {
            let rem = b" [Removable]";
            buf[pos..pos + 12].copy_from_slice(rem);
            pos += 12;
        }
        buf[pos] = 0;
        terminal::term_writeln(buf.as_ptr());
    }

    let mut _selected: i32 = -1;
    terminal::term_write(b"Select disk (1-" as *const u8);
    let mut num: [u8; 8] = [0u8; 8];
    crate::system::util::lumie_itoa(disk_count as i64, num.as_mut_ptr(), 10);
    terminal::term_write(num.as_ptr());
    terminal::term_write(b"): " as *const u8);
    loop {
        let c = keyboard::getchar();
        if c >= b'1' as i32 && c <= b'0' as i32 + disk_count {
            _selected = c - b'1' as i32;
            terminal::term_putchar(b'\n');
            break;
        }
    }

    if !confirm(b"Format and install LumieOS on this disk?\0" as *const u8) {
        terminal::term_writeln(b"Installation cancelled.\0" as *const u8);
        return 0;
    }

    let mut pkg: install_pkg::InstallPkg = core::mem::zeroed();
    let pkg_ret = install_pkg::install_pkg_open(b"install.pkg\0" as *const u8, &mut pkg as *mut _ as *mut core::ffi::c_void);
    if pkg_ret != 0 {
        terminal::term_writeln(b"ERROR: install.pkg not found on current drive.\0" as *const u8);
        terminal::term_writeln(b"Place install.pkg on the boot drive and try again.\0" as *const u8);
        keyboard::getchar();
        return -1;
    }

    let info = &*disk_io::disk_get_info(_selected);
    if !info.present {
        terminal::term_writeln(b"ERROR: Invalid disk selection.\0" as *const u8);
        install_pkg::install_pkg_close(&mut pkg as *mut _ as *mut core::ffi::c_void);
        return -1;
    }

    setup_progress(b"Formatting disk...\0" as *const u8, 5);
    {
        let mut total_sectors: u64 = 0;
        if ahci::is_ready() != 0 {
            total_sectors = ahci::get_sector_count();
            fs::use_ahci();
        }
        if total_sectors == 0 {
            total_sectors = 1024 * 1024;
        }
        fs::format(total_sectors);
        fs::reinit();
        if ahci::is_ready() != 0 {
            fs::use_ahci();
        }
    }
    setup_progress(b"Format complete\0" as *const u8, 15);

    setup_progress(b"Creating directories...\0" as *const u8, 20);
    if !fs::exists(b"/system\0" as *const u8) {
        fs::mkdir(b"/system\0" as *const u8);
    }
    if !fs::exists(b"/drivers\0" as *const u8) {
        fs::mkdir(b"/drivers\0" as *const u8);
    }

    setup_progress(b"Extracting files from install.pkg...\0" as *const u8, 30);
    install_pkg::install_pkg_extract_all(&mut pkg as *mut _ as *mut core::ffi::c_void, core::ptr::null_mut());

    // Bootloader installation is handled by loader; skipped here.

    setup_progress(b"Setting timezone...\0" as *const u8, 90);
    terminal::term_putchar(b'\n');
    terminal::term_writeln(b"Select timezone:\0" as *const u8);
    terminal::term_writeln(b"  1. Moscow (UTC+3)\0" as *const u8);
    terminal::term_writeln(b"  2. Krasnoyarsk (UTC+7)\0" as *const u8);
    terminal::term_write(b"Choice (1-2): \0" as *const u8);
    let mut _tz_sel: i32 = 0;
    loop {
        let c = keyboard::getchar();
        if c == b'1' as i32 { _tz_sel = 0; break; }
        if c == b'2' as i32 { _tz_sel = 1; break; }
    }
    terminal::term_putchar(b'\n');
    let tz_offsets = [180, 420];
    let mut tz_buf: [u8; 16] = [0u8; 16];
    crate::system::util::lumie_itoa(tz_offsets[_tz_sel as usize] as i64, tz_buf.as_mut_ptr(), 10);
    let tz_len = crate::system::util::lumie_strlen_raw(&tz_buf);
    tz_buf[tz_len] = 0;
    fs::write_file(
        b"/system/timezone.cfg\0" as *const u8,
        tz_buf.as_ptr(),
        tz_len as u32 + 1,
    );

    install_pkg::install_pkg_close(&mut pkg as *mut _ as *mut core::ffi::c_void);

    setup_progress(b"Installation complete!\0" as *const u8, 100);
    terminal::term_putchar(b'\n');
    terminal::term_writeln(b"LumieOS has been installed successfully!\0" as *const u8);
    terminal::term_writeln(b"Press any key to reboot...\0" as *const u8);
    keyboard::getchar();
    crate::lumie_reboot();

    0
}
