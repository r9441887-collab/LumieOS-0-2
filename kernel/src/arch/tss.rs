use crate::arch::gdt;

#[repr(C, packed)]
struct TaskStateSegment {
    _reserved1: u32,
    pub rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    _reserved2: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    _reserved3: u64,
    _reserved4: u16,
    iopb_offset: u16,
}

const TSS_SIZE: u32 = core::mem::size_of::<TaskStateSegment>() as u32;

static mut TSS: TaskStateSegment = TaskStateSegment {
    _reserved1: 0,
    rsp0: 0,
    rsp1: 0,
    rsp2: 0,
    _reserved2: 0,
    ist1: 0,
    ist2: 0,
    ist3: 0,
    ist4: 0,
    ist5: 0,
    ist6: 0,
    ist7: 0,
    _reserved3: 0,
    _reserved4: 0,
    iopb_offset: TSS_SIZE as u16,
};

static mut KERNEL_STACK: [u8; 16384] = [0u8; 16384];

pub unsafe fn tss_init() {
    let stack_top = KERNEL_STACK.as_mut_ptr().add(16384) as u64;
    TSS.rsp0 = stack_top;
    TSS.iopb_offset = TSS_SIZE as u16;

    let tss_addr = &TSS as *const TaskStateSegment as u64;
    gdt::gdt_set_tss(tss_addr, TSS_SIZE);
    gdt::gdt_load_tr();
}

pub unsafe fn tss_set_rsp0(rsp: u64) {
    TSS.rsp0 = rsp;
}
