use core::arch::asm;

const GDT_SIZE: usize = 7;

pub const GDT_NULL: u16 = 0x00;
pub const GDT_RING0_CODE: u16 = 0x08;
pub const GDT_RING0_DATA: u16 = 0x10;
pub const GDT_RING3_CODE: u16 = 0x18;
pub const GDT_RING3_DATA: u16 = 0x20;
pub const GDT_TSS: u16 = 0x28;

static mut GDT: [u64; GDT_SIZE] = [0u64; GDT_SIZE];

#[repr(C, packed)]
struct Gdtr {
    limit: u16,
    base: u64,
}

static mut GDTR: Gdtr = Gdtr { limit: 0, base: 0 };

unsafe fn gdt_make_entry(base: u32, limit: u32, access: u8, flags: u8) -> u64 {
    let mut e: u64 = 0;
    e |= (limit & 0xFFFF) as u64;
    e |= ((base & 0xFFFF) as u64) << 16;
    e |= (((base >> 16) & 0xFF) as u64) << 32;
    e |= (access as u64) << 40;
    e |= (((limit >> 16) & 0x0F) as u64) << 48;
    e |= ((flags & 0x0F) as u64) << 52;
    e |= (((base >> 24) & 0xFF) as u64) << 56;
    e
}

unsafe fn gdt_make_tss_lower(base: u64, limit: u32, access: u8) -> u64 {
    let base_lo = (base & 0xFFFFFF) as u32;
    let flags: u8 = 0;
    gdt_make_entry(base_lo, limit, access, flags)
}

unsafe fn gdt_make_tss_upper(base: u64) -> u64 {
    (base >> 32) as u64
}

pub unsafe fn gdt_init() {
    core::ptr::write_bytes(&mut GDT as *mut [u64; GDT_SIZE] as *mut u8, 0, 56);

    let r0c = gdt_make_entry(0, 0, 0x9A, 0x20);
    let r0d = gdt_make_entry(0, 0, 0x92, 0x00);
    let r3c = gdt_make_entry(0, 0, 0xFA, 0x20);
    let r3d = gdt_make_entry(0, 0, 0xF2, 0x00);

    GDT[1] = r0c;
    GDT[2] = r0d;
    GDT[3] = r3c;
    GDT[4] = r3d;

    GDTR.limit = (core::mem::size_of::<[u64; GDT_SIZE]>() - 1) as u16;
    GDTR.base = &GDT as *const [u64; GDT_SIZE] as u64;

    let ptr = &GDTR as *const Gdtr as u64;
    asm!("lgdt ({0})", in(reg) ptr, options(att_syntax));

    asm!(
        "pushq ${code}",
        "leaq 1f(%rip), {tmp}",
        "pushq {tmp}",
        "lretq",
        "1:",
        "movw ${data}, %ax",
        "movw %ax, %ds",
        "movw %ax, %es",
        "movw %ax, %fs",
        "movw %ax, %gs",
        "movw %ax, %ss",
        code = const GDT_RING0_CODE,
        data = const GDT_RING0_DATA,
        tmp = lateout(reg) _,
        options(att_syntax),
    );
}

pub unsafe fn gdt_set_tss(tss_base: u64, tss_limit: u32) {
    let lower = gdt_make_tss_lower(tss_base, tss_limit, 0x89);
    let upper = gdt_make_tss_upper(tss_base);
    GDT[5] = lower;
    GDT[6] = upper;
}

pub unsafe fn gdt_load_tr() {
    { asm!("ltr {0:x}", in(reg) GDT_TSS, options(att_syntax)); }
}
