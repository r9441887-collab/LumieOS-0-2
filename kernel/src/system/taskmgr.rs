use crate::console::terminal;
use crate::drivers::keyboard;
use crate::sched;
use crate::system::disk_io;

#[repr(C)]
pub struct TaskInfo {
    pub name: [u8; 32],
    pub state: i32,
    pub priority: u8,
}

const RUNNING: i32 = 0;
const READY: i32 = 1;
const BLOCKED: i32 = 2;
const DEAD: i32 = 3;

fn state_name(state: i32) -> &'static [u8] {
    match state {
        0 => b"RUNNING",
        1 => b"READY  ",
        2 => b"BLOCKED",
        3 => b"DEAD   ",
        _ => b"UNKNOWN",
    }
}

unsafe fn draw_header() {
    let w = terminal::term_get_width();
    terminal::term_set_fg(0xFFFFFF);
    terminal::term_set_bg(0x0000AA);
    for x in 0..w {
        terminal::term_set_pos(x, 0);
        terminal::term_putchar(b' ');
        terminal::term_set_pos(x, 1);
        terminal::term_putchar(b' ');
    }
    terminal::term_set_pos(2, 0);
    terminal::term_set_fg(0xFFFF00);
    terminal::term_write(b"LumieOS Task Manager\0" as *const u8);
    terminal::term_set_pos(2, 1);
    terminal::term_set_fg(0x55FFFF);
    terminal::term_write(b"[R]efresh  [Q]uit\0" as *const u8);

    terminal::term_set_bg(0x000000);
    terminal::term_set_fg(0x444444);
    for x in 0..w {
        terminal::term_set_pos(x, 2);
        terminal::term_putchar(b'-');
    }
}

unsafe fn draw_processes(start_row: i32, max_rows: i32) {
    let cnt = sched::get_count();
    let cnt = if cnt < 0 { 0 } else if cnt > max_rows - 1 { max_rows - 1 } else { cnt };

    let mut row = start_row;
    terminal::term_set_pos(2, row);
    row += 1;
    terminal::term_set_fg(0x00FF00);
    terminal::term_write(b"Processes:\0" as *const u8);
    row += 1;

    if cnt == 0 {
        terminal::term_set_pos(4, row);
        terminal::term_set_fg(0x444444);
        terminal::term_write(b"(none)\0" as *const u8);
        return;
    }

    for i in 0..cnt {
        let name = sched::get_name(i);
        let state = sched::get_state(i);
        let prio = sched::get_priority(i);

        terminal::term_set_pos(4, row);
        terminal::term_set_fg(0xFFFFFF);

        if !name.is_null() {
            let mut ni = 0;
            while *name.add(ni) != 0 && ni < 20 {
                terminal::term_putchar(*name.add(ni));
                ni += 1;
            }
            for _ in ni..20 {
                terminal::term_putchar(b' ');
            }
        } else {
            for _ in 0..20 {
                terminal::term_putchar(b' ');
            }
        }

        terminal::term_set_fg(if state == RUNNING { 0x00FF00 } else { 0x444444 });
        let sn = state_name(state);
        for &c in sn {
            terminal::term_putchar(c);
        }
        terminal::term_putchar(b' ');

        let mut pbuf: [u8; 8] = [0u8; 8];
        crate::system::util::lumie_itoa(prio as i64, pbuf.as_mut_ptr(), 10);
        terminal::term_set_fg(0x55FFFF);
        terminal::term_write(b"Prio:\0" as *const u8);
        terminal::term_write(pbuf.as_ptr());
        row += 1;
    }
}

unsafe fn draw_memory(start_row: i32) {
    terminal::term_set_pos(2, start_row);
    terminal::term_set_fg(0x00FF00);
    terminal::term_write(b"Memory: (API not available)\0" as *const u8);
}

unsafe fn draw_disks(start_row: i32) {
    let dc = disk_io::disk_enum_all();
    let dc = if dc < 0 { 0 } else { dc };

    terminal::term_set_pos(2, start_row);
    terminal::term_set_fg(0x00FF00);
    terminal::term_write(b"Disks:\0" as *const u8);
    let start_row2 = start_row + 1;

    if dc == 0 {
        terminal::term_set_pos(4, start_row2);
        terminal::term_set_fg(0x444444);
        terminal::term_write(b"(none)\0" as *const u8);
        return;
    }

    let mut row = start_row2;
    for i in 0..dc {
        if i >= 4 { break; }
        let info = disk_io::disk_get_info(i);
        terminal::term_set_pos(4, row);
        terminal::term_set_fg(0xFFFFFF);
        if !info.is_null() {
            let mut buf: [u8; 128] = [0u8; 128];
            let mut pos: usize = 0;
            crate::system::util::lumie_itoa(i as i64, buf[pos..].as_mut_ptr(), 10);
            while buf[pos] != 0 { pos += 1; }
            buf[pos] = b':';
            pos += 1;
            buf[pos] = b' ';
            pos += 1;
            let name_len = crate::system::util::lumie_strlen_raw(&(*info).name);
            buf[pos..pos + name_len].copy_from_slice(&(*info).name[..name_len]);
            pos += name_len;
            buf[pos] = b' ';
            pos += 1;
            buf[pos] = b' ';
            pos += 1;
            crate::system::util::lumie_itoa((*info).sector_count as i64, buf[pos..].as_mut_ptr(), 10);
            while buf[pos] != 0 { pos += 1; }
            let sec = b" sectors";
            buf[pos..pos + 8].copy_from_slice(sec);
            pos += 8;
            buf[pos] = 0;
            terminal::term_write(buf.as_ptr());
        } else {
            let mut buf: [u8; 16] = [0u8; 16];
            crate::system::util::lumie_itoa(i as i64, buf.as_mut_ptr(), 10);
            terminal::term_write(b"Disk \0" as *const u8);
            terminal::term_write(buf.as_ptr());
        }
        row += 1;
    }
}

pub unsafe fn taskmgr_run() {
    let mut w = terminal::term_get_width();
    let mut rows = terminal::term_get_height();
    if w < 40 { w = 40; }
    if rows < 15 { rows = 15; }

    terminal::term_clear(0x000000);

    loop {
        draw_header();
        draw_processes(4, rows - 12);
        draw_memory(rows - 7);
        draw_disks(rows - 4);

        terminal::term_set_fg(0x444444);
        terminal::term_set_bg(0x000000);
        terminal::term_set_pos(0, rows - 1);
        for x in 0..w {
            terminal::term_putchar(b' ');
        }
        terminal::term_set_pos(0, rows - 1);

        let key = keyboard::getchar();
        if key == b'q' as i32 || key == b'Q' as i32 {
            break;
        }
    }

    terminal::term_clear(0x000000);
}

pub unsafe fn taskmgr_init() {
}

pub unsafe fn taskmgr_list_tasks(buf: *mut TaskInfo, max: i32) -> i32 {
    if buf.is_null() || max <= 0 {
        return 0;
    }
    let cnt = sched::get_count();
    let cnt = if cnt < 0 { 0 } else if cnt > max { max } else { cnt };
    for i in 0..cnt {
        let task = &mut *buf.add(i as usize);
        let name_ptr = sched::get_name(i);
        task.name.fill(0);
        if !name_ptr.is_null() {
            let mut ni = 0;
            while *name_ptr.add(ni) != 0 && ni < 31 {
                task.name[ni] = *name_ptr.add(ni);
                ni += 1;
            }
        }
        task.state = sched::get_state(i);
        task.priority = sched::get_priority(i);
    }
    cnt
}
