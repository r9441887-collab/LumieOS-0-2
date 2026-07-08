#[allow(dead_code)]
pub const LINE_BUF_SIZE: usize = 4096;
pub const MAX_ARGS: usize = 64;

pub struct ParseResult {
    pub argv_start: [usize; MAX_ARGS],
    pub argv_len: [usize; MAX_ARGS],
    pub argc: usize,
}

pub fn shell_parse(line: &mut [u8]) -> ParseResult {
    let mut argv_start = [0usize; MAX_ARGS];
    let mut argv_len = [0usize; MAX_ARGS];
    let mut argc = 0usize;
    let mut in_word = false;
    let mut start = 0;

    for i in 0..line.len() {
        if line[i] == 0 {
            break;
        }
        if line[i] == b' ' || line[i] == b'\t' || line[i] == b'\n' {
            line[i] = 0;
            if in_word {
                if argc < MAX_ARGS {
                    argv_start[argc] = start;
                    argv_len[argc] = i - start;
                    argc += 1;
                }
                in_word = false;
            }
        } else {
            if !in_word {
                start = i;
                in_word = true;
            }
        }
    }

    if in_word {
        if argc < MAX_ARGS {
            let end = line.len();
            argv_start[argc] = start;
            argv_len[argc] = end - start;
            argc += 1;
        }
    }

    ParseResult { argv_start, argv_len, argc }
}
