use core::arch::asm;
use core::arch::naked_asm;

use crate::mm::heap;
use crate::console;
use crate::fs;
use crate::gfx;
use crate::sched;

pub const SYSCALL_EXIT: u64 = 0;
pub const SYSCALL_TERM_WRITE: u64 = 1;
pub const SYSCALL_TERM_CLEAR: u64 = 2;
pub const SYSCALL_TERM_SET_FG: u64 = 3;
pub const SYSCALL_TERM_SET_BG: u64 = 4;
pub const SYSCALL_TERM_SET_POS: u64 = 5;
pub const SYSCALL_TERM_PUTCHAR: u64 = 6;
pub const SYSCALL_TERM_GET_WIDTH: u64 = 7;
pub const SYSCALL_TERM_GET_HEIGHT: u64 = 8;
pub const SYSCALL_KBD_GETCHAR: u64 = 9;
pub const SYSCALL_KBD_KBHIT: u64 = 10;
pub const SYSCALL_FS_READ: u64 = 11;
pub const SYSCALL_FS_WRITE: u64 = 12;
pub const SYSCALL_FS_EXISTS: u64 = 13;
pub const SYSCALL_FS_LIST: u64 = 14;
pub const SYSCALL_FS_MKDIR: u64 = 15;
pub const SYSCALL_MALLOC: u64 = 16;
pub const SYSCALL_FREE: u64 = 17;
pub const SYSCALL_STALL: u64 = 18;
pub const SYSCALL_REBOOT: u64 = 19;
pub const SYSCALL_SHUTDOWN: u64 = 20;
pub const SYSCALL_GPU_FILL_RECT: u64 = 21;
pub const SYSCALL_GPU_PUT_PIXEL: u64 = 22;
pub const SYSCALL_GPU_FLIP: u64 = 23;
pub const SYSCALL_GPU_VSYNC: u64 = 24;
pub const SYSCALL_SCHED_YIELD: u64 = 25;
pub const SYSCALL_SCHED_COUNT: u64 = 26;
pub const SYSCALL_SCHED_NAME: u64 = 27;
pub const SYSCALL_GET_TIME: u64 = 28;
pub const SYSCALL_MEM_FREE: u64 = 29;
pub const SYSCALL_MAX: u64 = 32;

#[repr(C, align(16))]
struct CpuScratch {
    user_rsp: u64,
    kernel_rsp: u64,
}

static mut CPU_SCRATCH: CpuScratch = CpuScratch {
    user_rsp: 0,
    kernel_rsp: 0,
};

static mut SYSCALL_KSTACK: [u8; 8192] = [0u8; 8192];

pub fn syscall_rust_entry(
    no: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> u64 {
    match no {
        SYSCALL_EXIT => {
            unsafe { sched::sched_exit(); }
            0
        }
        SYSCALL_TERM_WRITE => {
            unsafe { console::term_write(arg1 as *const u8); }
            0
        }
        SYSCALL_TERM_CLEAR => {
            unsafe { console::term_clear(arg1 as u32); }
            0
        }
        SYSCALL_TERM_SET_FG => {
            unsafe { console::term_set_fg(arg1 as u32); }
            0
        }
        SYSCALL_TERM_SET_BG => {
            unsafe { console::term_set_bg(arg1 as u32); }
            0
        }
        SYSCALL_TERM_SET_POS => {
            unsafe { console::term_set_pos(arg1 as i32, arg2 as i32); }
            0
        }
        SYSCALL_TERM_PUTCHAR => {
            unsafe { console::term_putchar(arg1 as u8); }
            0
        }
        SYSCALL_TERM_GET_WIDTH => {
            console::term_get_width() as u64
        }
        SYSCALL_TERM_GET_HEIGHT => {
            console::term_get_height() as u64
        }
        SYSCALL_KBD_GETCHAR => {
            crate::lumie_getchar() as u64
        }
        SYSCALL_KBD_KBHIT => {
            crate::lumie_kbhit() as u64
        }
        SYSCALL_FS_READ => {
            unsafe { fs::read_file(arg1 as *const u8, arg2 as *mut u8, arg3 as u32) as u64 }
        }
        SYSCALL_FS_WRITE => {
            unsafe { fs::write_file(arg1 as *const u8, arg2 as *const u8, arg3 as u32) as u64 }
        }
        SYSCALL_FS_EXISTS => {
            let ok = unsafe { fs::exists(arg1 as *const u8) };
            if ok { 1 } else { 0 }
        }
        SYSCALL_FS_LIST => {
            unsafe {
                fs::list_dir(
                    arg1 as *const u8,
                    arg2 as *mut crate::fs::types::LumieDirEnt,
                    arg3 as i32,
                ) as u64
            }
        }
        SYSCALL_MALLOC => {
            unsafe { heap::kmalloc(arg1) as u64 }
        }
        SYSCALL_FREE => {
            unsafe { heap::kfree(arg1 as *mut u8); }
            0
        }
        SYSCALL_STALL => {
            crate::lumie_stall(arg1);
            0
        }
        SYSCALL_REBOOT => {
            crate::lumie_reboot();
            0
        }
        SYSCALL_SHUTDOWN => {
            crate::lumie_shutdown();
            0
        }
        SYSCALL_SCHED_YIELD => {
            unsafe { sched::sched_yield(); }
            0
        }
        SYSCALL_SCHED_COUNT => {
            sched::sched_get_count() as u64
        }
        SYSCALL_SCHED_NAME => {
            let id = arg1 as i32;
            let buf = arg2 as *mut u8;
            if let Some(name) = sched::sched_get_name(id) {
                let bytes = name.as_bytes();
                let max = arg3 as usize;
                let len = bytes.len().min(max - 1);
                unsafe {
                    core::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, len);
                    *buf.add(len) = 0;
                }
            }
            0
        }
        SYSCALL_GET_TIME => {
            let buf = unsafe { core::slice::from_raw_parts_mut(arg1 as *mut u8, arg2 as usize) };
            crate::lumie_get_time(buf, arg2 as i32) as u64
        }
        SYSCALL_FS_MKDIR => {
            unsafe { fs::mkdir(arg1 as *const u8) as u64 }
        }
        SYSCALL_GPU_FILL_RECT => {
            unsafe { gfx::gfx_fill_rect(arg1 as u32, arg2 as u32, arg3 as u32, arg4 as u32, arg5 as u32); }
            0
        }
        SYSCALL_GPU_PUT_PIXEL => {
            unsafe { gfx::gfx_put_pixel(arg1 as u32, arg2 as u32, arg3 as u32); }
            0
        }
        SYSCALL_GPU_FLIP => {
            unsafe { gfx::gfx_flip(); }
            0
        }
        SYSCALL_GPU_VSYNC => {
            unsafe { gfx::gfx_vsync(); }
            0
        }
        SYSCALL_MEM_FREE => {
            crate::mm::paging::get_page_stack_top() * 4096
        }
        _ => 0xFFFF_FFFF_FFFF_FFFFu64,
    }
}

unsafe fn wrmsr(msr: u32, val: u64) {
    let lo = (val & 0xFFFF_FFFF) as u32;
    let hi = ((val >> 32) & 0xFFFF_FFFF) as u32;
    asm!("wrmsr", in("ecx") msr, in("eax") lo, in("edx") hi);
}

pub unsafe fn syscall_init() {
    CPU_SCRATCH.kernel_rsp = SYSCALL_KSTACK.as_mut_ptr().add(8192) as u64;
    let scratch = &CPU_SCRATCH as *const CpuScratch as u64;
    wrmsr(0xC0000101, scratch);

    wrmsr(0xC0000082, syscall_entry as *const () as u64);

    let star: u64 = ((0x00000008u64) << 48) | ((0x00000008u64) << 32);
    wrmsr(0xC0000081, star);

    wrmsr(0xC0000084, 0x3C7FD5u64);
}

#[unsafe(naked)]
pub unsafe extern "C" fn syscall_entry() {
    naked_asm!(
        "swapgs",
        "mov gs:0x0, rsp",
        "mov rsp, gs:0x8",
        "push qword ptr gs:0x0",
        "push rcx",
        "push r11",
        "push r15",
        "push r14",
        "push r13",
        "push r12",
        "push rbp",
        "push rbx",
        "push r10",
        "push r9",
        "push r8",
        "push rdi",
        "push rsi",
        "push rdx",
        "push rcx",
        "push rax",
        "mov rdi, rax",
        "mov rcx, r10",
        "call {0}",
        "mov r15, rax",
        "pop rax",
        "pop rcx",
        "pop rsi",
        "pop rdi",
        "pop r8",
        "pop r9",
        "pop r10",
        "pop rbx",
        "pop rbp",
        "pop r12",
        "pop r13",
        "pop r14",
        "pop r15",
        "pop r11",
        "pop rcx",
        "mov rsp, qword ptr gs:0x0",
        "swapgs",
        "mov rax, r15",
        "sysretq",
        sym syscall_rust_entry,
    );
}
