use crate::{
    parse_int,
    Shell,
    USER_ROLE_ADMIN,
    USER_ROLE_USER,
};

pub fn cmd_help(sh: &Shell) {
    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("LumieOS (Windows Edition) Commands:");
    sh.svc.term_set_fg(15);
    sh.svc.term_writeln("  help            - Show this help");
    sh.svc.term_writeln("  clear/cls       - Clear screen");
    sh.svc.term_writeln("  ls/dir [path]   - List directory contents");
    sh.svc.term_writeln("  cd <path>       - Change directory");
    sh.svc.term_writeln("  pwd             - Print working directory");
    sh.svc.term_writeln("  cat/type <file> - Display file contents");
    sh.svc.term_writeln("  edit <file>     - Open text editor");
    sh.svc.term_writeln("  notepad         - Open Notepad (text editor)");
    sh.svc.term_writeln("  echo <text>     - Print text");
    sh.svc.term_writeln("  rm/del <file>   - Delete file");
    sh.svc.term_writeln("  rmdir <dir>     - Delete empty directory");
    sh.svc.term_writeln("  mkdir <dir>     - Create directory");
    sh.svc.term_writeln("  ps              - List running processes");
    sh.svc.term_writeln("  info            - System information");
    sh.svc.term_writeln("  ver             - OS version");
    sh.svc.term_writeln("  reboot          - Restart system");
    sh.svc.term_writeln("  shutdown        - Power off");
    sh.svc.term_writeln("  wher <dir> <pat> - Find files recursively");
    sh.svc.term_writeln("  wher1 <pat>      - Find everywhere from root");
    sh.svc.term_writeln("  renet <name>    - Download via HTTP");
    sh.svc.term_writeln("  extract <file>  - Decompress tar.gz/tar.xz");
    sh.svc.term_writeln("  time            - Show current time");
    sh.svc.term_writeln("  timezone [min]  - Show/set timezone (e.g. 180=UTC+3, 420=UTC+7)");
    sh.svc.term_writeln("  setup           - Install system files");
    sh.svc.term_writeln("  whoami          - Current user info");
    sh.svc.term_writeln("  su              - Switch user");
    sh.svc.term_writeln("  adduser <n> [r] - Add user (admin) [r=admin for admin role]");
    sh.svc.term_writeln("  passwd <pw>     - Change password");
    sh.svc.term_writeln("  regedit         - Registry editor");
    sh.svc.term_writeln("  lumiec <src> [o]- LumieC compiler");
    sh.svc.term_writeln("  sysload <mod>   - Load .sys module");
    sh.svc.term_writeln("  beep [freq] [ms] - PC speaker beep (default 440Hz 200ms)");
}

pub fn cmd_clear(sh: &Shell) {
    let color = if sh.current_drive == b'A' { 0 } else { 1 };
    sh.svc.term_clear(color);
    sh.svc.term_set_bg(color);
}

pub fn cmd_pwd(sh: &Shell) {
    sh.svc.term_writeln(sh.cwd_str());
}

pub fn cmd_echo(sh: &Shell, text: Option<&[u8]>) {
    let text = match text {
        Some(t) => core::str::from_utf8(t).unwrap_or(""),
        None => return,
    };

    if Shell::str_contains(text, "catch") || Shell::str_contains(text, "balls") {
        sh.svc.term_set_fg(11);
        sh.svc.term_writeln("  You found the Easter Egg!");
        sh.svc.term_writeln("");
        sh.svc.term_set_fg(14);
        sh.svc.term_writeln("       ___===___");
        sh.svc.term_writeln("      /  O   O  \\");
        sh.svc.term_set_fg(10);
        sh.svc.term_writeln("     |   \\___/   |");
        sh.svc.term_writeln("      \\  _____  /");
        sh.svc.term_set_fg(4);
        sh.svc.term_writeln("      /         \\");
        sh.svc.term_set_fg(13);
        sh.svc.term_writeln("     |  0     0  |");
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("      \\   U    /   CATCH THE BALLS!");
        sh.svc.term_set_fg(15);
        sh.svc.term_writeln("       \\______/");
        sh.svc.term_writeln("");
        sh.svc.term_set_fg(10);
        sh.svc.term_writeln("  Try the game: Catch the Balls!");
        sh.svc.term_set_fg(15);
        return;
    }

    sh.svc.term_writeln(text);
}

pub fn cmd_ver(sh: &Shell) {
    sh.svc.term_set_fg(15);
    sh.svc.term_writeln("LumieOS v0.1 - Windows Edition (64-bit UEFI)");
    if sh.lumieos_installed() {
        sh.svc.term_set_fg(11);
        sh.svc.term_writeln("Installed on: FAT32");
        sh.svc.term_set_fg(15);
    } else {
        sh.svc.term_set_fg(14);
        sh.svc.term_writeln("Not installed - run installer");
        sh.svc.term_set_fg(15);
    }
}

pub fn cmd_info(sh: &Shell) {
    let fb = sh.svc.gop_get_fb();
    let mut buf = [0u8; 64];

    sh.svc.term_set_fg(12);
    sh.svc.term_writeln("      ██╗     ██╗   ██╗███╗   ███╗██╗███████╗ ██████╗ ███████╗");
    sh.svc.term_set_fg(14);
    sh.svc.term_writeln("      ██║     ██║   ██║████╗ ████║██║██╔════╝██╔═══██╗██╔════╝");
    sh.svc.term_set_fg(10);
    sh.svc.term_writeln("      ██║     ██║   ██║██╔████╔██║██║█████╗  ██║   ██║███████╗");
    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("      ██║     ██║   ██║██║╚██╔╝██║██║██╔══╝  ██║   ██║╚════██║");
    sh.svc.term_set_fg(13);
    sh.svc.term_writeln("      ███████╗╚██████╔╝██║ ╚═╝ ██║██║███████╗╚██████╔╝███████║");
    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("      ╚══════╝ ╚═════╝ ╚═╝     ╚═╝╚═╝╚══════╝ ╚═════╝ ╚══════╝");
    sh.svc.term_writeln("");

    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("  ===== LumieOS Windows Edition =====");
    sh.svc.term_set_fg(15);

    sh.svc.term_set_fg(10);
    sh.svc.term_write("  Resolution:   ");
    sh.svc.term_set_fg(15);
    let s = &mut buf[..];
    let len = write_int(s, fb.width);
    sh.svc.term_write(core::str::from_utf8(&s[..len]).unwrap_or("0"));
    sh.svc.term_write("x");
    let len = write_int(s, fb.height);
    sh.svc.term_writeln(core::str::from_utf8(&s[..len]).unwrap_or("0"));

    sh.svc.term_set_fg(10);
    sh.svc.term_write("  Framebuffer:  ");
    sh.svc.term_set_fg(15);
    sh.svc.term_write("0x");
    let len = write_int_hex(s, fb.base as u32);
    sh.svc.term_write(core::str::from_utf8(&s[..len]).unwrap_or("0"));
    sh.svc.term_write(" (");
    let len = write_int(s, (fb.size / 1024) as i32);
    sh.svc.term_write(core::str::from_utf8(&s[..len]).unwrap_or("0"));
    sh.svc.term_writeln(" KB)");

    sh.svc.term_set_fg(10);
    sh.svc.term_write("  Terminal:     ");
    sh.svc.term_set_fg(15);
    let len = write_int(s, sh.svc.term_get_width());
    sh.svc.term_write(core::str::from_utf8(&s[..len]).unwrap_or("0"));
    sh.svc.term_write("x");
    let len = write_int(s, sh.svc.term_get_height());
    sh.svc.term_writeln(core::str::from_utf8(&s[..len]).unwrap_or("0"));

    sh.svc.term_set_fg(10);
    sh.svc.term_write("  Free Memory:  ");
    sh.svc.term_set_fg(15);
    let free_mem = sh.svc.mm_get_free_mem();
    let len = write_int(s, (free_mem / 1024) as i32);
    sh.svc.term_write(core::str::from_utf8(&s[..len]).unwrap_or("0"));
    sh.svc.term_writeln(" KB");

    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("  ===================================");
    sh.svc.term_set_fg(15);
}

pub fn cmd_time(sh: &Shell) {
    let mut buf = [0u8; 64];
    if sh.svc.get_time(&mut buf) > 0 {
        let tz_offset = sh.read_timezone();
        sh.svc.term_set_fg(11);
        sh.svc.term_write("Current time: ");
        sh.svc.term_set_fg(15);
        let time_str = core::str::from_utf8(&buf).unwrap_or("");
        sh.svc.term_writeln(time_str.trim_end_matches('\0'));
        if tz_offset != 0 {
            sh.svc.term_set_fg(14);
            sh.svc.term_write("Timezone offset: UTC");
            if tz_offset >= 0 {
                sh.svc.term_write("+");
            }
            let mut tz_buf = [0u8; 16];
            let n = write_int(&mut tz_buf, tz_offset);
            sh.svc.term_writeln(core::str::from_utf8(&tz_buf[..n]).unwrap_or(""));
        }
    } else {
        sh.svc.term_set_fg(14);
        sh.svc.term_writeln("Time not available");
    }
    sh.svc.term_set_fg(15);
}

pub fn cmd_timezone(sh: &Shell, arg: Option<&[u8]>) {
    if let Some(a) = arg {
        let a_str = core::str::from_utf8(a).unwrap_or("0");
        let offset = parse_int(a_str).unwrap_or(0);
        if offset < -720 || offset > 840 {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Invalid timezone offset. Use -720 to 840 minutes.");
            sh.svc.term_set_fg(15);
            return;
        }
        let mut tz_buf = [0u8; 16];
        let n = write_int(&mut tz_buf, offset);
        sh.svc.fs_write_file("/system/timezone.cfg", &tz_buf[..n + 1]);
        sh.svc.term_set_fg(10);
        sh.svc.term_write("Timezone set to UTC");
        if offset >= 0 {
            sh.svc.term_write("+");
        }
        sh.svc.term_writeln(core::str::from_utf8(&tz_buf[..n]).unwrap_or(""));
        sh.svc.term_set_fg(15);
    } else {
        let tz = sh.read_timezone();
        if tz == 0 && !sh.svc.fs_exists("/system/timezone.cfg") {
            sh.svc.term_set_fg(14);
            sh.svc.term_writeln("No timezone set. Use 'timezone <minutes>' to set.");
        } else {
            sh.svc.term_set_fg(11);
            sh.svc.term_write("Current timezone: UTC");
            if tz >= 0 {
                sh.svc.term_write("+");
            }
            let mut buf = [0u8; 16];
            let n = write_int(&mut buf, tz);
            sh.svc.term_writeln(core::str::from_utf8(&buf[..n]).unwrap_or(""));
        }
        sh.svc.term_set_fg(15);
    }
}

pub fn cmd_reboot(sh: &Shell) {
    if sh.confirm_action("Reboot system?") {
        sh.svc.reboot();
    }
}

pub fn cmd_shutdown(sh: &Shell) {
    if sh.confirm_action("Shutdown system?") {
        sh.svc.shutdown();
    }
}

pub fn cmd_ps(sh: &Shell) {
    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("PID   NAME          PRIO  STATE");
    sh.svc.term_set_fg(15);
    let count = sh.svc.sched_get_count();
    for i in 0..count {
        let mut name_buf = [0u8; 64];
        if sh.svc.sched_get_name(i, &mut name_buf) <= 0 {
            continue;
        }
        let name = core::str::from_utf8(&name_buf).unwrap_or("").trim_end_matches('\0');
        let st = sh.svc.sched_get_state(i);
        let prio = sh.svc.sched_get_priority(i);
        let mut line = [0u8; 64];
        let mut pos = 0;

        let id_s = format_int(i);
        for &b in b" " {
            if pos < 63 { line[pos] = b; pos += 1; }
        }
        for &b in id_s.as_bytes() {
            if pos < 63 { line[pos] = b; pos += 1; }
        }
        for &b in b"     " {
            if pos < 63 { line[pos] = b; pos += 1; }
        }
        for &b in name.as_bytes() {
            if pos < 63 { line[pos] = b; pos += 1; }
        }
        let padding = if name.len() < 14 { 14 - name.len() } else { 0 };
        for _ in 0..padding.min(6) {
            if pos < 63 { line[pos] = b' '; pos += 1; }
        }
        if prio == 0 {
            for &b in b"USER" { if pos < 63 { line[pos] = b; pos += 1; } }
        } else {
            for &b in b"SYS " { if pos < 63 { line[pos] = b; pos += 1; } }
        }
        for &b in b"  " { if pos < 63 { line[pos] = b; pos += 1; } }
        let state_str = match st {
            0 => b"RUNNING",
            1 => b"READY",
            2 => b"BLOCKED",
            _ => b"DEAD",
        };
        for &b in state_str {
            if pos < 63 { line[pos] = b; pos += 1; }
        }
        sh.svc.term_writeln(core::str::from_utf8(&line[..pos]).unwrap_or(""));
    }
    if count == 0 {
        sh.svc.term_writeln(" <no tasks>");
    }
}

pub fn cmd_whoami(sh: &Shell) {
    let mut name_buf = [0u8; 64];
    let n = sh.svc.users_current_name(&mut name_buf);
    if n > 0 {
        let name = core::str::from_utf8(&name_buf[..n as usize]).unwrap_or("");
        sh.svc.term_set_fg(11);
        sh.svc.term_write(name);
        sh.svc.term_set_fg(15);
        let role = sh.svc.users_current_role();
        if role == USER_ROLE_ADMIN {
            sh.svc.term_writeln(" (admin)");
        } else {
            sh.svc.term_writeln(" (user)");
        }
    } else {
        sh.svc.term_set_fg(14);
        sh.svc.term_writeln("Not logged in");
    }
    sh.svc.term_set_fg(15);
}

pub fn cmd_su(sh: &Shell) {
    let mut name = [0u8; 64];
    sh.svc.term_set_fg(11);
    sh.svc.term_write("Username: ");
    sh.svc.term_set_fg(15);
    let mut p = 0;
    loop {
        let c = sh.svc.kbd_getchar();
        if c == b'\n' as i32 {
            break;
        }
        if c == b'\x08' as i32 {
            if p > 0 {
                p -= 1;
                sh.svc.term_write("\b \b");
            }
        } else if c >= b' ' as i32 && c <= b'~' as i32 && p < 63 {
            name[p] = c as u8;
            p += 1;
            sh.svc.term_putchar(c as u8);
        }
    }
    sh.svc.term_writeln("");
    if p == 0 {
        return;
    }
    let name_str = core::str::from_utf8(&name[..p]).unwrap_or("");
    let uid = sh.svc.users_login(name_str, None);
    if uid >= 0 {
        sh.svc.term_set_fg(10);
        sh.svc.term_write("Switched to: ");
        sh.svc.term_writeln(name_str);
    } else {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Unknown user");
    }
    sh.svc.term_set_fg(15);
}

pub fn cmd_adduser(sh: &Shell, name: Option<&[u8]>, role_str: Option<&[u8]>) {
    if sh.svc.users_current_role() != USER_ROLE_ADMIN {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Only administrators can add users.");
        sh.svc.term_set_fg(15);
        return;
    }
    let name = match name {
        Some(n) => core::str::from_utf8(n).unwrap_or(""),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: adduser <name> [admin]");
            sh.svc.term_set_fg(15);
            return;
        }
    };
    let mut role = USER_ROLE_USER;
    if let Some(r) = role_str {
        let r_str = core::str::from_utf8(r).unwrap_or("");
        if r_str == "admin" || r_str == "1" {
            role = USER_ROLE_ADMIN;
        }
    }
    if sh.svc.users_add(name, "", role) == 0 {
        sh.svc.term_set_fg(10);
        sh.svc.term_write("User '");
        sh.svc.term_write(name);
        sh.svc.term_writeln("' created.");
    } else {
        sh.svc.term_set_fg(12);
        sh.svc.term_write("Failed to create user '");
        sh.svc.term_write(name);
        sh.svc.term_writeln("'.");
    }
    sh.svc.term_set_fg(15);
}

pub fn cmd_passwd(sh: &Shell, new_pass: Option<&[u8]>) {
    let mut name_buf = [0u8; 64];
    let n = sh.svc.users_current_name(&mut name_buf);
    if n <= 0 {
        return;
    }
    let new_pass = match new_pass {
        Some(p) => core::str::from_utf8(p).unwrap_or(""),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: passwd <new_password>");
            sh.svc.term_set_fg(15);
            return;
        }
    };
    let name = core::str::from_utf8(&name_buf[..n as usize]).unwrap_or("");
    let role = sh.svc.users_current_role();
    let mut saved_name = [0u8; 64];
    saved_name[..name.len().min(63)].copy_from_slice(name.as_bytes());
    sh.svc.users_remove(name);
    if sh.svc.users_add(name, new_pass, role) == 0 {
        sh.svc.term_set_fg(10);
        sh.svc.term_writeln("Password changed.");
    } else {
        sh.svc.users_add(name, "", role);
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Failed to change password.");
    }
    sh.svc.term_set_fg(15);
}

pub fn cmd_regedit(sh: &Shell) {
    let bg = if sh.current_drive == b'A' { 0 } else { 1 };
    sh.svc.term_clear(bg as u32);
    sh.svc.term_set_bg(bg as u32);
    sh.svc.term_set_fg(15);

    let mut line = [0u8; 128];
    loop {
        sh.svc.term_set_pos(0, 0);
        sh.svc.term_set_fg(11);
        sh.svc.term_writeln("LumieOS Registry Editor");
        sh.svc.term_set_fg(15);
        let width = sh.svc.term_get_width();
        for _ in 0..width {
            sh.svc.term_write("-");
        }
        sh.svc.term_writeln("");

        let mut list_buf = [0u8; 2048];
        sh.svc.reg_list(&mut list_buf);
        let list_str = core::str::from_utf8(&list_buf).unwrap_or("").trim_end_matches('\0');
        if list_str.is_empty() {
            sh.svc.term_set_fg(14);
            sh.svc.term_writeln("  <empty>");
            sh.svc.term_set_fg(15);
        } else {
            sh.svc.term_write(list_str);
        }

        for _ in 0..width {
            sh.svc.term_write("-");
        }
        sh.svc.term_writeln("");
        sh.svc.term_set_fg(14);
        sh.svc.term_writeln("Commands: get <key> | set <key>=<value> | del <key> | quit");
        sh.svc.term_set_fg(15);
        sh.svc.term_write("> ");

        let mut p = 0;
        line = [0u8; 128];
        loop {
            let c = sh.svc.kbd_getchar();
            if c == b'\n' as i32 {
                break;
            }
            if c == b'\x08' as i32 {
                if p > 0 {
                    p -= 1;
                    sh.svc.term_write("\b \b");
                }
            } else if c >= b' ' as i32 && c <= b'~' as i32 && p < 127 {
                line[p] = c as u8;
                p += 1;
                sh.svc.term_putchar(c as u8);
            }
        }
        sh.svc.term_writeln("");

        let cmd = core::str::from_utf8(&line[..p]).unwrap_or("").trim_end_matches('\0');
        match cmd {
            "quit" | "exit" => break,
            "list" => continue,
            _ => {}
        }

        if let Some(rest) = cmd.strip_prefix("get ") {
            let mut val = [0u8; 256];
            if sh.svc.reg_get(rest, &mut val) > 0 {
                sh.svc.term_write("  = ");
                let val_str = core::str::from_utf8(&val).unwrap_or("").trim_end_matches('\0');
                sh.svc.term_writeln(val_str);
            } else {
                sh.svc.term_set_fg(12);
                sh.svc.term_write("Key not found: ");
                sh.svc.term_writeln(rest);
                sh.svc.term_set_fg(15);
            }
            sh.svc.term_write("Press any key...");
            sh.svc.kbd_getchar();
            continue;
        }

        if let Some(rest) = cmd.strip_prefix("del ") {
            if sh.svc.reg_del(rest) == 0 {
                sh.svc.term_set_fg(10);
                sh.svc.term_writeln("Deleted.");
            } else {
                sh.svc.term_set_fg(12);
                sh.svc.term_writeln("Key not found.");
            }
            sh.svc.term_set_fg(15);
            continue;
        }

        if let Some(rest) = cmd.strip_prefix("set ") {
            if let Some(eq_pos) = rest.find('=') {
                let k = &rest[..eq_pos];
                let v = &rest[eq_pos + 1..];
                if sh.svc.reg_set(k, v) == 0 {
                    sh.svc.term_set_fg(10);
                    sh.svc.term_writeln("Set.");
                } else {
                    sh.svc.term_set_fg(12);
                    sh.svc.term_writeln("Failed.");
                }
            } else {
                sh.svc.term_set_fg(12);
                sh.svc.term_writeln("Usage: set key=value");
            }
            continue;
        }

        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Unknown command. Try: get, set, del, list, quit");
        sh.svc.term_set_fg(15);
        sh.svc.term_write("Press any key...");
        sh.svc.kbd_getchar();
    }

    let bg2 = if sh.current_drive == b'A' { 0 } else { 1 };
    sh.svc.term_clear(bg2 as u32);
    sh.svc.term_set_bg(bg2 as u32);
}

pub fn cmd_notepad(sh: &Shell) {
    let mut fname = [0u8; 256];
    sh.svc.term_set_fg(11);
    sh.svc.term_write("Notepad - Enter filename to edit: ");
    sh.svc.term_set_fg(15);
    let mut pos = 0;
    loop {
        let c = sh.svc.kbd_getchar();
        if c == b'\n' as i32 {
            sh.svc.term_writeln("");
            break;
        }
        if c == b'\x08' as i32 {
            if pos > 0 {
                pos -= 1;
                sh.svc.term_write("\b \b");
            }
            continue;
        }
        if c >= b' ' as i32 && c <= b'~' as i32 && pos < 255 {
            fname[pos] = c as u8;
            pos += 1;
            sh.svc.term_putchar(c as u8);
        }
    }
    if pos > 0 {
        let mut resolved = [0u8; 256];
        let name_str = core::str::from_utf8(&fname[..pos]).unwrap_or("");
        sh.resolve_path(name_str, &mut resolved);
        let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');
        sh.svc.editor_run(resolved_str);
    }
}

pub fn cmd_disks(sh: &Shell) {
    let count = sh.svc.disk_enum_all();
    if count == 0 {
        sh.svc.term_set_fg(14);
        sh.svc.term_writeln("No disks found.");
        sh.svc.term_set_fg(15);
        return;
    }
    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("Available drives:");
    sh.svc.term_set_fg(15);
    for i in 0..count {
        let mut info = crate::DiskInfo::default();
        if sh.svc.disk_get_info(i, &mut info) != 0 || !info.present {
            continue;
        }
        let letter = sh.svc.disk_get_drive_letter(i);
        let total_bytes = info.sector_count * info.sector_size as u64;
        let mut sz = [0u8; 32];
        let mut buf = [0u8; 64];
        let mut pos = 0;
        buf[pos] = b' ';
        pos += 1;
        if letter != 0 {
            buf[pos] = letter;
            pos += 1;
            buf[pos] = b':';
            pos += 1;
            buf[pos] = b' ';
            pos += 1;
        }
        let name_str = core::str::from_utf8(&info.name).unwrap_or("").trim_end_matches('\0');
        for &b in name_str.as_bytes() {
            if pos < 63 { buf[pos] = b; pos += 1; }
        }
        for &b in b" (" {
            if pos < 63 { buf[pos] = b; pos += 1; }
        }
        let n;
        let unit;
        if total_bytes >= 1024 * 1024 * 1024 {
            n = write_int(&mut sz, (total_bytes / (1024 * 1024 * 1024)) as i32);
            unit = b" GB)";
        } else {
            n = write_int(&mut sz, (total_bytes / (1024 * 1024)) as i32);
            unit = b" MB)";
        }
        for &b in core::str::from_utf8(&sz[..n]).unwrap_or("0").as_bytes() {
            if pos < 63 { buf[pos] = b; pos += 1; }
        }
        for &b in unit {
            if pos < 63 { buf[pos] = b; pos += 1; }
        }
        sh.svc.term_writeln(core::str::from_utf8(&buf[..pos]).unwrap_or(""));
    }
}

pub fn cmd_beep(sh: &Shell, freq: u32, dur: u32) {
    let freq = if freq == 0 { 440 } else { freq };
    let dur = if dur == 0 { 200 } else { dur };
    sh.svc.term_set_fg(10);
    sh.svc.term_write("Beep: ");
    let mut fbuf = [0u8; 16];
    let mut dbuf = [0u8; 16];
    let fn_ = write_int(&mut fbuf, freq as i32);
    let dn = write_int(&mut dbuf, dur as i32);
    sh.svc.term_write(core::str::from_utf8(&fbuf[..fn_]).unwrap_or("440"));
    sh.svc.term_write(" Hz, ");
    sh.svc.term_write(core::str::from_utf8(&dbuf[..dn]).unwrap_or("200"));
    sh.svc.term_writeln(" ms");
    sh.svc.term_set_fg(15);
    sh.svc.pcspkr_beep(freq, dur);
}

pub fn write_int(buf: &mut [u8], val: i32) -> usize {
    if val == 0 {
        if !buf.is_empty() {
            buf[0] = b'0';
            return 1;
        }
        return 0;
    }
    let mut pos = 0;
    let negative = val < 0;
    let mut v = if negative { -(val as i64) } else { val as i64 };
    let mut digits = [0u8; 16];
    let mut nd = 0;
    while v > 0 {
        digits[nd] = (v % 10) as u8 + b'0';
        v /= 10;
        nd += 1;
    }
    if negative && pos < buf.len() {
        buf[pos] = b'-';
        pos += 1;
    }
    for i in (0..nd).rev() {
        if pos < buf.len() {
            buf[pos] = digits[i];
            pos += 1;
        }
    }
    pos
}

pub fn write_int_hex(buf: &mut [u8], val: u32) -> usize {
    if val == 0 {
        if !buf.is_empty() {
            buf[0] = b'0';
            return 1;
        }
        return 0;
    }
    let mut pos = 0;
    let mut v = val;
    let mut digits = [0u8; 16];
    let mut nd = 0;
    let hex_chars = b"0123456789ABCDEF";
    while v > 0 {
        digits[nd] = hex_chars[(v & 0xF) as usize];
        v >>= 4;
        nd += 1;
    }
    for i in (0..nd).rev() {
        if pos < buf.len() {
            buf[pos] = digits[i];
            pos += 1;
        }
    }
    pos
}

fn format_int(val: i32) -> [u8; 16] {
    let mut buf = [0u8; 16];
    let n = write_int(&mut buf, val);
    let mut out = [0u8; 16];
    out[..n].copy_from_slice(&buf[..n]);
    out
}
