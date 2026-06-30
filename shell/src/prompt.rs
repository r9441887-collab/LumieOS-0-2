use crate::Shell;

pub fn render_prompt(sh: &Shell) {
    let mut name_buf = [0u8; 64];
    let n = sh.svc.users_current_name(&mut name_buf);
    let uname = if n > 0 {
        core::str::from_utf8(&name_buf[..n as usize]).unwrap_or("user")
    } else {
        "user"
    };

    sh.svc.term_set_fg(11);
    sh.svc.term_write(uname);
    sh.svc.term_set_fg(15);
    sh.svc.term_write("@");
    sh.svc.term_set_fg(11);
    sh.svc.term_write("lumieos");
    sh.svc.term_set_fg(15);
    sh.svc.term_write(":");

    sh.svc.term_set_fg(11);
    sh.svc.term_putchar(sh.current_drive);
    sh.svc.term_write(sh.cwd_str());
    sh.svc.term_set_fg(15);
    sh.svc.term_write("$ ");
}
