#![no_std]

mod parser;
mod builtins;
mod filesystem;
mod commands;
mod login;
mod prompt;

extern crate lumie_std;

pub struct LumieDirEnt {
    pub name: [u8; 256],
    pub is_dir: bool,
    pub size: u32,
}

pub trait ShellServices {
    fn term_write(&self, s: &str);
    fn term_writeln(&self, s: &str);
    fn term_clear(&self, bg: u32);
    fn term_set_fg(&self, c: u32);
    fn term_set_bg(&self, c: u32);
    fn term_set_pos(&self, x: i32, y: i32);
    fn term_get_width(&self) -> i32;
    fn term_get_height(&self) -> i32;
    fn term_putchar(&self, c: u8);

    fn kbd_getchar(&self) -> i32;
    fn kbd_kbhit(&self) -> i32;

    fn fs_exists(&self, path: &str) -> bool;
    fn fs_list_dir(&self, path: &str, entries: &mut [LumieDirEnt]) -> i32;
    fn fs_mkdir(&self, path: &str) -> i32;
    fn fs_delete(&self, path: &str) -> i32;
    fn fs_read_file(&self, path: &str, buf: &mut [u8]) -> i32;
    fn fs_write_file(&self, path: &str, data: &[u8]) -> i32;
    fn fs_get_file_size(&self, path: &str) -> i32;
    fn fs_format(&self, total_sectors: u64);
    fn fs_set_drive(&self, drive_letter: char);
    fn fs_get_current_drive(&self) -> char;
    fn fs_try_set_drive(&mut self, letter: char) -> bool;
    fn fs_use_ahci(&self);
    fn fs_reinit(&self);
    fn fs_ramdisk_init(&self);
    fn fs_ramdisk_format_fat32(&self);

    fn get_time(&self, buf: &mut [u8]) -> i32;

    fn sched_get_count(&self) -> i32;
    fn sched_get_name(&self, id: i32, buf: &mut [u8]) -> i32;
    fn sched_get_state(&self, id: i32) -> i32;
    fn sched_get_priority(&self, id: i32) -> u8;

    fn reboot(&self);
    fn shutdown(&self);

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

    fn mouse_poll(&self, ms: &mut MouseState) -> bool;
    fn mouse_restore(&self, x: i32, y: i32);
    fn mouse_draw(&self, x: i32, y: i32);
    fn mouse_get_pos(&self, x: &mut i32, y: &mut i32);
    fn mouse_set_visible(&self, v: bool);

    fn editor_run(&self, path: &str);
    fn gop_get_fb(&self) -> FramebufferInfo;
    fn mm_get_free_mem(&self) -> u64;

    fn disk_enum_all(&self) -> i32;
    fn disk_get_info(&self, id: i32, info: &mut DiskInfo) -> i32;
    fn disk_get_drive_letter(&self, id: i32) -> u8;

    fn ahci_is_ready(&self) -> bool;
    fn ahci_get_sector_count(&self) -> u64;

    fn net_init(&self) -> i32;
    fn net_renet_download(&self, name: Option<&str>);

    fn extract_gzip_tar(&self, file: Option<&str>);

    fn lc_compile_file(&self, ctx: &mut LcCtx, path: &str) -> i32;
    fn lc_output_sys(&self, ctx: &LcCtx, path: &str, mod_name: &str) -> i32;

    fn pcspkr_beep(&self, freq: u32, dur: u32);
    fn drvcheck_run_scan(&self);

    fn bootcache_clear(&self);
    fn bootcache_count(&self) -> i32;
    fn bootcache_load(&self, lines: &mut [[u8; 256]], max: i32) -> i32;

    fn sys_load(&self, path: &str, bi: &SysBootInfo, mod_out: &mut SysModule) -> i32;
    fn lumie_get_kernel_image(&self, base: &mut *const u8, size: &mut u32) -> i32;
    fn lumie_pack_module(&self, data: &[u8], magic: u32, subtype: u32, name: &str, packed: &mut *mut u8, packed_sz: &mut u32) -> i32;

    fn desktop_init(&self) -> i32;
    fn desktop_run(&self);
    fn setup_gui_run(&self);
    fn lumie_edit(&self, path: &str);

    fn pe_check(&self, buf: &[u8]) -> bool;
    fn pe_type(&self, buf: &[u8]) -> Option<&str>;
    fn pe_machine_str(&self, buf: &[u8]) -> Option<&str>;
}

pub struct Shell<'a> {
    pub svc: &'a dyn ShellServices,
    pub cwd: [u8; 256],
    pub cwd_len: usize,
    pub mouse_visible: bool,
    pub current_drive: u8,
}

impl<'a> Shell<'a> {
    pub fn new(svc: &'a dyn ShellServices) -> Self {
        let mut cwd = [0u8; 256];
        cwd[0] = b'/';
        Shell {
            svc,
            cwd,
            cwd_len: 1,
            mouse_visible: false,
            current_drive: b'C',
        }
    }

    pub fn resolve_path(&self, input: &str, output: &mut [u8]) -> usize {
        let cwd_str = core::str::from_utf8(&self.cwd[..self.cwd_len]).unwrap_or("/");
        let cwd_bytes = cwd_str.as_bytes();

        if input.is_empty() {
            let n = cwd_bytes.len().min(output.len() - 1);
            output[..n].copy_from_slice(&cwd_bytes[..n]);
            output[n] = 0;
            return n;
        }

        if input.as_bytes()[0] == b'/' {
            let ib = input.as_bytes();
            let n = ib.len().min(output.len() - 1);
            output[..n].copy_from_slice(&ib[..n]);
            output[n] = 0;
            return n;
        }

        let mut len = cwd_bytes.len().min(output.len() - 1);
        output[..len].copy_from_slice(&cwd_bytes[..len]);
        output[len] = 0;

        if len > 0 && output[len - 1] != b'/' {
            if len < output.len() - 1 {
                output[len] = b'/';
                output[len + 1] = 0;
                len += 1;
            }
        }

        let ib = input.as_bytes();
        let ilen = ib.len().min(output.len() - 1 - len);
        if ilen > 0 {
            output[len..len + ilen].copy_from_slice(&ib[..ilen]);
        }
        len += ilen;
        output[len] = 0;
        len
    }

    pub fn set_cwd(&mut self, path: &[u8]) {
        let n = path.len().min(255);
        self.cwd[..n].copy_from_slice(&path[..n]);
        self.cwd[n] = 0;
        self.cwd_len = n;
        if self.cwd_len == 0 || self.cwd[0] == 0 {
            self.cwd[0] = b'/';
            self.cwd[1] = 0;
            self.cwd_len = 1;
        }
    }

    pub fn cwd_str(&self) -> &str {
        core::str::from_utf8(&self.cwd[..self.cwd_len]).unwrap_or("/")
    }

    fn str_contains(a: &str, b: &str) -> bool {
        let a_bytes = a.as_bytes();
        let b_bytes = b.as_bytes();
        let al = a_bytes.len();
        let bl = b_bytes.len();
        if bl == 0 || bl > al {
            return false;
        }
        for i in 0..=al - bl {
            let mut ok = true;
            for j in 0..bl {
                let mut ca = a_bytes[i + j];
                let mut cb = b_bytes[j];
                if ca >= b'A' && ca <= b'Z' {
                    ca += 32;
                }
                if cb >= b'A' && cb <= b'Z' {
                    cb += 32;
                }
                if ca != cb {
                    ok = false;
                    break;
                }
            }
            if ok {
                return true;
            }
        }
        false
    }

    pub fn confirm_action(&self, msg: &str) -> bool {
        self.svc.term_write(msg);
        self.svc.term_write(" (y/n): ");
        loop {
            let c = self.svc.kbd_getchar();
            if c == b'y' as i32 || c == b'Y' as i32 {
                self.svc.term_writeln("y");
                return true;
            }
            if c == b'n' as i32 || c == b'N' as i32 {
                self.svc.term_writeln("n");
                return false;
            }
        }
    }

    pub fn lumieos_installed(&self) -> bool {
        self.svc.fs_exists("/system/kernel.lkrn")
    }

    pub fn check_system_files(&self, base_dir: &str) -> i32 {
        let required = ["kernel.lkrn", "kbd.ldrv", "fs.ldrv", "mouse.ldrv", "shell.lsh"];
        for f in &required {
            let mut path_buf = [0u8; 128];
            let blen = base_dir.len().min(100);
            path_buf[..blen].copy_from_slice(base_dir.as_bytes());
            let mut len = blen;
            if len > 0 && path_buf[len - 1] != b'/' {
                path_buf[len] = b'/';
                len += 1;
            }
            let fbytes = f.as_bytes();
            path_buf[len..len + fbytes.len()].copy_from_slice(fbytes);
            len += fbytes.len();
            path_buf[len] = 0;
            let p = core::str::from_utf8(&path_buf[..len]).unwrap_or("");
            if !self.svc.fs_exists(p) {
                return -1;
            }
        }
        0
    }

    pub fn try_set_drive(&mut self, letter: u8) -> bool {
        if letter == b'A' {
            self.current_drive = b'A';
            self.svc.fs_ramdisk_init();
            self.svc.fs_set_drive('A');
            if !self.svc.fs_exists("/") {
                self.svc.fs_ramdisk_format_fat32();
                self.svc.fs_reinit();
            }
            self.set_cwd(b"/");
            return true;
        }
        if letter == b'C' {
            self.current_drive = b'C';
            self.svc.fs_set_drive('C');
            self.svc.fs_reinit();
            return true;
        }
        false
    }

    pub fn read_timezone(&self) -> i32 {
        if !self.svc.fs_exists("/system/timezone.cfg") {
            return 0;
        }
        let mut tz_buf = [0u8; 16];
        let n = self.svc.fs_read_file("/system/timezone.cfg", &mut tz_buf);
        if n > 0 {
            let tz_str = core::str::from_utf8(&tz_buf[..n as usize]).unwrap_or("0");
            parse_int(tz_str).unwrap_or(0)
        } else {
            0
        }
    }

    fn match_name(pattern: &str, name: &str) -> bool {
        let plen = pattern.len();
        let nlen = name.len();
        if plen == 0 {
            return false;
        }
        let pbytes = pattern.as_bytes();

        if pbytes[0] == b'*' && pbytes[plen - 1] == b'*' {
            if plen <= 2 {
                return true;
            }
            let mid = &pattern[1..plen - 1];
            return name.contains(mid);
        }

        if pbytes[0] == b'*' {
            if nlen < plen - 1 {
                return false;
            }
            return &name[nlen - (plen - 1)..] == &pattern[1..];
        }

        if pbytes[plen - 1] == b'*' {
            return name.starts_with(&pattern[..plen - 1]);
        }

        name == pattern
    }

    pub fn run(&mut self) {
        login::do_login(self);
        self.main_loop();
    }

    fn main_loop(&mut self) {
        let term_bg = if self.current_drive == b'A' { 0 } else { 1 };
        self.svc.term_clear(term_bg);
        self.svc.term_set_bg(term_bg);
        self.svc.term_set_fg(15);

        self.svc.reg_init();

        let mut _line_buf = [0u8; 4096];
        let mut show_prompt = true;

        {
            let mut panic_val = [0u8; 8];
            if self.svc.reg_get("KERNELPANICLOL", &mut panic_val) > 0 && &panic_val[..1] == b"1" {
                self.svc.term_clear(4);
                self.svc.term_set_bg(4);
                self.svc.term_set_fg(15);
                let half_w = self.svc.term_get_width() / 2;
                let half_h = self.svc.term_get_height() / 2;
                self.svc.term_set_pos(half_w - 12, half_h - 4);
                self.svc.term_set_fg(14);
                self.svc.term_writeln("  ==============================");
                self.svc.term_set_pos(half_w - 12, half_h - 3);
                self.svc.term_set_fg(15);
                self.svc.term_writeln("  |                            |");
                self.svc.term_set_pos(half_w - 12, half_h - 2);
                self.svc.term_set_fg(11);
                self.svc.term_writeln("  |    KERNEL PANIC LOL!       |");
                self.svc.term_set_pos(half_w - 12, half_h - 1);
                self.svc.term_set_fg(15);
                self.svc.term_writeln("  |                            |");
                self.svc.term_set_pos(half_w - 12, half_h);
                self.svc.term_set_fg(10);
                self.svc.term_writeln("  |   You found the easter     |");
                self.svc.term_set_pos(half_w - 12, half_h + 1);
                self.svc.term_set_fg(10);
                self.svc.term_writeln("  |   egg! System goes BOOM!   |");
                self.svc.term_set_pos(half_w - 12, half_h + 2);
                self.svc.term_set_fg(15);
                self.svc.term_writeln("  |                            |");
                self.svc.term_set_pos(half_w - 12, half_h + 3);
                self.svc.term_set_fg(14);
                self.svc.term_writeln("  ==============================");
                self.svc.term_set_pos(half_w - 14, half_h + 6);
                self.svc.term_set_fg(15);
                self.svc.term_write("Press any key to reboot...");
                self.svc.kbd_getchar();
                self.svc.reg_set("KERNELPANICLOL", "0");
                self.svc.reboot();
                return;
            }
        }

        let mut start_path = [0u8; 128];
        if self.svc.reg_get_start(&mut start_path) > 0 {
            let sp = core::str::from_utf8(&start_path).unwrap_or("").trim_end_matches('\0');
            if !sp.is_empty() {
                if self.check_system_files(sp) != 0 {
                    self.svc.term_set_fg(14);
                    self.svc.term_writeln("Warning: Some system files are missing.");
                    self.svc.term_write("Check '");
                    self.svc.term_write(sp);
                    self.svc.term_writeln("' for required files.");
                    self.svc.term_set_fg(10);
                    self.svc.term_write("Run 'setup' to reinstall, or 'regedit' to fix Start path.");
                    self.svc.term_set_fg(15);
                    self.svc.term_writeln("");
                }
            }
        }

        self.svc.term_set_fg(11);
        self.svc.term_writeln("LumieOS v0.1 - Windows Edition");
        self.svc.term_set_fg(15);
        let mut welcome = [0u8; 64];
        let mut name_buf = [0u8; 64];
        let has_name = self.svc.users_current_name(&mut name_buf);
        let wname = if has_name > 0 {
            core::str::from_utf8(&name_buf[..has_name as usize]).unwrap_or("unknown")
        } else if self.current_drive == b'A' {
            "SYSTEM"
        } else {
            "unknown"
        };
        welcome[..7].copy_from_slice(b"Welcome");
        welcome[7] = b',';
        welcome[8] = b' ';
        let wlen = 9 + wname.len().min(54);
        welcome[9..9 + wname.len().min(54)].copy_from_slice(wname.as_bytes());
        self.svc.term_writeln(core::str::from_utf8(&welcome[..wlen]).unwrap_or("Welcome!"));
        self.svc.term_writeln("Type 'help' for commands.");
        let mut tbuf = [0u8; 64];
        if self.svc.get_time(&mut tbuf) > 0 {
            self.svc.term_set_fg(14);
            self.svc.term_write("Current time: ");
            self.svc.term_set_fg(15);
            let ts = core::str::from_utf8(&tbuf).unwrap_or("");
            self.svc.term_writeln(ts.trim_end_matches('\0'));
        }
        self.svc.term_writeln("");

        loop {
            let mut ms = MouseState::default();
            if self.svc.mouse_poll(&mut ms) {
                if self.mouse_visible {
                    self.svc.mouse_restore(ms.x - ms.dx, ms.y - ms.dy);
                }
                self.svc.mouse_draw(ms.x, ms.y);
                self.mouse_visible = true;
            } else if self.mouse_visible {
                let (mut mx, mut my) = (0, 0);
                self.svc.mouse_get_pos(&mut mx, &mut my);
                self.svc.mouse_restore(mx, my);
                self.svc.mouse_draw(mx, my);
                self.mouse_visible = true;
            }

            if show_prompt {
                prompt::render_prompt(self);
            }
            show_prompt = true;

            let mut line_pos: usize = 0;
            _line_buf = [0u8; 4096];

            loop {
                let c = self.svc.kbd_getchar();
                if c == b'\n' as i32 {
                    self.svc.term_writeln("");
                    break;
                }
                if c == b'\x08' as i32 {
                    if line_pos > 0 {
                        line_pos -= 1;
                        self.svc.term_putchar(b'\x08');
                        self.svc.term_putchar(b' ');
                        self.svc.term_putchar(b'\x08');
                    }
                    continue;
                }
                if c == 0x1B {
                    line_pos = 0;
                    _line_buf = [0u8; 4096];
                    self.svc.term_writeln("^C");
                    show_prompt = true;
                    break;
                }
                if c >= b' ' as i32 && c <= b'~' as i32 && line_pos < 4095 {
                    _line_buf[line_pos] = c as u8;
                    line_pos += 1;
                    self.svc.term_putchar(c as u8);
                }
            }

            if line_pos == 0 {
                continue;
            }

            let pr = parser::shell_parse(&mut _line_buf[..line_pos + 1]);
            if pr.argc == 0 {
                continue;
            }

            fn arg_slice<'a>(buf: &'a [u8], start: usize, len: usize) -> &'a [u8] {
                if len == 0 { return b""; }
                &buf[start..start + len]
            }
            let cmd0 = arg_slice(&_line_buf, pr.argv_start[0], pr.argv_len[0]);

            macro_rules! arg {
                ($i:expr) => {{
                    let i = $i;
                    if i < pr.argc { Some(arg_slice(&_line_buf, pr.argv_start[i], pr.argv_len[i])) } else { None }
                }}
            }
            macro_rules! arg_str {
                ($i:expr) => {{
                    arg!($i).map(|a| core::str::from_utf8(a).unwrap_or(""))
                }}
            }

            if cmd0.len() == 2 && cmd0[1] == b':' {
                let letter = if cmd0[0] >= b'a' && cmd0[0] <= b'z' {
                    cmd0[0] - 32
                } else {
                    cmd0[0]
                };
                if letter == b'A' || letter == b'C' {
                    self.try_set_drive(letter);
                    continue;
                }
                self.svc.term_set_fg(12);
                self.svc.term_write("Unknown drive: ");
                let cmd_s = core::str::from_utf8(cmd0).unwrap_or("");
                self.svc.term_writeln(cmd_s);
                self.svc.term_set_fg(15);
                continue;
            }

            match cmd0 {
                b"help" | b"?" => builtins::cmd_help(self),
                b"clear" | b"cls" => builtins::cmd_clear(self),
                b"ls" | b"dir" => filesystem::cmd_ls(self, arg!(1)),
                b"cd" => filesystem::cmd_cd(self, arg!(1)),
                b"pwd" => builtins::cmd_pwd(self),
                b"cat" | b"type" => filesystem::cmd_cat(self, arg!(1)),
                b"rm" | b"del" => filesystem::cmd_rm(self, arg!(1)),
                b"rmdir" => filesystem::cmd_rmdir(self, arg!(1)),
                b"mkdir" => filesystem::cmd_mkdir(self, arg!(1)),
                b"echo" => builtins::cmd_echo(self, arg!(1)),
                b"info" => builtins::cmd_info(self),
                b"ver" => builtins::cmd_ver(self),
                b"time" => builtins::cmd_time(self),
                b"timezone" => builtins::cmd_timezone(self, arg!(1)),
                b"reboot" => {
                    if self.confirm_action("Reboot system?") {
                        self.svc.reboot();
                    }
                }
                b"shutdown" => {
                    if self.confirm_action("Shutdown system?") {
                        self.svc.shutdown();
                    }
                }
                b"whoami" => builtins::cmd_whoami(self),
                b"su" | b"login" => builtins::cmd_su(self),
                b"adduser" => builtins::cmd_adduser(self, arg!(1), arg!(2)),
                b"passwd" => builtins::cmd_passwd(self, arg!(1)),
                b"regedit" => builtins::cmd_regedit(self),
                b"notepad" => builtins::cmd_notepad(self),
                b"disks" => builtins::cmd_disks(self),
                b"ps" => builtins::cmd_ps(self),
                b"beep" => builtins::cmd_beep(
                    self,
                    arg!(1).and_then(|a| parse_int(core::str::from_utf8(a).unwrap_or("440"))).unwrap_or(440i32) as u32,
                    arg!(2).and_then(|a| parse_int(core::str::from_utf8(a).unwrap_or("200"))).unwrap_or(200i32) as u32,
                ),
                b"edit" => commands::cmd_edit(self, arg!(1)),
                b"setup" | b"install" => {
                    if self.current_drive != b'A' && self.svc.users_current_role() != crate::USER_ROLE_ADMIN {
                        self.svc.term_set_fg(12);
                        self.svc.term_writeln("Only administrators can run setup/install.");
                        self.svc.term_set_fg(15);
                    } else {
                        self.svc.setup_gui_run();
                    }
                }
                b"wher" => filesystem::cmd_wher(self, arg!(1), arg!(2)),
                b"wher1" => filesystem::cmd_wher1(self, arg!(1)),
                b"format" => filesystem::cmd_format(self, arg!(1)),
                b"lumiec" => commands::cmd_lumiec(self, arg!(1), arg!(2)),
                b"drvcheck" => self.svc.drvcheck_run_scan(),
                b"bootcache" => {
                    let mut arg_slices: [&[u8]; parser::MAX_ARGS] = [b""; parser::MAX_ARGS];
                    for i in 0..pr.argc {
                        arg_slices[i] = arg_slice(&_line_buf, pr.argv_start[i], pr.argv_len[i]);
                    }
                    commands::cmd_bootcache(self, &arg_slices[..pr.argc]);
                }
                b"sysload" => commands::cmd_sysload(self, arg!(1)),
                b"desktop" => {
                    self.svc.desktop_init();
                    self.svc.desktop_run();
                }
                b"extract" => {
                    self.svc.extract_gzip_tar(arg_str!(1));
                }
                b"renet" => {
                    if self.svc.net_init() != 0 {
                        self.svc.term_set_fg(12);
                        self.svc.term_writeln("Network not available");
                        self.svc.term_set_fg(15);
                    } else {
                        self.svc.net_renet_download(arg_str!(1));
                    }
                }
                _ => {
                    let cmd_str = core::str::from_utf8(cmd0).unwrap_or("");
                    let mut fpath = [0u8; 256];
                    self.resolve_path(cmd_str, &mut fpath);
                    let fpath_str = core::str::from_utf8(&fpath).unwrap_or("").trim_end_matches('\0');
                    let fsize = self.svc.fs_get_file_size(fpath_str);
                    if fsize > 0 {
                        let mut fbuf = [0u8; 65536];
                        let nread = self.svc.fs_read_file(fpath_str, &mut fbuf);
                        if nread == fsize {
                            if self.svc.pe_check(&fbuf[..nread as usize]) {
                                let pt = self.svc.pe_type(&fbuf[..nread as usize]);
                                let arch = self.svc.pe_machine_str(&fbuf[..nread as usize]);
                                if let Some(pt) = pt {
                                    self.svc.term_set_fg(14);
                                    self.svc.term_write("PE ");
                                    self.svc.term_write(pt);
                                    self.svc.term_write(" detected: ");
                                    self.svc.term_write(fpath_str);
                                    if let Some(arch) = arch {
                                        self.svc.term_write(" (");
                                        self.svc.term_write(arch);
                                        self.svc.term_write(")");
                                    }
                                    self.svc.term_set_fg(15);
                                    self.svc.term_writeln("");

                                    if pt == "EXE" {
                                        self.svc.term_set_fg(11);
                                        self.svc.term_writeln("Attempting to load PE executable...");
                                        self.svc.term_set_fg(15);
                                        self.svc.term_writeln("PE file wrapped as .lsh app, but native PE execution not yet supported.");
                                    }
                                }
                                continue;
                            }
                        }
                    }
                    self.svc.term_set_fg(12);
                    self.svc.term_write("Unknown command: ");
                    self.svc.term_writeln(cmd_str);
                    self.svc.term_set_fg(15);
                }
            }
        }
    }
}

fn parse_int(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (neg, start) = if s.as_bytes().get(0) == Some(&b'-') {
        (true, 1)
    } else {
        (false, 0)
    };
    let mut val: i32 = 0;
    for &c in s.as_bytes()[start..].iter() {
        if c < b'0' || c > b'9' {
            return None;
        }
        val = val.wrapping_mul(10).wrapping_add((c - b'0') as i32);
    }
    if neg {
        Some(-val)
    } else {
        Some(val)
    }
}

pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
}

impl Default for MouseState {
    fn default() -> Self {
        MouseState {
            x: 0,
            y: 0,
            dx: 0,
            dy: 0,
            buttons: 0,
        }
    }
}

pub struct FramebufferInfo {
    pub base: u64,
    pub width: i32,
    pub height: i32,
    pub pitch: i32,
    pub size: u32,
}

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

pub struct SysBootInfo {
    pub version: u32,
    pub gop_fb_base: u64,
    pub gop_width: i32,
    pub gop_height: i32,
    pub gop_pitch: i32,
}

#[derive(Default)]
pub struct SysModule {
    pub entry: Option<fn(&SysBootInfo, &mut *mut u8) -> i32>,
    pub base: u64,
    pub size: u32,
}

#[derive(Default)]
pub struct LcCtx {
    pub errors: i32,
    pub code_pos: i32,
}

pub const USER_ROLE_USER: i32 = 0;
pub const USER_ROLE_ADMIN: i32 = 1;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
