use core::arch::asm;

pub const PIC1_CMD: u16 = 0x20;
pub const PIC1_DATA: u16 = 0x21;
pub const PIC2_CMD: u16 = 0xA0;
pub const PIC2_DATA: u16 = 0xA1;
pub const PIC_ICW1: u8 = 0x11;
pub const PIC_ICW4: u8 = 0x01;

pub unsafe fn pic_writeb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val);
}

pub unsafe fn pic_readb(port: u16) -> u8 {
    let val: u8;
    asm!("in al, dx", out("al") val, in("dx") port);
    val
}

pub unsafe fn pic_remap(offset1: u8, offset2: u8) {
    let mask1 = pic_readb(PIC1_DATA);
    let mask2 = pic_readb(PIC2_DATA);

    pic_writeb(PIC1_CMD, PIC_ICW1);
    pic_writeb(PIC2_CMD, PIC_ICW1);
    pic_writeb(PIC1_DATA, offset1);
    pic_writeb(PIC2_DATA, offset2);
    pic_writeb(PIC1_DATA, 4);
    pic_writeb(PIC2_DATA, 2);
    pic_writeb(PIC1_DATA, PIC_ICW4);
    pic_writeb(PIC2_DATA, PIC_ICW4);

    pic_writeb(PIC1_DATA, mask1);
    pic_writeb(PIC2_DATA, mask2);
}

pub unsafe fn pic_unmask_irq(irq: u8) {
    if irq < 8 {
        pic_writeb(PIC1_DATA, pic_readb(PIC1_DATA) & !(1u8 << irq));
    } else {
        pic_writeb(PIC2_DATA, pic_readb(PIC2_DATA) & !(1u8 << (irq - 8)));
    }
}
