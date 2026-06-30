pub const MAX_TASKS: usize = 32;
pub const TASK_USER_PRIORITY: u8 = 0;
pub const TASK_SYSTEM_PRIORITY: u8 = 1;
pub const TASK_USER_TICKS: i32 = 1;
pub const TASK_SYSTEM_TICKS: i32 = 100;

pub const TASK_RUNNING: i32 = 0;
pub const TASK_READY: i32 = 1;
pub const TASK_BLOCKED: i32 = 2;
pub const TASK_DEAD: i32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Task {
    pub id: i32,
    pub name: [u8; 32],
    pub rsp: u64,
    pub rsp0: u64,
    pub stack: [u8; 8192],
    pub priority: u8,
    pub state: i32,
    pub ticks_left: i32,
}

impl Task {
    const fn new() -> Self {
        Self {
            id: 0,
            name: [0u8; 32],
            rsp: 0,
            rsp0: 0,
            stack: [0u8; 8192],
            priority: 0,
            state: 0,
            ticks_left: 0,
        }
    }
}

pub struct Scheduler {
    pub tasks: [Task; MAX_TASKS],
    pub num_tasks: i32,
    pub current_task: i32,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: [Task::new(); MAX_TASKS],
            num_tasks: 0,
            current_task: -1,
        }
    }
}
