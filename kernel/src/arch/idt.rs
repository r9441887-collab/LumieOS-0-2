use core::arch::asm;

const IDT_ENTRIES: usize = 256;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct IdtEntry {
    offset_lo: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_hi: u32,
    reserved: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Idtr {
    limit: u16,
    base: u64,
}

#[repr(C, align(16))]
struct IdtTable {
    entries: [IdtEntry; IDT_ENTRIES],
}

const IDT_ENTRY_INIT: IdtEntry = IdtEntry {
    offset_lo: 0,
    selector: 0,
    ist: 0,
    type_attr: 0,
    offset_mid: 0,
    offset_hi: 0,
    reserved: 0,
};

static mut IDT: IdtTable = IdtTable {
    entries: [IDT_ENTRY_INIT; 256],
};
static mut IDTR: Idtr = Idtr { limit: 0, base: 0 };

pub unsafe fn idt_set_entry(index: usize, handler: u64, selector: u16, type_attr: u8) {
    IDT.entries[index].offset_lo = (handler & 0xFFFF) as u16;
    IDT.entries[index].selector = selector;
    IDT.entries[index].ist = 0;
    IDT.entries[index].type_attr = type_attr;
    IDT.entries[index].offset_mid = ((handler >> 16) & 0xFFFF) as u16;
    IDT.entries[index].offset_hi = ((handler >> 32) & 0xFFFFFFFF) as u32;
    IDT.entries[index].reserved = 0;
}

pub unsafe fn idt_init() {
    core::ptr::write_bytes(
        &mut IDT as *mut IdtTable as *mut u8,
        0,
        core::mem::size_of::<IdtTable>(),
    );
    IDTR.limit = (core::mem::size_of::<IdtTable>() - 1) as u16;
    IDTR.base = &IDT as *const IdtTable as u64;

    let ptr = &IDTR as *const Idtr as u64;
    asm!("lidt ({0})", in(reg) ptr, options(att_syntax));
}
