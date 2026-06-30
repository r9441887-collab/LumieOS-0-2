pub mod task;
pub mod timer;

use core::arch::asm;

use crate::arch::gdt;
use crate::arch::idt;
use crate::arch::pic;
use crate::sched::task::*;
use crate::sched::timer::*;

pub static mut SCHED: Scheduler = Scheduler::new();

pub unsafe fn sched_init() {
    gdt::gdt_init();
    idt::idt_init();

    pic::pic_remap(0x20, 0x28);

    pit_set_sched_mode(1000);

    let cs_sel: u16 = 0x08;
    let gate_attr: u8 = 0x8E;

    idt::idt_set_entry(0x20, pit_isr as u64, cs_sel, gate_attr);

    for i in 0..256usize {
        if i != 0x20 {
            idt::idt_set_entry(i, stub_isr as u64, cs_sel, gate_attr);
        }
    }

    pic::pic_unmask_irq(0);

    core::ptr::write_bytes(
        &mut SCHED.tasks as *mut [Task; MAX_TASKS] as *mut u8,
        0,
        core::mem::size_of::<[Task; MAX_TASKS]>(),
    );

    sched_create_task("idle", idle_task, TASK_SYSTEM_PRIORITY);

    let shell_id = SCHED.num_tasks;
    SCHED.num_tasks += 1;
    SCHED.tasks[shell_id as usize].id = shell_id;
    let name_bytes = b"shell\0";
    let mut i = 0;
    while i < 31 && name_bytes[i] != 0 {
        SCHED.tasks[shell_id as usize].name[i] = name_bytes[i];
        i += 1;
    }
    SCHED.tasks[shell_id as usize].name[i] = 0;
    SCHED.tasks[shell_id as usize].priority = TASK_SYSTEM_PRIORITY;
    SCHED.tasks[shell_id as usize].state = TASK_RUNNING;
    SCHED.tasks[shell_id as usize].ticks_left = TASK_SYSTEM_TICKS;
    let cur_rsp: u64;
    asm!("mov {}, rsp", out(reg) cur_rsp, options(nostack));
    SCHED.tasks[shell_id as usize].rsp = cur_rsp;

    SCHED.current_task = shell_id;

    asm!("sti");
}

pub unsafe fn sched_create_task(name: &str, entry: extern "C" fn(), priority: u8) -> i32 {
    if SCHED.num_tasks as usize >= MAX_TASKS {
        return -1;
    }

    let id = SCHED.num_tasks as usize;
    SCHED.num_tasks += 1;
    SCHED.tasks[id].id = id as i32;

    let name_bytes = name.as_bytes();
    let max_len = 31usize.min(name_bytes.len());
    SCHED.tasks[id].name[..max_len].copy_from_slice(&name_bytes[..max_len]);
    SCHED.tasks[id].name[max_len] = 0;

    SCHED.tasks[id].priority = priority;
    SCHED.tasks[id].state = TASK_READY;
    SCHED.tasks[id].ticks_left = if priority == TASK_SYSTEM_PRIORITY {
        TASK_SYSTEM_TICKS
    } else {
        TASK_USER_TICKS
    };

    core::ptr::write_bytes(
        SCHED.tasks[id].stack.as_mut_ptr(),
        0,
        SCHED.tasks[id].stack.len(),
    );

    let stack_top = SCHED.tasks[id].stack.as_mut_ptr().add(8192) as *mut u64;
    let mut sp = stack_top;
    sp = sp.sub(1);
    *sp = 0x202; // RFLAGS (IF=1)
    sp = sp.sub(1);
    *sp = 0x08; // CS
    sp = sp.sub(1);
    *sp = entry as u64; // RIP
    for _ in 0..15 {
        sp = sp.sub(1);
        *sp = 0;
    }
    SCHED.tasks[id].rsp = sp as u64;
    id as i32
}

pub unsafe fn sched_exit() {
    let ct = SCHED.current_task;
    if ct >= 0 && (ct as usize) < MAX_TASKS {
        SCHED.tasks[ct as usize].state = TASK_DEAD;
    }
    asm!("cli", "hlt", options(noreturn));
}

pub fn sched_get_current() -> i32 {
    unsafe { SCHED.current_task }
}

pub fn sched_get_count() -> i32 {
    unsafe { SCHED.num_tasks }
}

pub fn sched_get_name(id: i32) -> Option<&'static str> {
    unsafe {
        if id < 0 || id >= SCHED.num_tasks {
            return None;
        }
        let mut len = 0usize;
        while len < 32 && SCHED.tasks[id as usize].name[len] != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(SCHED.tasks[id as usize].name.as_ptr(), len);
        Some(core::str::from_utf8_unchecked(slice))
    }
}

pub fn sched_get_state(id: i32) -> i32 {
    unsafe {
        if id < 0 || id >= SCHED.num_tasks {
            return -1;
        }
        SCHED.tasks[id as usize].state
    }
}

pub unsafe fn sched_yield() {
    asm!("int 0x20");
}

pub fn sched_get_priority(id: i32) -> u8 {
    unsafe {
        if id < 0 || id >= SCHED.num_tasks {
            return 0;
        }
        SCHED.tasks[id as usize].priority
    }
}
