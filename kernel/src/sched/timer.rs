use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::arch::pic;
use crate::sched::task::{TASK_READY, TASK_RUNNING, TASK_SYSTEM_PRIORITY, TASK_SYSTEM_TICKS, TASK_USER_TICKS};

static G_TICKS: AtomicU64 = AtomicU64::new(0);

pub unsafe fn pit_set_sched_mode(hz: u32) {
    let mut hz = hz;
    if hz == 0 {
        hz = 1000;
    }
    let mut divisor = 1193182u32 / hz;
    if divisor < 2 {
        divisor = 2;
    }
    if divisor > 65535 {
        divisor = 65535;
    }

    pic::pic_writeb(0x43, 0x34);
    pic::pic_writeb(0x40, (divisor & 0xFF) as u8);
    pic::pic_writeb(0x40, ((divisor >> 8) & 0xFF) as u8);
}

#[no_mangle]
pub unsafe extern "C" fn sched_tick_handler(rsp: u64) -> u64 {
    let tasks = &mut *core::ptr::addr_of_mut!(super::SCHED);
    let current_task = tasks.current_task;
    let num_tasks = tasks.num_tasks;

    if current_task >= 0 && current_task < num_tasks {
        tasks.tasks[current_task as usize].rsp = rsp;
        if tasks.tasks[current_task as usize].ticks_left > 0 {
            tasks.tasks[current_task as usize].ticks_left -= 1;
        }
    }

    G_TICKS.fetch_add(1, Ordering::Relaxed);

    if current_task >= 0 && current_task < num_tasks {
        let ct = current_task as usize;
        if tasks.tasks[ct].ticks_left > 0
            && (tasks.tasks[ct].state == TASK_RUNNING || tasks.tasks[ct].state == TASK_READY)
        {
            return tasks.tasks[ct].rsp;
        }
    }

    let mut next = if current_task < 0 { 0 } else { current_task };
    let mut attempts = 0;
    let max_attempts = {
        let m = num_tasks * 4 + 1;
        if m < 4 { 4 } else { m }
    };

    while attempts < max_attempts {
        next = if next >= 0 {
            let nn = num_tasks;
            (next + 1) % if nn != 0 { nn } else { 1 }
        } else {
            0
        };
        if tasks.tasks[next as usize].state == TASK_RUNNING
            || tasks.tasks[next as usize].state == TASK_READY
        {
            tasks.tasks[next as usize].ticks_left = if tasks.tasks[next as usize].priority == TASK_SYSTEM_PRIORITY {
                TASK_SYSTEM_TICKS
            } else {
                TASK_USER_TICKS
            };
            if next != current_task {
                break;
            }
        }
        attempts += 1;
    }

    tasks.current_task = next;
    tasks.tasks[next as usize].rsp
}

#[naked]
pub unsafe extern "C" fn pit_isr() {
    asm!(
        "push rax",
        "push rcx",
        "push rdx",
        "push rbx",
        "push rbp",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rdi, rsp",
        "call {0}",
        "mov rsp, rax",
        "mov al, 0x20",
        "out 0x20, al",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rbp",
        "pop rbx",
        "pop rdx",
        "pop rcx",
        "pop rax",
        "iretq",
        sym sched_tick_handler,
        options(noreturn),
    );
}

#[naked]
pub unsafe extern "C" fn stub_isr() {
    asm!(
        "push rax",
        "mov al, 0x20",
        "out 0x20, al",
        "out 0xA0, al",
        "pop rax",
        "iretq",
        options(noreturn),
    );
}

pub extern "C" fn idle_task() {
    loop {
        unsafe { asm!("hlt"); }
    }
}
