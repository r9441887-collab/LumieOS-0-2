use crate::Shell;

pub fn cmd_per(sh: &Shell, arg1: Option<&[u8]>, arg2: Option<&[u8]>) {
    let text = match arg1 {
        Some(t) => t,
        None => {
            sh.svc.term_set_fg(11);
            sh.svc.term_writeln("per - encode anything to binary/x64");
            sh.svc.term_set_fg(15);
            sh.svc.term_writeln("Usage:");
            sh.svc.term_writeln("  per <text>           - show binary representation");
            sh.svc.term_writeln("  per <text> <file>    - save binary to file");
            sh.svc.term_writeln("  per --hex <hex>      - decode hex to binary");
            sh.svc.term_writeln("  per --file <path>    - read file and show binary");
            sh.svc.term_writeln("");
            sh.svc.term_set_fg(11);
            sh.svc.term_writeln("Examples:");
            sh.svc.term_writeln("  per Hello");
            sh.svc.term_writeln("  per Привет");
            sh.svc.term_writeln("  per --hex 48656c6c6f");
            sh.svc.term_writeln("  per Hello output.bin");
            sh.svc.term_set_fg(15);
            return;
        }
    };

    let text_str = core::str::from_utf8(text).unwrap_or("");

    if text_str == "--hex" {
        if let Some(hex_bytes) = arg2 {
            let hex_str = core::str::from_utf8(hex_bytes).unwrap_or("");
            let decoded = hex_decode(hex_str);
            if decoded.is_empty() {
                sh.svc.term_set_fg(12);
                sh.svc.term_writeln("Invalid hex string.");
                sh.svc.term_set_fg(15);
                return;
            }
            sh.svc.term_set_fg(11);
            sh.svc.term_write("Hex decoded (");
            let mut sz_buf = [0u8; 16];
            let sz_n = crate::builtins::write_int(&mut sz_buf, decoded.len() as i32);
            sh.svc.term_write(core::str::from_utf8(&sz_buf[..sz_n]).unwrap_or("0"));
            sh.svc.term_writeln(" bytes):");
            sh.svc.term_set_fg(15);
            print_hex_dump(sh, &decoded);
            return;
        }
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Usage: per --hex <hex_string>");
        sh.svc.term_set_fg(15);
        return;
    }

    if text_str == "--file" {
        if let Some(path) = arg2 {
            let path_str = core::str::from_utf8(path).unwrap_or("");
            let mut resolved = [0u8; 256];
            sh.resolve_path(path_str, &mut resolved);
            let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');
            let fsize = sh.svc.fs_get_file_size(resolved_str);
            if fsize <= 0 {
                sh.svc.term_set_fg(12);
                sh.svc.term_write("File not found: ");
                sh.svc.term_writeln(resolved_str);
                sh.svc.term_set_fg(15);
                return;
            }
            let mut buf = [0u8; 32768];
            let read_size = if fsize as usize > buf.len() { buf.len() } else { fsize as usize };
            let n = sh.svc.fs_read_file(resolved_str, &mut buf[..read_size]);
            if n <= 0 {
                sh.svc.term_set_fg(12);
                sh.svc.term_writeln("Failed to read file.");
                sh.svc.term_set_fg(15);
                return;
            }
            sh.svc.term_set_fg(11);
            sh.svc.term_write("File: ");
            sh.svc.term_write(resolved_str);
            sh.svc.term_write(" (");
            let mut sz_buf = [0u8; 16];
            let sz_n = crate::builtins::write_int(&mut sz_buf, n);
            sh.svc.term_write(core::str::from_utf8(&sz_buf[..sz_n]).unwrap_or("0"));
            sh.svc.term_writeln(" bytes)");
            sh.svc.term_set_fg(15);
            print_hex_dump(sh, &buf[..n as usize]);
            return;
        }
        sh.svc.term_set_fg(12);
        sh.svc.term_writeln("Usage: per --file <path>");
        sh.svc.term_set_fg(15);
        return;
    }

    let bytes = text;

    sh.svc.term_set_fg(11);
    sh.svc.term_write("Input: ");
    sh.svc.term_set_fg(15);
    sh.svc.term_write(text_str);
    sh.svc.term_set_fg(11);
    sh.svc.term_write(" (");
    let mut sz_buf = [0u8; 16];
    let sz_n = crate::builtins::write_int(&mut sz_buf, bytes.len() as i32);
    sh.svc.term_write(core::str::from_utf8(&sz_buf[..sz_n]).unwrap_or("0"));
    sh.svc.term_writeln(" bytes)");
    sh.svc.term_set_fg(15);

    sh.svc.term_writeln("");

    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("Binary (x86-64 encoding):");
    sh.svc.term_set_fg(15);
    print_hex_dump(sh, bytes);

    sh.svc.term_writeln("");

    sh.svc.term_set_fg(11);
    sh.svc.term_writeln("x86-64 instructions (UTF-8 data):");
    sh.svc.term_set_fg(15);
    print_x64_asm(sh, bytes);

    if let Some(file_arg) = arg2 {
        let file_str = core::str::from_utf8(file_arg).unwrap_or("");
        if !file_str.is_empty() {
            let mut resolved = [0u8; 256];
            sh.resolve_path(file_str, &mut resolved);
            let resolved_str = core::str::from_utf8(&resolved).unwrap_or("").trim_end_matches('\0');
            let rc = sh.svc.fs_write_file(resolved_str, bytes);
            if rc == 0 {
                sh.svc.term_set_fg(10);
                sh.svc.term_write("Saved to: ");
                sh.svc.term_writeln(resolved_str);
            } else {
                sh.svc.term_set_fg(12);
                sh.svc.term_write("Failed to write: ");
                sh.svc.term_writeln(resolved_str);
            }
            sh.svc.term_set_fg(15);
        }
    }
}

fn hex_decode(hex: &str) -> [u8; 1024] {
    let mut out = [0u8; 1024];
    let hex_clean: [u8; 2048] = {
        let mut tmp = [0u8; 2048];
        let mut n = 0;
        for &b in hex.as_bytes() {
            if b != b' ' && b != b'\n' && b != b'\r' && b != b'\t' {
                if n < 2048 {
                    tmp[n] = b;
                    n += 1;
                }
            }
        }
        tmp
    };
    let mut i = 0;
    let mut pos = 0;
    let len = hex.len();
    let hb = hex.as_bytes();
    while i + 1 < len && pos < 1024 {
        let hi = hex_digit(hb[i]);
        let lo = hex_digit(hb[i + 1]);
        if hi < 16 && lo < 16 {
            out[pos] = (hi << 4) | lo;
            pos += 1;
        }
        i += 2;
    }
    out
}

fn hex_digit(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 16,
    }
}

fn print_hex_dump(sh: &Shell, data: &[u8]) {
    let mut offset: u32 = 0;
    let mut i = 0;
    while i < data.len() {
        let mut line = [0u8; 80];
        let mut p = 0;

        let mut obuf = [0u8; 8];
        let on = crate::builtins::write_int_hex(&mut obuf, offset);
        for _ in on..8 {
            if p < 79 { line[p] = b'0'; p += 1; }
        }
        for j in 0..on {
            if p < 79 { line[p] = obuf[j]; p += 1; }
        }
        if p < 79 { line[p] = b' '; p += 1; }
        if p < 79 { line[p] = b' '; p += 1; }

        for j in 0..16 {
            if i + j < data.len() {
                let b = data[i + j];
                let hex_chars = b"0123456789ABCDEF";
                if p < 79 { line[p] = hex_chars[(b >> 4) as usize]; p += 1; }
                if p < 79 { line[p] = hex_chars[(b & 0xF) as usize]; p += 1; }
            } else {
                if p < 79 { line[p] = b' '; p += 1; }
                if p < 79 { line[p] = b' '; p += 1; }
            }
            if j == 7 {
                if p < 79 { line[p] = b' '; p += 1; }
                if p < 79 { line[p] = b' '; p += 1; }
            } else {
                if p < 79 { line[p] = b' '; p += 1; }
            }
        }

        if p < 79 { line[p] = b' '; p += 1; }
        if p < 79 { line[p] = b'|'; p += 1; }

        for j in 0..16 {
            if i + j < data.len() {
                let b = data[i + j];
                let c = if b >= 0x20 && b <= 0x7E { b } else { b'.' };
                if p < 79 { line[p] = c; p += 1; }
            }
        }

        if p < 79 { line[p] = b'|'; p += 1; }

        sh.svc.term_writeln(core::str::from_utf8(&line[..p]).unwrap_or(""));
        offset = offset.wrapping_add(16);
        i += 16;
    }
}

fn print_x64_asm(sh: &Shell, data: &[u8]) {
    sh.svc.term_writeln("  ; Data as x86-64 bytes in memory");
    sh.svc.term_writeln("  ; Each byte is an addressable value");

    let mut i = 0;
    while i < data.len() {
        let mut line = [0u8; 64];
        let mut p = 0;

        let prefix = b"  ; +";
        for &c in prefix { if p < 63 { line[p] = c; p += 1; } }

        let mut obuf = [0u8; 8];
        let on = crate::builtins::write_int(&mut obuf, i as i32);
        for j in 0..on {
            if p < 63 { line[p] = obuf[j]; p += 1; }
        }

        let sep = b": ";
        for &c in sep { if p < 63 { line[p] = c; p += 1; } }

        let end = (i + 8).min(data.len());
        for j in i..end {
            let b = data[j];
            let hex_chars = b"0123456789ABCDEF";
            if p < 63 { line[p] = b'0'; p += 1; }
            if p < 63 { line[p] = hex_chars[(b >> 4) as usize]; p += 1; }
            if p < 63 { line[p] = hex_chars[(b & 0xF) as usize]; p += 1; }
            if p < 63 { line[p] = b'h'; p += 1; }
            if j + 1 < end { if p < 63 { line[p] = b','; p += 1; } }
            if p < 63 { line[p] = b' '; p += 1; }
        }

        sh.svc.term_writeln(core::str::from_utf8(&line[..p]).unwrap_or(""));
        i += 8;
    }

    sh.svc.term_writeln("");
    sh.svc.term_set_fg(14);
    sh.svc.term_write("Size: ");
    let mut sz_buf = [0u8; 16];
    let sz_n = crate::builtins::write_int(&mut sz_buf, data.len() as i32);
    sh.svc.term_write(core::str::from_utf8(&sz_buf[..sz_n]).unwrap_or("0"));
    sh.svc.term_writeln(" bytes");
    sh.svc.term_set_fg(15);
}
