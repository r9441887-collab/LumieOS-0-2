use core::arch::asm;
use core::ptr::{read_volatile, write_volatile};
use crate::drivers::nv_gpu_fw::*;

pub const NV_GPU_FILL_THRESHOLD: u32 = 64;

pub type NvGpuFillRect = unsafe extern "C" fn(u32, u32, u32, u32, u32) -> i32;
pub type NvGpuPutPixel = unsafe extern "C" fn(u32, u32, u32) -> i32;
pub type NvGpuGetPixel = unsafe extern "C" fn(u32, u32) -> u32;
pub type NvGpuVsync = unsafe extern "C" fn();
pub type NvGpuFlip = unsafe extern "C" fn();
pub type NvGpuIsActive = unsafe extern "C" fn() -> i32;

#[repr(C)]
pub struct NvGpuApi {
    pub fill_rect: Option<NvGpuFillRect>,
    pub put_pixel: Option<NvGpuPutPixel>,
    pub get_pixel: Option<NvGpuGetPixel>,
    pub vsync: Option<NvGpuVsync>,
    pub flip: Option<NvGpuFlip>,
    pub is_active: Option<NvGpuIsActive>,
}

#[repr(C)]
pub struct NvGpuState {
    pub fb_base: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub api_valid: i32,
    bar0_base: u64,
    bar1_base: u64,
    bar1_size: u64,
    fb_offset: u64,
    found: i32,
    fifo_ready: i32,
    channel_id: u32,
    push_base: u64,
    push_size: u32,
    push_pos: u32,
    double_buffer: i32,
    backbuffer_offset: u64,
    front_buf: i32,
    gpu_bus: u8,
    gpu_dev: u8,
    gpu_func: u8,
}

pub static mut G_NV_GPU_API: Option<&'static NvGpuApi> = None;

const NV_STATE_INIT: NvGpuState = NvGpuState {
    fb_base: 0,
    width: 0,
    height: 0,
    pitch: 0,
    api_valid: 0,
    bar0_base: 0,
    bar1_base: 0,
    bar1_size: 0,
    fb_offset: 0,
    found: 0,
    fifo_ready: 0,
    channel_id: 0,
    push_base: 0,
    push_size: 0,
    push_pos: 0,
    double_buffer: 0,
    backbuffer_offset: 0,
    front_buf: 0,
    gpu_bus: 0,
    gpu_dev: 0,
    gpu_func: 0,
};

pub static mut G_NV_STATE: NvGpuState = NV_STATE_INIT;

#[repr(C)]
struct SysBootInfo {
    version: u32,
    alloc: Option<unsafe extern "C" fn(u32) -> *mut u8>,
    free: Option<unsafe extern "C" fn(*mut u8)>,
    log: Option<unsafe extern "C" fn(*const u8)>,
    log_hex: Option<unsafe extern "C" fn(u64)>,
    gop_fb_base: u64,
    gop_width: u32,
    gop_height: u32,
    gop_pitch: u32,
}

const PCI_CONF_ADDR: u16 = 0xCF8;
const PCI_CONF_DATA: u16 = 0xCFC;
const NVIDIA_VENDOR: u16 = 0x10DE;

const NV_TIMEOUT_US: u64 = 1_000_000;
const TSC_GHZ: u64 = 2;

const PRAMIN_BASE: u64 = 0x00700000;
const USERD_BASE: u64 = 0x00C00000;
const CHAN_STRIDE: u64 = 0x1000;
const USERD_STRIDE: u64 = 0x0200;
const PUSH_SIZE: u32 = 0x4000;

const FALCON_CPUCTL: u32 = 0x100;
const FALCON_CPUSTAT: u32 = 0x104;
const FALCON_IRQSTAT: u32 = 0x108;
const FALCON_IRQMODE: u32 = 0x10C;
const FALCON_IMEMC: u32 = 0x180;
const FALCON_IMEMD: u32 = 0x184;
const FALCON_DMEMC: u32 = 0x1C0;
const FALCON_DMEMD: u32 = 0x1C4;
const FALCON_IRQSTAT32: u32 = 0x840;

const FECS_BASE: u32 = 0x409000;
const GPCCS_BASE: u32 = 0x41A000;

const P_CTXCTL_BUNDLE_ADDR_HI: u32 = 0x40800C;
const P_CTXCTL_BUNDLE_ADDR_LO: u32 = 0x408010;
const P_CTXCTL_PT_ADDR_HI: u32 = 0x408014;
const P_CTXCTL_PT_ADDR_LO: u32 = 0x408018;
const P_CTXCTL_PT_SIZE: u32 = 0x40801C;
const P_CTXCTL_CTXMEM_ADDR_HI: u32 = 0x408030;
const P_CTXCTL_CTXMEM_ADDR_LO: u32 = 0x408038;
const P_CTXCTL_TRIGGER: u32 = 0x408040;

const CTX_REGION_ALIGN: u32 = 0x1000;
const CTXMEM_SIZE: u32 = 0x8000;
const BUNDLE_SIZE: u32 = 0x1000;
const PT_SIZE: u32 = 0x1000;

const PUSH_HDR_2D_FMT: u32 = 0x000000CF;
const PUSH_HDR_2D_OP: u32 = 0x00000001;

const GK104_GOLDEN_MAIN: &[u32; 64] = &[
    0x404154, 0x00000004, 0x40415C, 0x00000000, 0x404158, 0x00000000,
    0x406020, 0x00001003, 0x406024, 0x00001003,
    0x406800, 0x00000000, 0x406808, 0x00000000, 0x40680C, 0x00000000,
    0x406050, 0x00000000, 0x406054, 0x00000000, 0x406058, 0x00000000, 0x40605C, 0x00000000,
    0x406400, 0x00000000, 0x406404, 0x00000000, 0x406408, 0x00000000, 0x40640C, 0x00000000,
    0x418800, 0x00000000, 0x418804, 0x00000000, 0x418808, 0x00000000, 0x41880C, 0x00000000,
    0x408800, 0x00000000, 0x408804, 0x00000000, 0x408808, 0x00000000, 0x40880C, 0x00000000,
    0x408100, 0x00000000, 0x408104, 0x00000000, 0x408108, 0x00000000, 0x40810C, 0x00000000,
    0x406100, 0x00000001, 0x406104, 0x00000001,
    0x00000000, 0x00000000,
];

fn pci_read(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let addr: u32 =
        0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);
    unsafe {
        asm!("outl %eax, %dx", in("eax") addr, in("dx") PCI_CONF_ADDR, options(att_syntax));
    }
    let val: u32;
    unsafe {
        asm!("inl %dx, %eax", out("eax") val, in("dx") PCI_CONF_DATA, options(att_syntax));
    }
    val
}

fn pci_write(bus: u8, dev: u8, func: u8, offset: u8, val: u32) {
    let addr: u32 =
        0x80000000 | ((bus as u32) << 16) | ((dev as u32) << 11) | ((func as u32) << 8) | ((offset as u32) & 0xFC);
    unsafe {
        asm!("outl %eax, %dx", in("eax") addr, in("dx") PCI_CONF_ADDR, options(att_syntax));
        asm!("outl %eax, %dx", in("eax") val, in("dx") PCI_CONF_DATA, options(att_syntax));
    }
}

fn pci_enable_mem_busmaster(bus: u8, dev: u8, func: u8) {
    let cmd = pci_read(bus, dev, func, 0x04);
    pci_write(bus, dev, func, 0x04, cmd | 0x06);
}

fn reg_read32(bar0: u64, offset: u32) -> u32 {
    unsafe { read_volatile((bar0 + offset as u64) as *const u32) }
}

fn reg_write32(bar0: u64, offset: u32, val: u32) {
    unsafe { write_volatile((bar0 + offset as u64) as *mut u32, val) }
}

fn vram_read32(bar1: u64, offset: u64) -> u32 {
    unsafe { read_volatile((bar1 + offset) as *const u32) }
}

fn vram_write32(bar1: u64, offset: u64, val: u32) {
    unsafe { write_volatile((bar1 + offset) as *mut u32, val) }
}

fn tsc_read() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        asm!("rdtsc", out("eax") lo, out("edx") hi, options(att_syntax));
    }
    ((hi as u64) << 32) | lo as u64
}

fn read_bar(bus: u8, dev: u8, func: u8, off: u8) -> u64 {
    let low = pci_read(bus, dev, func, off);
    if low & 1 != 0 {
        return 0;
    }
    let mut addr = (low as u64) & 0xFFFFFFF0;
    if (low & 0x06) == 0x04 {
        let high = pci_read(bus, dev, func, off + 4);
        addr |= (high as u64) << 32;
    }
    addr
}

fn bar_size(bus: u8, dev: u8, func: u8, off: u8) -> u64 {
    let orig = pci_read(bus, dev, func, off);
    let orig64: u64 = if (orig & 0x06) == 0x04 {
        let hi = pci_read(bus, dev, func, off + 4);
        ((hi as u64) << 32) | orig as u64
    } else {
        orig as u64
    };

    pci_write(bus, dev, func, off, 0xFFFFFFFF);
    if (orig & 0x06) == 0x04 {
        pci_write(bus, dev, func, off + 4, 0xFFFFFFFF);
    }

    let sz_low = pci_read(bus, dev, func, off);
    let size: u64 = if (orig & 0x06) == 0x04 {
        let sz_hi = pci_read(bus, dev, func, off + 4);
        (!((((sz_hi as u64) << 32) | sz_low as u64) & !0x0F) + 1)
    } else {
        (!(sz_low & 0xFFFFFFF0) + 1) as u64
    };

    pci_write(bus, dev, func, off, orig as u32);
    if (orig & 0x06) == 0x04 {
        pci_write(bus, dev, func, off + 4, (orig64 >> 32) as u32);
    }
    size
}

fn probe_scanout(bar0: u64) -> u64 {
    let lo = reg_read32(bar0, 0x00810308);
    let hi = reg_read32(bar0, 0x0081030C);
    let mut addr = ((hi as u64) << 32) | lo as u64;
    if addr > 0x400000 && addr < 0x4000000000 {
        return addr;
    }
    let lo2 = reg_read32(bar0, 0x00610308);
    let hi2 = reg_read32(bar0, 0x0061030C);
    addr = ((hi2 as u64) << 32) | lo2 as u64;
    if addr > 0x400000 && addr < 0x4000000000 {
        return addr;
    }
    0
}

fn find_gpu() -> i32 {
    unsafe { G_NV_STATE.found = 0 }

    for bus in 0..256u8 {
        for dev in 0..32u8 {
            let id0 = pci_read(bus, dev, 0, 0);
            if id0 == 0xFFFFFFFF {
                continue;
            }
            let hdr_type = pci_read(bus, dev, 0, 0x0C);
            let is_multi = (hdr_type >> 23) & 1;
            let max_func = if is_multi != 0 { 8 } else { 1 };
            for func in 0..max_func {
                let id = if func == 0 { id0 } else { pci_read(bus, dev, func, 0) };
                if id == 0xFFFFFFFF {
                    continue;
                }
                let vendor = (id & 0xFFFF) as u16;
                if vendor != NVIDIA_VENDOR {
                    continue;
                }
                let class_rev = pci_read(bus, dev, func, 8);
                let class_code = (class_rev >> 24) as u8;
                if class_code != 0x03 {
                    continue;
                }
                unsafe {
                    G_NV_STATE.found = 1;
                    G_NV_STATE.gpu_bus = bus;
                    G_NV_STATE.gpu_dev = dev;
                    G_NV_STATE.gpu_func = func;
                }
                break;
            }
            if unsafe { G_NV_STATE.found != 0 } {
                break;
            }
        }
        if unsafe { G_NV_STATE.found != 0 } {
            break;
        }
    }

    unsafe {
        if G_NV_STATE.found != 0 {
            pci_enable_mem_busmaster(G_NV_STATE.gpu_bus, G_NV_STATE.gpu_dev, G_NV_STATE.gpu_func);
        }
        G_NV_STATE.found
    }
}

fn fifo_init() -> i32 {
    unsafe {
        let bar0 = G_NV_STATE.bar0_base;
        let bar1 = G_NV_STATE.bar1_base;

        let fb_size = G_NV_STATE.pitch * G_NV_STATE.height;
        let push_offset = (G_NV_STATE.fb_offset as u32 + fb_size * 2 + 0xFFFF) & !0xFFFF;

        if (push_offset as u64) + PUSH_SIZE as u64 > G_NV_STATE.bar1_size {
            return 0;
        }

        let mut chan: u32 = 0;
        let mut found_chan = 0;
        for i in 0..128 {
            let entry = reg_read32(bar0, PRAMIN_BASE as u32 + i * CHAN_STRIDE as u32);
            if entry & 1 == 0 {
                chan = i;
                found_chan = 1;
                break;
            }
        }
        if found_chan == 0 {
            return 0;
        }

        let mut i: u32 = 0;
        while i < PUSH_SIZE {
            vram_write32(bar1, push_offset as u64 + i as u64, 0);
            i += 4;
        }

        let userd_chan = bar0 + USERD_BASE + chan as u64 * USERD_STRIDE;
        reg_write32(bar0, (userd_chan - bar0) as u32 + 0x00, push_offset);
        reg_write32(bar0, (userd_chan - bar0) as u32 + 0x04, push_offset);
        reg_write32(bar0, (userd_chan - bar0) as u32 + 0x08, 0x00010000);
        reg_write32(bar0, (userd_chan - bar0) as u32 + 0x0C, 0x00000001);

        let ramin = bar0 + PRAMIN_BASE;
        let ramin_chan = ramin + chan as u64 * CHAN_STRIDE;
        let userd_offset = USERD_BASE as u32 + chan * USERD_STRIDE as u32;
        reg_write32(bar0, (ramin_chan - bar0) as u32 + 0x00, (userd_offset >> 8) | 1);
        reg_write32(bar0, (ramin_chan - bar0) as u32 + 0x04, 0);
        reg_write32(bar0, (ramin_chan - bar0) as u32 + 0x08, 0);

        reg_write32(bar0, 0x002100, 1);
        reg_write32(bar0, 0x00400100, 0x0000902D);
        reg_write32(bar0, 0x00400104, 0xFFFFFFFF);

        G_NV_STATE.channel_id = chan;
        G_NV_STATE.push_base = bar1 + push_offset as u64;
        G_NV_STATE.push_size = PUSH_SIZE;
        G_NV_STATE.push_pos = 0;
        G_NV_STATE.fifo_ready = 1;

        1
    }
}

fn push_reserve(dwords_needed: u32) -> i32 {
    unsafe {
        if G_NV_STATE.push_pos + dwords_needed * 4 >= G_NV_STATE.push_size {
            return 0;
        }
    }
    1
}

fn push_off() -> u32 {
    unsafe { (G_NV_STATE.push_base - G_NV_STATE.bar1_base) as u32 }
}

fn push_write(data: u32) {
    unsafe {
        vram_write32(G_NV_STATE.bar1_base, push_off() as u64 + G_NV_STATE.push_pos as u64, data);
        G_NV_STATE.push_pos += 4;
    }
}

fn push_submit() {
    unsafe {
        let bar0 = G_NV_STATE.bar0_base;
        let chan = G_NV_STATE.channel_id;
        let userd_chan = bar0 + USERD_BASE + chan as u64 * USERD_STRIDE;
        let new_put = push_off() + G_NV_STATE.push_pos;

        asm!("sfence", options(nostack));

        reg_write32(bar0, (userd_chan - bar0) as u32, new_put);
        reg_write32(bar0, 0x002040, chan);
        reg_write32(bar0, 0x002100, 1);

        let deadline = tsc_read() + NV_TIMEOUT_US * TSC_GHZ * 1000;
        loop {
            let get_val = read_volatile((userd_chan + 4) as *const u32);
            if get_val == new_put {
                break;
            }
            if tsc_read() >= deadline {
                break;
            }
            asm!("pause", options(nostack));
        }

        G_NV_STATE.push_pos = 0;
    }
}

fn golden_apply(bar0: u64) {
    let mut i = 0;
    while i + 1 < GK104_GOLDEN_MAIN.len() {
        let addr = GK104_GOLDEN_MAIN[i];
        let val = GK104_GOLDEN_MAIN[i + 1];
        if addr == 0 {
            break;
        }
        reg_write32(bar0, addr, val);
        i += 2;
    }
}

fn ctx_init(bar0: u64, bar1: u64) -> i32 {
    unsafe {
        let fb_size = G_NV_STATE.pitch * G_NV_STATE.height;
        let push_end = (G_NV_STATE.fb_offset as u32 + fb_size * 2 + PUSH_SIZE + 0xFFFF) & !0xFFFF;
        let base = (push_end + CTX_REGION_ALIGN) & !(CTX_REGION_ALIGN - 1);

        if (base as u64) + CTXMEM_SIZE as u64 + BUNDLE_SIZE as u64 + PT_SIZE as u64 > G_NV_STATE.bar1_size {
            return 0;
        }

        let ctxmem_off = base;
        let bundle_off = base + CTXMEM_SIZE;
        let pt_off = base + CTXMEM_SIZE + BUNDLE_SIZE;

        let mut i: u32 = 0;
        while i < PT_SIZE {
            vram_write32(bar1, pt_off as u64 + i as u64, 0);
            i += 4;
        }
        let mut i: u32 = 0;
        while i < BUNDLE_SIZE {
            vram_write32(bar1, bundle_off as u64 + i as u64, 0);
            i += 4;
        }
        let mut i: u32 = 0;
        while i < CTXMEM_SIZE {
            vram_write32(bar1, ctxmem_off as u64 + i as u64, 0);
            i += 4;
        }

        reg_write32(bar0, P_CTXCTL_BUNDLE_ADDR_HI, 0);
        reg_write32(bar0, P_CTXCTL_BUNDLE_ADDR_LO, bundle_off | 0x80000000);
        reg_write32(bar0, P_CTXCTL_PT_ADDR_HI, 0);
        reg_write32(bar0, P_CTXCTL_PT_ADDR_LO, pt_off);
        reg_write32(bar0, P_CTXCTL_PT_SIZE, 0x00000001);
        reg_write32(bar0, P_CTXCTL_CTXMEM_ADDR_HI, 0);
        reg_write32(bar0, P_CTXCTL_CTXMEM_ADDR_LO, ctxmem_off | 0x00000001);

        1
    }
}

fn falcon_load(bar0: u64, base: u32, code: &[u32], data: &[u32]) -> i32 {
    reg_write32(bar0, base + FALCON_DMEMC, 0x01000000);
    for i in 0..data.len() {
        reg_write32(bar0, base + FALCON_DMEMD, data[i]);
    }

    reg_write32(bar0, base + FALCON_IMEMC, 0x01000000);
    for i in 0..code.len() {
        reg_write32(bar0, base + FALCON_IMEMD, code[i]);
    }

    1
}

fn falcon_start(bar0: u64, base: u32) -> i32 {
    reg_write32(bar0, base + FALCON_IRQSTAT32, 0xFFFFFFFF);
    reg_write32(bar0, base + FALCON_CPUCTL, 0x00000002);

    let deadline = tsc_read() + NV_TIMEOUT_US * TSC_GHZ * 1000;
    loop {
        let stat = reg_read32(bar0, base + FALCON_CPUSTAT);
        if stat & 1 != 0 {
            return 1;
        }
        if tsc_read() >= deadline {
            break;
        }
        unsafe {
            asm!("pause", options(nostack));
        }
    }
    0
}

fn init_3d() -> i32 {
    unsafe {
        if G_NV_STATE.found == 0 || G_NV_STATE.bar0_base == 0 {
            return 0;
        }
        if G_NV_STATE.fifo_ready == 0 {
            return 0;
        }

        let bar0 = G_NV_STATE.bar0_base;
        let bar1 = G_NV_STATE.bar1_base;

        golden_apply(bar0);

        if ctx_init(bar0, bar1) == 0 {
            return 0;
        }

        if falcon_load(bar0, FECS_BASE, GK104_FECS_CODE, GK104_FECS_DATA) == 0 {
            return 0;
        }

        if falcon_load(bar0, GPCCS_BASE, GK104_GPCCS_CODE, GK104_GPCCS_DATA) == 0 {
            return 0;
        }

        reg_write32(bar0, FECS_BASE + FALCON_IRQMODE, 0x00000000);
        reg_write32(bar0, GPCCS_BASE + FALCON_IRQMODE, 0x00000000);

        if falcon_start(bar0, FECS_BASE) == 0 {
            return 0;
        }
        if falcon_start(bar0, GPCCS_BASE) == 0 {
            return 0;
        }

        reg_write32(bar0, P_CTXCTL_TRIGGER, 0x00000000);

        for _ in 0..1000 {
            asm!("pause", options(nostack));
        }

        let pg_status = reg_read32(bar0, 0x400100);
        if pg_status != 0 {
            return 0;
        }

        reg_write32(bar0, 0x00400108, 0x000090B5);
        reg_write32(bar0, 0x0040010C, 0xFFFFFFFF);

        1
    }
}

pub unsafe fn nv_gpu_init(boot_info: *const core::ffi::c_void) -> i32 {
    G_NV_STATE = NV_STATE_INIT;

    if find_gpu() == 0 {
        return 0;
    }

    let bar0 = read_bar(G_NV_STATE.gpu_bus, G_NV_STATE.gpu_dev, G_NV_STATE.gpu_func, 0x10);
    let bar1 = read_bar(G_NV_STATE.gpu_bus, G_NV_STATE.gpu_dev, G_NV_STATE.gpu_func, 0x18);
    let bar1_sz = bar_size(G_NV_STATE.gpu_bus, G_NV_STATE.gpu_dev, G_NV_STATE.gpu_func, 0x18);

    if bar0 == 0 || bar1 == 0 || bar1_sz == 0 {
        return 0;
    }

    G_NV_STATE.bar0_base = bar0;
    G_NV_STATE.bar1_base = bar1;
    G_NV_STATE.bar1_size = bar1_sz;

    let info = &*(boot_info as *const SysBootInfo);
    G_NV_STATE.width = info.gop_width;
    G_NV_STATE.height = info.gop_height;
    G_NV_STATE.pitch = info.gop_pitch;

    if info.gop_fb_base >= bar1 && info.gop_fb_base < bar1 + bar1_sz {
        G_NV_STATE.fb_offset = info.gop_fb_base - bar1;
    } else {
        let scanout = probe_scanout(bar0);
        if scanout != 0 && scanout >= bar1 && scanout < bar1 + bar1_sz {
            G_NV_STATE.fb_offset = scanout - bar1;
        } else {
            return 0;
        }
    }

    G_NV_STATE.fb_base = bar1 + G_NV_STATE.fb_offset;

    fifo_init();
    init_3d();

    G_NV_STATE.api_valid = 1;
    1
}

pub unsafe fn nv_gpu_fill_rect(x: u32, y: u32, w: u32, h: u32, color: u32) -> i32 {
    if let Some(api) = G_NV_GPU_API {
        if let Some(fill) = api.fill_rect {
            return fill(x, y, w, h, color);
        }
        return -1;
    }
    if G_NV_STATE.fifo_ready == 0 {
        return -1;
    }

    asm!("cli", options(nostack));

    let bar1 = G_NV_STATE.bar1_base;
    let fb = if G_NV_STATE.front_buf != 0 {
        G_NV_STATE.fb_offset
    } else {
        G_NV_STATE.backbuffer_offset
    };
    let pitch = G_NV_STATE.pitch;
    let dst_addr = (fb + y as u64 * pitch as u64 + x as u64 * 4) as u32;
    let dst_size = (w << 16) | h;
    let dst_origin = (x << 16) | y;
    let dst_coord = (w << 16) | h;

    if push_reserve(4 * 10) == 0 {
        asm!("sti", options(nostack));
        return -1;
    }

    push_write((1 << 24) | (0 << 13) | ((0x0200 >> 2) << 2) | (1 << 1));
    push_write(PUSH_HDR_2D_FMT);

    push_write((1 << 24) | (0 << 13) | ((0x0208 >> 2) << 2) | (1 << 1));
    push_write(PUSH_HDR_2D_OP);

    push_write((1 << 24) | (0 << 13) | ((0x0210 >> 2) << 2) | (1 << 1));
    push_write(color);

    push_write((2 << 24) | (0 << 13) | ((0x0258 >> 2) << 2) | (0 << 1));
    push_write(0);
    push_write(dst_addr);

    push_write((1 << 24) | (0 << 13) | ((0x0304 >> 2) << 2) | (1 << 1));
    push_write(pitch);

    push_write((1 << 24) | (0 << 13) | ((0x030C >> 2) << 2) | (1 << 1));
    push_write(dst_size);

    push_write((2 << 24) | (0 << 13) | ((0x0318 >> 2) << 2) | (0 << 1));
    push_write(dst_origin);
    push_write(dst_coord);

    push_submit();
    asm!("sti", options(nostack));
    0
}

pub unsafe fn nv_gpu_put_pixel(x: u32, y: u32, color: u32) -> i32 {
    if let Some(api) = G_NV_GPU_API {
        if let Some(put) = api.put_pixel {
            return put(x, y, color);
        }
        return -1;
    }
    if G_NV_STATE.found == 0 || G_NV_STATE.bar1_base == 0 {
        return -1;
    }
    if x >= G_NV_STATE.width || y >= G_NV_STATE.height {
        return -1;
    }
    let base = if G_NV_STATE.front_buf != 0 {
        G_NV_STATE.fb_offset
    } else {
        G_NV_STATE.backbuffer_offset
    };
    let off = base + y as u64 * G_NV_STATE.pitch as u64 + x as u64 * 4;
    vram_write32(G_NV_STATE.bar1_base, off, color);
    0
}

pub unsafe fn nv_gpu_get_pixel(x: u32, y: u32) -> u32 {
    if let Some(api) = G_NV_GPU_API {
        if let Some(get) = api.get_pixel {
            return get(x, y);
        }
        return 0;
    }
    if G_NV_STATE.found == 0 || G_NV_STATE.bar1_base == 0 {
        return 0;
    }
    if x >= G_NV_STATE.width || y >= G_NV_STATE.height {
        return 0;
    }
    let base = if G_NV_STATE.front_buf != 0 {
        G_NV_STATE.fb_offset
    } else {
        G_NV_STATE.backbuffer_offset
    };
    let off = base + y as u64 * G_NV_STATE.pitch as u64 + x as u64 * 4;
    vram_read32(G_NV_STATE.bar1_base, off)
}

pub unsafe fn nv_gpu_vsync() {
    if let Some(api) = G_NV_GPU_API {
        if let Some(vsync) = api.vsync {
            vsync();
            return;
        }
    }
    if G_NV_STATE.found == 0 {
        return;
    }
    let bar0 = G_NV_STATE.bar0_base;
    let stamp = reg_read32(bar0, 0x00810390);
    let deadline = tsc_read() + (NV_TIMEOUT_US / 100) * TSC_GHZ * 1000;
    loop {
        let ns = reg_read32(bar0, 0x00810390);
        if ns != stamp {
            break;
        }
        if tsc_read() >= deadline {
            break;
        }
        asm!("pause", options(nostack));
    }
}

pub unsafe fn nv_gpu_flip() {
    if let Some(api) = G_NV_GPU_API {
        if let Some(flip) = api.flip {
            flip();
            return;
        }
    }
    if G_NV_STATE.found == 0 || G_NV_STATE.double_buffer == 0 {
        return;
    }
    let bar0 = G_NV_STATE.bar0_base;
    let new_base = if G_NV_STATE.front_buf != 0 {
        G_NV_STATE.fb_offset
    } else {
        G_NV_STATE.backbuffer_offset
    };
    reg_write32(bar0, 0x00810300, new_base as u32);
    reg_write32(bar0, 0x00810304, 0);
    nv_gpu_vsync();
    G_NV_STATE.front_buf = if G_NV_STATE.front_buf != 0 { 0 } else { 1 };
}

pub unsafe fn nv_gpu_is_active() -> i32 {
    if let Some(api) = G_NV_GPU_API {
        if let Some(active) = api.is_active {
            return active();
        }
        return 0;
    }
    if G_NV_STATE.found != 0 && G_NV_STATE.bar1_base != 0 {
        1
    } else {
        0
    }
}
