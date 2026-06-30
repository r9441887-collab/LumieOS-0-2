use core::arch::asm;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct GdtEntry {
    limit_lo: u16,
    base_lo: u16,
    base_mid: u8,
    access: u8,
    flags_limit_hi: u8,
    base_hi: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Gdtr {
    limit: u16,
    base: u64,
}

const GDT_ENTRIES: usize = 3;

#[repr(C, packed)]
struct GdtTable {
    entries: [GdtEntry; GDT_ENTRIES],
}

const GDT_ENTRY_INIT: GdtEntry = GdtEntry {
    limit_lo: 0,
    base_lo: 0,
    base_mid: 0,
    access: 0,
    flags_limit_hi: 0,
    base_hi: 0,
};

static mut GDT: GdtTable = GdtTable {
    entries: [GDT_ENTRY_INIT; 3],
};
static mut GDTR: Gdtr = Gdtr { limit: 0, base: 0 };

pub unsafe fn gdt_init() {
    core::ptr::write_bytes(
        &mut GDT as *mut GdtTable as *mut u8,
        0,
        core::mem::size_of::<GdtTable>(),
    );

    GDT.entries[1].access = 0x9A;
    GDT.entries[1].flags_limit_hi = 0x20;

    GDT.entries[2].access = 0x92;

    GDTR.limit = (core::mem::size_of::<GdtTable>() - 1) as u16;
    GDTR.base = &GDT as *const GdtTable as u64;

    let ptr = &GDTR as *const Gdtr as u64;
    asm!("lgdt ({0})", in(reg) ptr, options(att_syntax));

    asm!(
        "pushq $0x08",
        "leaq 1f(%rip), {tmp}",
        "pushq {tmp}",
        "lretq",
        "1:",
        "movw $0x10, %ax",
        "movw %ax, %ds",
        "movw %ax, %es",
        "movw %ax, %fs",
        "movw %ax, %gs",
        "movw %ax, %ss",
        tmp = lateout(reg) _,
        options(att_syntax),
    );
}
