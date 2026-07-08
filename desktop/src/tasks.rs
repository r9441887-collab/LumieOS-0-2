use crate::widgets::Window;

#[derive(Clone, Copy)]
pub struct Task {
    pub name: [u8; 32],
    pub name_len: usize,
    pub icon_color: u32,
    pub open: bool,
    pub window_idx: i32,
}

impl Task {
    pub fn new(name: &str, icon_color: u32) -> Self {
        let mut n = [0u8; 32];
        let len = name.len().min(31);
        n[..len].copy_from_slice(&name.as_bytes()[..len]);
        Task {
            name: n,
            name_len: len,
            icon_color,
            open: false,
            window_idx: -1,
        }
    }

    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }
}

pub fn find_open_task(tasks: &[Task]) -> i32 {
    for i in (0..tasks.len()).rev() {
        if tasks[i].open {
            return i as i32;
        }
    }
    -1
}

pub fn switch_to_task(tasks: &mut [Task], windows: &mut [Window], idx: usize) {
    if idx >= tasks.len() {
        return;
    }
    let task = &tasks[idx];
    if !task.open {
        return;
    }
    if task.window_idx >= 0 && (task.window_idx as usize) < windows.len() {
        let win_idx = task.window_idx as usize;
        if windows[win_idx].open {
            for w in windows.iter_mut() {
                w.active = false;
            }
            windows[win_idx].active = true;
        }
    }
}
