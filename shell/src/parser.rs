pub const LINE_BUF_SIZE: usize = 4096;
pub const MAX_ARGS: usize = 64;

pub fn shell_parse(line: &mut [u8]) -> Vec<&[u8]> {
    let mut argv: Vec<&[u8]> = Vec::with_capacity(MAX_ARGS);
    let mut in_word = false;
    let mut start = 0;

    for i in 0..line.len() {
        if line[i] == 0 {
            break;
        }
        if line[i] == b' ' || line[i] == b'\t' || line[i] == b'\n' {
            line[i] = 0;
            if in_word {
                argv.push(&line[start..i]);
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
        let end = line.len();
        argv.push(&line[start..end]);
    }

    argv
}
