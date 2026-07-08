use crate::Shell;

pub fn do_login(sh: &mut Shell) {
    let term_bg = if sh.current_drive == b'A' { 0 } else { 1 };
    sh.svc.term_clear(term_bg);
    sh.svc.term_set_bg(term_bg);
    sh.svc.term_set_fg(15);

    /* Read target.cfg and switch to target disk */
    {
        let mut cfg = [0u8; 256];
        let n = sh.svc.fs_read_file("/system/target.cfg", &mut cfg);
        if n > 0 {
            let cfg_str = core::str::from_utf8(&cfg[..n as usize]).unwrap_or("").trim_end_matches('\0');
            let mut lines = cfg_str.splitn(3, '\n');
            let _type = lines.next().unwrap_or("").trim();
            let _name = lines.next().unwrap_or("").trim();
            let _ = lines.next();

            let count = sh.svc.disk_enum_all();
            let mut found_target = false;
            for i in 0..count {
                let mut info = crate::DiskInfo::default();
                if sh.svc.disk_get_info(i, &mut info) != 0 || !info.present {
                    continue;
                }
                if _type == "ahci" && info.is_ahci {
                    sh.svc.fs_use_ahci();
                    sh.current_drive = b'C';
                    found_target = true;
                    break;
                }
            }
            if !found_target {
                if _type == "blkio" {
                    sh.current_drive = b'A';
                } else if sh.svc.ahci_is_ready() {
                    sh.svc.fs_use_ahci();
                    sh.current_drive = b'C';
                }
            }
            sh.svc.fs_delete("/system/target.cfg");
        }
    }

    /* Ensure we have a valid fallback */
    if sh.current_drive != b'A' && sh.current_drive != b'C' {
        sh.current_drive = if sh.svc.ahci_is_ready() { b'C' } else { b'A' };
    }

    /* Set up drive */
    let term_bg2 = if sh.current_drive == b'A' { 0 } else { 1 };
    sh.svc.term_clear(term_bg2);
    sh.svc.term_set_bg(term_bg2);
    sh.svc.term_set_fg(15);

    if sh.current_drive == b'A' {
        sh.try_set_drive(b'A');
    } else {
        sh.svc.users_init();
        /* Auto-login admin if keyboard is dead */
        let mut _dead_kbd_count: u32 = 0;
        while !sh.svc.users_is_logged_in() {
            sh.svc.term_set_fg(11);
            sh.svc.term_write("LumieOS ");
            sh.svc.term_set_fg(15);
            sh.svc.term_writeln("Login");
            sh.svc.term_set_fg(11);
            sh.svc.term_write("Username: ");
            sh.svc.term_set_fg(15);

            let mut login_name = [0u8; 64];
            let mut p = 0;
            loop {
                let c = sh.svc.kbd_getchar();
                if c == b'\n' as i32 {
                    break;
                }
                if c == -1 {
                    _dead_kbd_count += 1;
                    if _dead_kbd_count > 50000 {
                        sh.svc.users_login("admin", Some("admin"));
                        sh.svc.term_writeln("admin (auto)");
                        break;
                    }
                    continue;
                }
                _dead_kbd_count = 0;
                if c == b'\x08' as i32 {
                    if p > 0 {
                        p -= 1;
                        sh.svc.term_write("\x08 \x08");
                    }
                } else if c >= b' ' as i32 && c <= b'~' as i32 && p < 63 {
                    login_name[p] = c as u8;
                    p += 1;
                    sh.svc.term_putchar(c as u8);
                }
            }
            sh.svc.term_writeln("");
            if p > 0 {
                let name_str = core::str::from_utf8(&login_name[..p]).unwrap_or("");
                let uid = sh.svc.users_login(name_str, None);
                if uid < 0 {
                    sh.svc.term_set_fg(12);
                    sh.svc.term_writeln("Unknown user. Try again.");
                    sh.svc.term_set_fg(15);
                }
            }
        }
    }
}
