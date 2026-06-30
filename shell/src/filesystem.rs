use crate::{Shell, LumieDirEnt, builtins::write_int};

pub fn cmd_ls(sh: &Shell, path: Option<&[u8]>) {
    let mut entries = [LumieDirEnt {
        name: [0u8; 256],
        is_dir: false,
        size: 0,
    }; 256];
    let mut resolved = [0u8; 256];
    let dir = match path {
        Some(p) => {
            let p_str = core::str::from_utf8(p).unwrap_or("");
            sh.resolve_path(p_str, &mut resolved);
            core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0').to_owned()
        }
        None => sh.cwd_str().to_owned(),
    };
    let count = sh.svc.fs_list_dir(&dir, &mut entries);
    if count < 0 {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Error: cannot list directory");
        sh.svc.term_set_fg(15);
        return;
    }

    let mut total_size: u32 = 0;
    for i in 0..count {
        let idx = i as usize;
        let name = core::str::from_utf8(&entries[idx].name).unwrap_or("").trim_end_matches('\0');
        let is_pe = if !entries[idx].is_dir {
            name.ends_with(".exe") || name.ends_with(".dll")
        } else {
            false
        };

        if entries[idx].is_dir {
            sh.svc.term_set_fg(11);
            sh.svc.term_write("[DIR ] ");
        } else if is_pe {
            sh.svc.term_set_fg(13);
            sh.svc.term_write("[APP ] ");
        } else {
            sh.svc.term_set_fg(10);
            sh.svc.term_write("[FILE] ");
        }
        sh.svc.term_set_fg(15);
        sh.svc.term_write(name);

        if !entries[idx].is_dir {
            let mut size_str = [0u8; 32];
            let n = write_int(&mut size_str, entries[idx].size as i32);
            sh.svc.term_set_fg(14);
            sh.svc.term_write(" (");
            sh.svc.term_write(core::str::from_utf8(&size_str[..n]).unwrap_or("0"));
            sh.svc.term_writeln(" bytes)");
            total_size += entries[idx].size;
        } else {
            sh.svc.term_writeln("");
        }
    }

    sh.svc.term_set_fg(15);
    let mut buf = [0u8; 64];
    let n = write_int(&mut buf, count);
    sh.svc.term_write(core::str::from_utf8(&buf[..n]).unwrap_or("0"));
    sh.svc.term_write(" items");
    if total_size > 0 {
        let mut sz = [0u8; 32];
        let n = write_int(&mut sz, total_size as i32);
        sh.svc.term_write(", ");
        sh.svc.term_write(core::str::from_utf8(&sz[..n]).unwrap_or("0"));
        sh.svc.term_write(" bytes total");
    }
    sh.svc.term_writeln("");
}

pub fn cmd_cd(sh: &mut Shell, path: Option<&[u8]>) {
    match path {
        None | Some(b"") => {
            sh.set_cwd(b"/");
            return;
        }
        Some(p) => {
            let p_str = core::str::from_utf8(p).unwrap_or("");
            let mut resolved = [0u8; 256];
            sh.resolve_path(p_str, &mut resolved);
            let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');

            // Strip trailing slash
            let mut rlen = resolved_str.len();
            if rlen > 1 && resolved_str.as_bytes()[rlen - 1] == b'/' {
                rlen -= 1;
            }
            let clean = &resolved_str[..rlen];

            if !sh.svc.fs_exists(clean) {
                sh.svc.term_set_fg(12);
                sh.svc.term_write("cd: directory not found: ");
                sh.svc.term_writeln(p_str);
                sh.svc.term_set_fg(15);
                return;
            }

            let mut check = [LumieDirEnt {
                name: [0u8; 256],
                is_dir: false,
                size: 0,
            }; 1];
            if sh.svc.fs_list_dir(clean, &mut check) < 0 {
                sh.svc.term_set_fg(12);
                sh.svc.term_write("cd: not a directory: ");
                sh.svc.term_writeln(p_str);
                sh.svc.term_set_fg(15);
                return;
            }

            sh.cwd[..clean.len()].copy_from_slice(clean.as_bytes());
            sh.cwd[clean.len()] = 0;
            sh.cwd_len = clean.len();
            if sh.cwd_len == 0 || sh.cwd[0] == 0 {
                sh.cwd[0] = b'/';
                sh.cwd[1] = 0;
                sh.cwd_len = 1;
            }
        }
    }
}

fn may_delete(sh: &Shell, path: &str) -> bool {
    if sh.current_drive == b'A' {
        return true;
    }
    let role = sh.svc.users_current_role();
    if role < 0 {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Permission denied: not logged in.");
        sh.svc.term_set_fg(15);
        return false;
    }
    if role == crate::USER_ROLE_USER {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Permission denied: users cannot delete files.");
        sh.svc.term_set_fg(15);
        return false;
    }
    if role == crate::USER_ROLE_ADMIN {
        if sh.svc.users_is_protected_path(path) {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Permission denied: system/driver files are protected.");
            sh.svc.term_set_fg(15);
            return false;
        }
        let running = ["/system/kernel.lkrn", "/system/shell.lsh"];
        for r in &running {
            if path == *r {
                sh.svc.term_set_fg(12);
                sh.svc.term_writeln("Permission denied: file is running.");
                sh.svc.term_set_fg(15);
                return false;
            }
        }
    }
    true
}

pub fn cmd_cat(sh: &Shell, file: Option<&[u8]>) {
    let file = match file {
        Some(f) => core::str::from_utf8(f).unwrap_or(""),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: cat <filename>");
            sh.svc.term_set_fg(15);
            return;
        }
    };

    let mut resolved = [0u8; 256];
    sh.resolve_path(file, &mut resolved);
    let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');

    let size = sh.svc.fs_get_file_size(resolved_str);
    if size < 0 {
        sh.svc.term_set_fg(12);
        sh.svc.term_write("Error: file '");
        sh.svc.term_write(file);
        sh.svc.term_writeln("' not found");
        sh.svc.term_set_fg(15);
        return;
    }

    if size > 64 * 1024 {
        sh.svc.term_set_fg(14);
        sh.svc.term_write("File too large (");
        let mut sz = [0u8; 32];
        let n = write_int(&mut sz, size);
        sh.svc.term_write(core::str::from_utf8(&sz[..n]).unwrap_or("0"));
        sh.svc.term_writeln(" bytes). Max 64KB.");
        sh.svc.term_set_fg(15);
        return;
    }

    let alloc_sz = if size < 4096 { 4096 } else { (size + 1) as usize };
    let mut buf = vec![0u8; alloc_sz];
    let read = sh.svc.fs_read_file(resolved_str, &mut buf);
    if read < 0 {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Error reading file");
        sh.svc.term_set_fg(15);
        return;
    }

    let data = &buf[..read as usize];
    if let Some(pt) = sh.svc.pe_type(data) {
        let arch = sh.svc.pe_machine_str(data);
        let mut sect_str = [0u8; 16];
        // We can't easily access nt->n_sections from the trait, skip section count
        sh.svc.term_set_fg(11);
        sh.svc.term_write("PE ");
        sh.svc.term_write(pt);
        sh.svc.term_write(" executable");
        sh.svc.term_set_fg(15);
        sh.svc.term_write(" (");
        if let Some(arch) = arch {
            sh.svc.term_write(arch);
            sh.svc.term_write(", ");
        }
        let mut sz = [0u8; 16];
        let n = write_int(&mut sz, size);
        sh.svc.term_write(core::str::from_utf8(&sz[..n]).unwrap_or("0"));
        sh.svc.term_writeln(" bytes)");
        return;
    }

    let last_nl = if read > 0 && buf[read as usize - 1] != b'\n' { 0 } else { 1 };
    let s = core::str::from_utf8(&buf[..read as usize]).unwrap_or("");
    sh.svc.term_write(s);
    if last_nl == 0 {
        sh.svc.term_writeln("");
    }
}

pub fn cmd_rm(sh: &Shell, file: Option<&[u8]>) {
    let file = match file {
        Some(f) => core::str::from_utf8(f).unwrap_or(""),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: rm <file>");
            sh.svc.term_set_fg(15);
            return;
        }
    };

    let mut resolved = [0u8; 256];
    sh.resolve_path(file, &mut resolved);
    let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');

    if !may_delete(sh, resolved_str) {
        return;
    }

    let ret = sh.svc.fs_delete(resolved_str);
    if ret == -2 {
        sh.svc.term_set_fg(12);
        sh.svc.term_write("rm: directory not empty: ");
        sh.svc.term_writeln(file);
        sh.svc.term_set_fg(15);
    } else if ret != 0 {
        sh.svc.term_set_fg(12);
        sh.svc.term_write("rm: failed to delete: ");
        sh.svc.term_writeln(file);
        sh.svc.term_set_fg(15);
    }
}

pub fn cmd_rmdir(sh: &Shell, path: Option<&[u8]>) {
    let path = match path {
        Some(p) => core::str::from_utf8(p).unwrap_or(""),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: rmdir <directory>");
            sh.svc.term_set_fg(15);
            return;
        }
    };

    let mut resolved = [0u8; 256];
    sh.resolve_path(path, &mut resolved);
    let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');

    if !may_delete(sh, resolved_str) {
        return;
    }

    let ret = sh.svc.fs_delete(resolved_str);
    if ret == -2 {
        sh.svc.term_set_fg(12);
        sh.svc.term_write("rmdir: directory not empty: ");
        sh.svc.term_writeln(path);
        sh.svc.term_set_fg(15);
    } else if ret != 0 {
        sh.svc.term_set_fg(12);
        sh.svc.term_write("rmdir: failed to delete: ");
        sh.svc.term_writeln(path);
        sh.svc.term_set_fg(15);
    }
}

pub fn cmd_mkdir(sh: &Shell, path: Option<&[u8]>) {
    let path = match path {
        Some(p) => core::str::from_utf8(p).unwrap_or(""),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: mkdir <directory>");
            sh.svc.term_set_fg(15);
            return;
        }
    };

    let mut resolved = [0u8; 256];
    sh.resolve_path(path, &mut resolved);
    let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');

    if sh.svc.fs_mkdir(resolved_str) != 0 {
        sh.svc.term_set_fg(12);
        sh.svc.term_write("mkdir: failed to create: ");
        sh.svc.term_writeln(path);
        sh.svc.term_set_fg(15);
    }
}

pub fn cmd_wher(sh: &Shell, dir: Option<&[u8]>, pattern: Option<&[u8]>) {
    let pattern = match pattern {
        Some(p) => core::str::from_utf8(p).unwrap_or(""),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: wher <directory> <pattern>");
            sh.svc.term_set_fg(15);
            return;
        }
    };

    let base = match dir {
        Some(d) => {
            let d_str = core::str::from_utf8(d).unwrap_or("/");
            if d_str.is_empty() { "/" } else { d_str }
        }
        None => "/",
    };

    const WHER_STACK_SIZE: usize = 256;
    const WHER_PATH_MAX: usize = 256;

    let mut stack_mem = [[0u8; WHER_PATH_MAX]; WHER_STACK_SIZE];
    let mut sp: usize = 0;
    let blen = base.len().min(WHER_PATH_MAX - 1);
    stack_mem[sp][..blen].copy_from_slice(base.as_bytes());
    stack_mem[sp][blen] = 0;
    sp += 1;

    let mut found = 0;

    while sp > 0 {
        sp -= 1;
        let cur_len = stack_mem[sp].iter().position(|&c| c == 0).unwrap_or(WHER_PATH_MAX);
        let cur = core::str::from_utf8(&stack_mem[sp][..cur_len]).unwrap_or("/");

        let mut entries = [LumieDirEnt {
            name: [0u8; 256],
            is_dir: false,
            size: 0,
        }; 128];
        let count = sh.svc.fs_list_dir(cur, &mut entries);
        if count < 0 {
            continue;
        }

        for i in 0..count {
            let idx = i as usize;
            let ename = core::str::from_utf8(&entries[idx].name).unwrap_or("").trim_end_matches('\0');
            let ename_bytes = ename.as_bytes();

            let mut flen = cur_len;
            let mut full = [0u8; WHER_PATH_MAX];
            full[..flen].copy_from_slice(&stack_mem[sp][..flen]);
            if flen > 0 && full[flen - 1] != b'/' {
                if flen < WHER_PATH_MAX - 1 {
                    full[flen] = b'/';
                    flen += 1;
                }
            }
            let nlen = ename_bytes.len().min(WHER_PATH_MAX - 1 - flen);
            full[flen..flen + nlen].copy_from_slice(&ename_bytes[..nlen]);
            flen += nlen;
            full[flen] = 0;

            if entries[idx].is_dir {
                if sp < WHER_STACK_SIZE {
                    stack_mem[sp][..flen].copy_from_slice(&full[..flen]);
                    stack_mem[sp][flen] = 0;
                    sp += 1;
                }
            }

            if Shell::match_name(pattern, ename) {
                let full_str = core::str::from_utf8(&full[..flen]).unwrap_or("");
                sh.svc.term_set_fg(if entries[idx].is_dir { 11 } else { 10 });
                sh.svc.term_write(full_str);
                if !entries[idx].is_dir {
                    let mut sz = [0u8; 32];
                    let n = write_int(&mut sz, entries[idx].size as i32);
                    sh.svc.term_set_fg(14);
                    sh.svc.term_write(" (");
                    sh.svc.term_write(core::str::from_utf8(&sz[..n]).unwrap_or("0"));
                    sh.svc.term_write(" bytes)");
                }
                sh.svc.term_set_fg(15);
                sh.svc.term_writeln("");
                found += 1;
            }
        }
    }

    sh.svc.term_set_fg(15);
    if found == 0 {
        sh.svc.term_set_fg(14);
        sh.svc.term_write("wher: no matches for '");
        sh.svc.term_write(pattern);
        sh.svc.term_writeln("'");
    } else {
        let mut buf = [0u8; 32];
        let n = write_int(&mut buf, found);
        sh.svc.term_write(core::str::from_utf8(&buf[..n]).unwrap_or("0"));
        sh.svc.term_writeln(" matches found");
    }
    sh.svc.term_set_fg(15);
}

pub fn cmd_wher1(sh: &Shell, pattern: Option<&[u8]>) {
    match pattern {
        Some(p) => cmd_wher(sh, Some(b"/"), Some(p)),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: wher1 <pattern>");
            sh.svc.term_set_fg(15);
        }
    }
}

pub fn cmd_format(sh: &Shell, drive: Option<&[u8]>) {
    let drive = match drive {
        Some(d) => {
            let d_str = core::str::from_utf8(d).unwrap_or("");
            if d_str.len() < 2 || d_str.as_bytes()[1] != b':' {
                sh.svc.term_set_fg(12);
                sh.svc.term_writeln("Usage: format <drive>:  (e.g. format C:)");
                sh.svc.term_set_fg(15);
                return;
            }
            d_str
        }
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: format <drive>:  (e.g. format C:)");
            sh.svc.term_set_fg(15);
            return;
        }
    };

    let mut letter = drive.as_bytes()[0];
    if letter >= b'a' && letter <= b'z' {
        letter = letter - b'a' + b'A';
    }

    if sh.current_drive != b'A' && sh.svc.users_current_role() != crate::USER_ROLE_ADMIN {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Only administrators can format disks.");
        sh.svc.term_set_fg(15);
        return;
    }

    sh.svc.term_set_fg(14);
    sh.svc.term_write("WARNING: This will erase all data on drive ");
    sh.svc.term_write(drive);
    sh.svc.term_writeln("!");
    sh.svc.term_set_fg(15);
    sh.svc.term_write("Are you sure? (y/n): ");

    loop {
        let c = sh.svc.kbd_getchar();
        if c == b'y' as i32 || c == b'Y' as i32 {
            sh.svc.term_putchar(b'y');
            sh.svc.term_writeln("");
            break;
        }
        if c == b'n' as i32 || c == b'N' as i32 {
            sh.svc.term_putchar(b'n');
            sh.svc.term_writeln("");
            sh.svc.term_writeln("Format cancelled.");
            return;
        }
    }

    sh.svc.term_write("Formatting drive ");
    sh.svc.term_write(drive);
    sh.svc.term_writeln("...");

    let mut ok = false;
    if letter == b'A' {
        sh.svc.fs_ramdisk_init();
        sh.svc.fs_ramdisk_format_fat32();
        // try_set_drive will be called later if needed
        ok = true;
    } else if letter == b'C' {
        let total = if sh.svc.ahci_is_ready() {
            sh.svc.ahci_get_sector_count()
        } else {
            0
        };
        if total == 0 {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("No AHCI disk found for C:");
            sh.svc.term_set_fg(15);
            return;
        }
        sh.svc.fs_format(total);
        sh.svc.fs_reinit();
        ok = true;
    }

    if ok {
        sh.svc.term_set_fg(10);
        sh.svc.term_write("Drive ");
        sh.svc.term_write(drive);
        sh.svc.term_writeln(" formatted successfully as FAT32.");
    } else {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Format failed.");
    }
    sh.svc.term_set_fg(15);
}
