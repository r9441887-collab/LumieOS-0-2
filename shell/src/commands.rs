use crate::{Shell, builtins::write_int, LcCtx};

pub fn cmd_edit(sh: &Shell, file: Option<&[u8]>) {
    match file {
        Some(f) => {
            let f_str = core::str::from_utf8(f).unwrap_or("");
            let mut resolved = [0u8; 256];
            sh.resolve_path(f_str, &mut resolved);
            let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');
            sh.svc.lumie_edit(resolved_str);
        }
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: edit <filename>");
            sh.svc.term_set_fg(15);
        }
    }
}

#[allow(dead_code)]
pub fn cmd_setup(sh: &Shell) {
    if sh.lumieos_installed() {
        if !sh.confirm_action("LumieOS is already installed. Reinstall?") {
            return;
        }
    }

    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("Installing LumieOS...");
    sh.svc.term_set_fg(15);

    if !sh.svc.fs_exists("/system") {
        sh.svc.fs_mkdir("/system");
    }
    if !sh.svc.fs_exists("/drivers") {
        sh.svc.fs_mkdir("/drivers");
    }

    let mut count = 0;
    let mut ok = 0;

    let mut kbase: *const u8 = core::ptr::null();
    let mut ksize: u32 = 0;
    if sh.svc.lumie_get_kernel_image(&mut kbase, &mut ksize) == 0 && !kbase.is_null() {
        let mut packed: *mut u8 = core::ptr::null_mut();
        let mut packed_sz: u32 = 0;
        let kdata = unsafe { core::slice::from_raw_parts(kbase, ksize as usize) };
        if sh.svc.lumie_pack_module(kdata, LUMIE_MAGIC_LKRN, 0, "LumieOS Kernel", &mut packed, &mut packed_sz) == 0 && !packed.is_null() {
            let pdata = unsafe { core::slice::from_raw_parts(packed, packed_sz as usize) };
            sh.svc.fs_write_file("/system/kernel.lkrn", pdata);
            ok += 1;
        }
        count += 1;
        sh.svc.term_write("  Kernel: ");
        sh.svc.term_set_fg(if ok > 0 { 10 } else { 12 });
        sh.svc.term_writeln(if ok > 0 { "[OK]" } else { "[FAIL]" });
        sh.svc.term_set_fg(15);
    }

    sh.svc.term_set_fg(11);
    let mut summary = [0u8; 64];
    let n = write_int(&mut summary, ok);
    sh.svc.term_write(core::str::from_utf8(&summary[..n]).unwrap_or("0"));
    sh.svc.term_write(" of ");
    let n = write_int(&mut summary, count);
    sh.svc.term_write(core::str::from_utf8(&summary[..n]).unwrap_or("0"));
    sh.svc.term_writeln(" files installed successfully.");
    sh.svc.term_set_fg(15);

    if ok == count && count > 0 {
        sh.svc.term_set_fg(10);
        sh.svc.term_writeln("Installation complete! Timezone can be set with 'timezone' command.");
    } else {
        sh.svc.term_set_fg(14);
        sh.svc.term_writeln("Installation completed with some errors.");
    }
    sh.svc.term_set_fg(15);
}

#[allow(dead_code)]
const LUMIE_MAGIC_LKRN: u32 = 0x4E524B4C;
#[allow(dead_code)]
const LUMIE_MAGIC_LSH: u32 = 0x484C534C;
#[allow(dead_code)]
const LUMIE_MAGIC_LDRV: u32 = 0x5652444C;

pub fn cmd_lumiec(sh: &Shell, src_path: Option<&[u8]>, out_path_arg: Option<&[u8]>) {
    let src_path = match src_path {
        Some(s) => core::str::from_utf8(s).unwrap_or(""),
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: lumiec <input.lc> [output.sys]");
            sh.svc.term_set_fg(15);
            return;
        }
    };

    let out_path: &str;
    let mut path_buf = [0u8; 256];
    if let Some(out) = out_path_arg {
        let out_str = core::str::from_utf8(out).unwrap_or("");
        path_buf[..out_str.len()].copy_from_slice(out_str.as_bytes());
        path_buf[out_str.len()] = 0;
        out_path = core::str::from_utf8(&path_buf[..out_str.len()]).unwrap_or("/system/output.sys");
    } else {
        path_buf[..src_path.len()].copy_from_slice(src_path.as_bytes());
        let len = src_path.len();
        if len > 3
            && path_buf[len - 3] == b'.'
            && (path_buf[len - 2] == b'l' || path_buf[len - 2] == b'L')
            && (path_buf[len - 1] == b'c' || path_buf[len - 1] == b'C')
        {
            path_buf[len - 2] = b's';
            path_buf[len - 1] = b'y';
            out_path = core::str::from_utf8(&path_buf[..len]).unwrap_or("/system/output.sys");
        } else {
            let default = b"/system/output.sys";
            path_buf[..default.len()].copy_from_slice(default);
            out_path = core::str::from_utf8(&path_buf[..default.len()]).unwrap_or("/system/output.sys");
        }
    }

    let mut ctx = LcCtx::default();

    sh.svc.term_set_fg(11);
    sh.svc.term_write("LumieC: compiling ");
    sh.svc.term_write(src_path);
    sh.svc.term_writeln("...");
    sh.svc.term_set_fg(15);

    let ret = sh.svc.lc_compile_file(&mut ctx, src_path);
    if ret != 0 {
        sh.svc.term_set_fg(12);
        let mut err = [0u8; 32];
        let n = write_int(&mut err, ctx.errors);
        sh.svc.term_write("Compilation failed with ");
        sh.svc.term_write(core::str::from_utf8(&err[..n]).unwrap_or("0"));
        sh.svc.term_writeln(" error(s)");
        sh.svc.term_set_fg(15);
        return;
    }

    sh.svc.term_set_fg(10);
    let mut sz = [0u8; 16];
    let n = write_int(&mut sz, ctx.code_pos);
    sh.svc.term_write("Code size: ");
    sh.svc.term_write(core::str::from_utf8(&sz[..n]).unwrap_or("0"));
    sh.svc.term_writeln(" bytes");
    sh.svc.term_set_fg(15);

    let mut mod_name = [0u8; 32];
    let mut mod_len = 0;
    for &b in src_path.as_bytes() {
        if mod_len < 31 {
            mod_name[mod_len] = b;
            mod_len += 1;
        }
    }
    for i in (0..mod_len).rev() {
        if mod_name[i] == b'/' || mod_name[i] == b'\\' {
            let remaining = mod_len - i - 1;
            for j in 0..remaining {
                mod_name[j] = mod_name[i + 1 + j];
            }
            mod_len = remaining;
            break;
        }
    }
    for i in 0..mod_len {
        if mod_name[i] == b'.' {
            mod_name[i] = 0;
            mod_len = i;
            break;
        }
    }
    let mn = core::str::from_utf8(&mod_name[..mod_len]).unwrap_or("module");

    if sh.svc.lc_output_sys(&ctx, out_path, mn) == 0 {
        sh.svc.term_set_fg(10);
        sh.svc.term_write("Output: ");
        sh.svc.term_writeln(out_path);
        sh.svc.term_set_fg(10);
        sh.svc.term_write("To load: sysload ");
        sh.svc.term_writeln(out_path);
    } else {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Failed to write output file");
    }
    sh.svc.term_set_fg(15);
}

#[allow(dead_code)]
pub fn cmd_drvcheck(sh: &Shell) {
    sh.svc.drvcheck_run_scan();
}

pub fn cmd_bootcache(sh: &Shell, args: &[&[u8]]) {
    if args.len() > 1 && args[1] == b"clear" {
        sh.svc.bootcache_clear();
        sh.svc.term_set_fg(10);
        sh.svc.term_writeln("Boot cache cleared.");
        sh.svc.term_set_fg(15);
    } else if args.len() > 1 && args[1] == b"count" {
        let cnt = sh.svc.bootcache_count();
        let mut cbuf = [0u8; 16];
        let n = write_int(&mut cbuf, cnt);
        sh.svc.term_write("Boot cache entries: ");
        sh.svc.term_writeln(core::str::from_utf8(&cbuf[..n]).unwrap_or("0"));
    } else if args.len() > 1 && args[1] == b"show" {
        let mut lines = [[0u8; 256]; 64];
        let n = sh.svc.bootcache_load(&mut lines, 64);
        if n > 0 {
            for i in 0..n as usize {
                let line = core::str::from_utf8(&lines[i]).unwrap_or("").trim_end_matches('\0');
                sh.svc.term_writeln(line);
            }
        } else {
            sh.svc.term_writeln("Boot cache is empty.");
        }
    } else {
        sh.svc.term_writeln("Usage: bootcache [clear|count|show]");
    }
}

pub fn cmd_sysload(sh: &Shell, path: Option<&[u8]>) {
    match path {
        Some(p) => {
            let p_str = core::str::from_utf8(p).unwrap_or("");
            let mut resolved = [0u8; 256];
            sh.resolve_path(p_str, &mut resolved);
            let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');

            let fb = sh.svc.gop_get_fb();
            let bi = crate::SysBootInfo {
                version: 1,
                gop_fb_base: fb.base,
                gop_width: fb.width,
                gop_height: fb.height,
                gop_pitch: fb.pitch,
            };
            let mut mod_out = crate::SysModule::default();

            if sh.svc.sys_load(resolved_str, &bi, &mut mod_out) == 0 && mod_out.entry.is_some() {
                sh.svc.term_set_fg(10);
                sh.svc.term_write("Loaded: ");
                sh.svc.term_writeln(resolved_str);
            } else {
                sh.svc.term_set_fg(12);
                sh.svc.term_writeln("Failed to load module");
            }
        }
        None => {
            sh.svc.term_set_fg(12);
            sh.svc.term_writeln("Usage: sysload <path.sys>");
        }
    }
    sh.svc.term_set_fg(15);
}

#[allow(dead_code)]
pub fn cmd_desktop(sh: &Shell) {
    sh.svc.desktop_init();
    sh.svc.desktop_run();
}

#[allow(dead_code)]
pub fn cmd_extract(sh: &Shell, file: Option<&[u8]>) {
    sh.svc.extract_gzip_tar(file.map(|f| core::str::from_utf8(f).unwrap_or("")));
}

#[allow(dead_code)]
pub fn cmd_renet(sh: &Shell, name: Option<&[u8]>) {
    if sh.svc.net_init() != 0 {
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Network not available");
        sh.svc.term_set_fg(15);
    } else {
        sh.svc.net_renet_download(name.map(|n| core::str::from_utf8(n).unwrap_or("")));
    }
}
