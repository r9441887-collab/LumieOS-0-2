#![allow(dead_code)]
use core::arch::asm;
use core::ptr;
use crate::mm::heap;

const CMD_TAB_SIZE: u32 = 256;
const PRDT_MAX: usize = 8;
const AHCI_MAX_PORTS: usize = 8;

#[repr(C, packed)]
struct AhciPrd {
    dba: u64,
    reserved: u32,
    byte_count: u32,
}

#[repr(C, packed)]
struct AhciCmdTable {
    cfis: [u8; 64],
    acmd: [u8; 16],
    reserved: [u8; 48],
    prdt: [AhciPrd; PRDT_MAX],
}

#[repr(C, packed)]
struct AhciCmdHeader {
    dw0: u32,
    dw1: u32,
    ctba: u64,
    reserved: [u32; 4],
}

#[repr(C, packed)]
struct AhciCmdList {
    headers: [AhciCmdHeader; 32],
}

#[repr(C, packed)]
struct AhciFis {
    fis: [u8; 256],
}

struct AhciPort {
    port_num: i32,
    sector_count: u64,
    sector_size: u32,
    is_ready: i32,
    is_ssd: i32,
    cmd_list: *mut AhciCmdList,
    cmd_table: *mut AhciCmdTable,
    fis: *mut AhciFis,
}

static mut G_ABAR: *mut u8 = core::ptr::null_mut();
static mut G_AHCI_FOUND: i32 = 0;
static mut G_PORTS: [Option<AhciPort>; AHCI_MAX_PORTS] =
    [None, None, None, None, None, None, None, None];
static mut G_PORT_COUNT: i32 = 0;

// PCI config space access via port I/O
unsafe fn pci_outl(port: u16, val: u32) {
    asm!("out dx, eax", in("dx") port, in("eax") val, options(nostack, preserves_flags));
}

unsafe fn pci_inl(port: u16) -> u32 {
    let val: u32;
    asm!("in eax, dx", out("eax") val, in("dx") port, options(nostack, preserves_flags));
    val
}

unsafe fn pci_cfg_read(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let addr = 0x80000000u32
        | ((bus as u32) << 16)
        | (((dev as u32) & 0x1F) << 11)
        | (((func as u32) & 7) << 8)
        | ((off as u32) & 0xFC);
    pci_outl(0xCF8, addr);
    pci_inl(0xCFC)
}

unsafe fn pci_cfg_write(bus: u8, dev: u8, func: u8, off: u8, val: u32) {
    let addr = 0x80000000u32
        | ((bus as u32) << 16)
        | (((dev as u32) & 0x1F) << 11)
        | (((func as u32) & 7) << 8)
        | ((off as u32) & 0xFC);
    pci_outl(0xCF8, addr);
    pci_outl(0xCFC, val);
}

// AHCI register access helpers (MMIO)
const AHCI_CAP: u32 = 0x00;
const AHCI_GHC: u32 = 0x04;
const AHCI_CAP2: u32 = 0x24;
const AHCI_PORTS: u32 = 0x0C;
const GHC_HR: u32 = 1;
const GHC_AE: u32 = 1 << 31;

const fn port_off(i: u32) -> u32 { 0x100 + i * 0x80 }
const PORT_CLB: u32 = 0x00;
const PORT_CLBU: u32 = 0x04;
const PORT_FB: u32 = 0x08;
const PORT_FBU: u32 = 0x0C;
const PORT_IS: u32 = 0x10;
const PORT_IE: u32 = 0x14;
const PORT_CMD: u32 = 0x18;
const PORT_TFD: u32 = 0x20;
const PORT_SIG: u32 = 0x24;
const PORT_SSTS: u32 = 0x28;
const PORT_SCTL: u32 = 0x2C;
const PORT_SERR: u32 = 0x30;
const PORT_CI: u32 = 0x38;
const PORT_ACT: u32 = 0x3C;

/* Port command flags */
const PORT_CMD_ST: u32 = 1;
const PORT_CMD_SUD: u32 = 1 << 1;
const PORT_CMD_POD: u32 = 1 << 2;
const PORT_CMD_FRE: u32 = 1 << 4;
const PORT_CMD_FR: u32 = 1 << 14;
const PORT_CMD_CR: u32 = 1 << 15;

const SIG_SATA: u32 = 0x00000101;

unsafe fn reg_read(reg: u32) -> u32 {
    if G_ABAR.is_null() { return 0; }
    ptr::read_volatile(G_ABAR.add(reg as usize) as *const u32)
}

unsafe fn reg_write(reg: u32, val: u32) {
    if G_ABAR.is_null() { return; }
    ptr::write_volatile(G_ABAR.add(reg as usize) as *mut u32, val);
}

unsafe fn port_reg_read(port: u32, reg: u32) -> u32 {
    if G_ABAR.is_null() { return 0; }
    let off = port_off(port) + reg;
    ptr::read_volatile(G_ABAR.add(off as usize) as *const u32)
}

unsafe fn port_reg_write(port: u32, reg: u32, val: u32) {
    if G_ABAR.is_null() { return; }
    let off = port_off(port) + reg;
    ptr::write_volatile(G_ABAR.add(off as usize) as *mut u32, val);
}

unsafe fn mdelay(ms: u32) {
    for _ in 0..ms * 100000 {
        asm!("pause", options(nostack));
    }
}

unsafe fn ahci_find_controller() -> i32 {
    for bus in 0u16..256 {
        let hdr0 = pci_cfg_read(bus as u8, 0, 0, 0);
        if hdr0 == 0xFFFFFFFF { continue; }
        for dev in 0u8..32 {
            let id = pci_cfg_read(bus as u8, dev, 0, 0);
            if id == 0xFFFFFFFF { continue; }
            let mf = (pci_cfg_read(bus as u8, dev, 0, 0xC) >> 23) & 1;
            let funcs = if mf != 0 { 8u8 } else { 1u8 };
            for func in 0u8..funcs {
                let idf = if func == 0 { id } else { pci_cfg_read(bus as u8, dev, func, 0) };
                if idf == 0xFFFFFFFF { continue; }
                let cr = pci_cfg_read(bus as u8, dev, func, 8);
                let class_code = ((cr >> 24) & 0xFF) as u8;
                let subclass = ((cr >> 16) & 0xFF) as u8;
                let prog_if = ((cr >> 8) & 0xFF) as u8;
                if class_code == 0x01 && subclass == 0x06 && prog_if == 0x01 {
                    let bar5 = pci_cfg_read(bus as u8, dev, func, 0x24);
                    let abar = bar5 & !0xF;
                    if abar == 0 { continue; }

                    let cmd = pci_cfg_read(bus as u8, dev, func, 0x04);
                    pci_cfg_write(bus as u8, dev, func, 0x04, cmd | 6);

                    G_ABAR = abar as *mut u8;
                    return 0;
                }
            }
        }
    }
    -1
}

unsafe fn ahci_init_port(p: u32) -> i32 {
    port_reg_write(p, PORT_SCTL, 0x301);
    mdelay(10);
    port_reg_write(p, PORT_SCTL, 0);

    let sig = port_reg_read(p, PORT_SIG);
    if sig != SIG_SATA { return -1; }

    let mut cmd = port_reg_read(p, PORT_CMD);
    cmd |= PORT_CMD_SUD | PORT_CMD_POD;
    port_reg_write(p, PORT_CMD, cmd);
    mdelay(10);

    let ssts = port_reg_read(p, PORT_SSTS);
    let ipm = ((ssts >> 8) & 0x0F) as u8;
    let det = (ssts & 0x0F) as u8;
    if det != 3 || ipm != 1 { return -1; }

    let idx = G_PORT_COUNT;
    if idx as usize >= AHCI_MAX_PORTS { return -1; }

    let cmd_list = heap::kmalloc(16384) as *mut AhciCmdList;
    if cmd_list.is_null() { return -1; }
    ptr::write_bytes(cmd_list as *mut u8, 0, 16384);

    let cmd_table = heap::kmalloc(core::mem::size_of::<AhciCmdTable>() as u64) as *mut AhciCmdTable;
    if cmd_table.is_null() { heap::kfree(cmd_list as *mut u8); return -1; }
    ptr::write_bytes(cmd_table as *mut u8, 0, core::mem::size_of::<AhciCmdTable>());

    let fis = heap::kmalloc(256) as *mut AhciFis;
    if fis.is_null() { heap::kfree(cmd_list as *mut u8); heap::kfree(cmd_table as *mut u8); return -1; }
    ptr::write_bytes(fis as *mut u8, 0, 256);

    let clb_phys = cmd_list as u64;
    port_reg_write(p, PORT_CLB, (clb_phys & 0xFFFFFFFF) as u32);
    port_reg_write(p, PORT_CLBU, (clb_phys >> 32) as u32);

    let fb_phys = fis as u64;
    port_reg_write(p, PORT_FB, (fb_phys & 0xFFFFFFFF) as u32);
    port_reg_write(p, PORT_FBU, (fb_phys >> 32) as u32);

    port_reg_write(p, PORT_IS, 0xFFFFFFFF);
    port_reg_write(p, PORT_IE, 0);

    cmd = port_reg_read(p, PORT_CMD);
    cmd |= PORT_CMD_FRE;
    port_reg_write(p, PORT_CMD, cmd);
    mdelay(5);

    cmd = port_reg_read(p, PORT_CMD);
    cmd |= PORT_CMD_ST;
    port_reg_write(p, PORT_CMD, cmd);
    mdelay(5);

    let ident_buf = heap::kmalloc(512);
    if ident_buf.is_null() { heap::kfree(cmd_list as *mut u8); heap::kfree(cmd_table as *mut u8); heap::kfree(fis as *mut u8); return -1; }
    ptr::write_bytes(ident_buf, 0, 512);

    ptr::write_bytes(cmd_table as *mut u8, 0, core::mem::size_of::<AhciCmdTable>());

    let cfis = &mut (*cmd_table).cfis;
    cfis[0] = 0x27;
    cfis[1] = 0x80;
    cfis[2] = 0xEC;
    cfis[3] = 0;
    cfis[4] = 0;
    cfis[5] = 0;
    cfis[6] = 0;
    cfis[7] = 0xC0;
    for i in 8..16 { cfis[i] = 0; }

    (*cmd_table).prdt[0].dba = ident_buf as u64;
    (*cmd_table).prdt[0].byte_count = 512 - 1;

    ptr::write_bytes(cmd_list as *mut u8, 0, core::mem::size_of::<AhciCmdList>());
    (*cmd_list).headers[0].dw0 = (5 << 0) | (1 << 16);
    (*cmd_list).headers[0].dw1 = 512 - 1;
    (*cmd_list).headers[0].ctba = cmd_table as u64;

    port_reg_write(p, PORT_CI, 1);
    while port_reg_read(p, PORT_CI) & 1 != 0 { mdelay(1); }

    let tfd = port_reg_read(p, PORT_TFD);
    if tfd & 0x7F != 0 {
        heap::kfree(ident_buf);
        heap::kfree(cmd_list as *mut u8);
        heap::kfree(cmd_table as *mut u8);
        heap::kfree(fis as *mut u8);
        return -1;
    }

    let ident = ident_buf as *mut u16;
    let lba48_valid = (ptr::read_volatile(ident.add(83)) & (1 << 15)) != 0;
    let lba48 = lba48_valid && (ptr::read_volatile(ident.add(83)) & (1 << 10)) != 0;
    let sector_count = if lba48 {
        ptr::read_volatile(ident.add(100) as *mut u64)
    } else {
        ptr::read_volatile(ident.add(60) as *mut u32) as u64
    };

    let is_ssd = {
        let word217 = ptr::read_volatile(ident.add(217));
        if word217 == 0 || word217 == 1 { 1 } else { 0 }
    };

    let w106 = ptr::read_volatile(ident.add(106));
    let ls_type = (w106 >> 12) & 3;
    let sector_size = match ls_type {
        1 | 0 => 512,
        2 => ptr::read_volatile(ident.add(117) as *mut u32) * 512,
        3 => if w106 & (1 << 14) != 0 { 4096u32 } else { 512 },
        _ => 512,
    };

    heap::kfree(ident_buf);

    if sector_count == 0 {
        heap::kfree(cmd_list as *mut u8);
        heap::kfree(cmd_table as *mut u8);
        heap::kfree(fis as *mut u8);
        return -1;
    }

    G_PORTS[idx as usize] = Some(AhciPort {
        port_num: p as i32,
        sector_count,
        sector_size,
        is_ready: 1,
        is_ssd,
        cmd_list,
        cmd_table,
        fis,
    });
    G_PORT_COUNT = idx + 1;

    0
}

pub unsafe fn init() {
    if G_AHCI_FOUND != 0 { return; }

    if ahci_find_controller() < 0 { return; }
    if G_ABAR.is_null() { return; }

    let mut ghc = reg_read(AHCI_GHC);
    ghc |= GHC_HR;
    reg_write(AHCI_GHC, ghc);
    while reg_read(AHCI_GHC) & GHC_HR != 0 { mdelay(1); }

    ghc = reg_read(AHCI_GHC);
    ghc |= GHC_AE;
    reg_write(AHCI_GHC, ghc);

    let cap = reg_read(AHCI_CAP);
    let ports_impl = reg_read(AHCI_PORTS);
    let max_ports = (cap & 0x1F) + 1;

    G_PORT_COUNT = 0;

    for p in 0..max_ports {
        if ports_impl & (1 << p) == 0 { continue; }
        ahci_init_port(p);
    }

    if G_PORT_COUNT > 0 {
        G_AHCI_FOUND = 1;
    }
}

unsafe fn ahci_find_port_index(port_num: i32) -> i32 {
    for i in 0..G_PORT_COUNT {
        if let Some(ref p) = G_PORTS[i as usize] {
            if p.port_num == port_num { return i; }
        }
    }
    -1
}

unsafe fn ahci_port_io(port: i32, cmd: u8, lba: u32, count: u32, buf_phys: u64) -> i32 {
    let idx = ahci_find_port_index(port);
    if idx < 0 { return -1; }
    let p = match G_PORTS[idx as usize].as_mut() {
        Some(p) => p,
        None => return -1,
    };
    if p.is_ready == 0 { return -1; }

    let ss = if p.sector_size == 0 { 512 } else { p.sector_size };

    ptr::write_bytes(p.cmd_table as *mut u8, 0, core::mem::size_of::<AhciCmdTable>());

    let cfis = &mut (*p.cmd_table).cfis;
    cfis[0] = 0x27;
    cfis[1] = 0x80;
    cfis[2] = cmd;
    cfis[3] = 0;
    cfis[4] = (lba >> 0) as u8;
    cfis[5] = (lba >> 8) as u8;
    cfis[6] = (lba >> 16) as u8;
    cfis[7] = 0x40 | (((lba >> 24) & 0x0F) as u8);
    cfis[8] = ((lba as u64) >> 32) as u8;
    cfis[9] = ((lba as u64) >> 40) as u8;
    for i in 10..15 { cfis[i] = 0; }
    cfis[12] = (count >> 0) as u8;
    cfis[13] = (count >> 8) as u8;
    cfis[14] = 0;
    cfis[15] = 0;

    let bytes = count * ss;
    let prd_entries = ((bytes + 0x400000 - 1) / 0x400000) as usize;
    if prd_entries > PRDT_MAX { return -1; }
    let prd_entries = if prd_entries < 1 { 1 } else { prd_entries };

    let mut remaining = bytes;
    for i in 0..prd_entries {
        (*p.cmd_table).prdt[i].dba = buf_phys + (i as u64) * 0x400000;
        let bc = if remaining > 0x400000 { 0x400000 } else { remaining };
        (*p.cmd_table).prdt[i].byte_count = bc - 1;
        remaining -= bc;
    }

    ptr::write_bytes(p.cmd_list as *mut u8, 0, core::mem::size_of::<AhciCmdList>());
    (*p.cmd_list).headers[0].dw0 = (5 << 0) | ((prd_entries as u32) << 16);
    (*p.cmd_list).headers[0].dw1 = bytes - 1;
    (*p.cmd_list).headers[0].ctba = p.cmd_table as u64;

    port_reg_write(port as u32, PORT_IS, 0xFFFFFFFF);
    port_reg_write(port as u32, PORT_CI, 1);
    while port_reg_read(port as u32, PORT_CI) & 1 != 0 { mdelay(1); }

    let tfd = port_reg_read(port as u32, PORT_TFD);
    if tfd & 0x7F != 0 { return -1; }

    0
}

pub unsafe fn read_sectors(lba: u32, count: u32, buffer: *mut u8) -> i32 {
    if G_PORT_COUNT < 1 { return -1; }
    let port_num = match G_PORTS[0].as_ref() {
        Some(p) => p.port_num,
        None => return -1,
    };
    let buf_phys = buffer as u64;
    ahci_port_io(port_num, 0x25, lba, count, buf_phys)
}

pub unsafe fn write_sectors(lba: u32, count: u32, buffer: *const u8) -> i32 {
    if G_PORT_COUNT < 1 { return -1; }
    let port_num = match G_PORTS[0].as_ref() {
        Some(p) => p.port_num,
        None => return -1,
    };
    let buf_phys = buffer as u64;
    ahci_port_io(port_num, 0x35, lba, count, buf_phys)
}

pub unsafe fn read_sectors_port(port: i32, lba: u32, count: u32, buffer: *mut u8) -> i32 {
    let buf_phys = buffer as u64;
    ahci_port_io(port, 0x25, lba, count, buf_phys)
}

pub unsafe fn write_sectors_port(port: i32, lba: u32, count: u32, buffer: *const u8) -> i32 {
    let buf_phys = buffer as u64;
    ahci_port_io(port, 0x35, lba, count, buf_phys)
}

pub fn get_port_count() -> i32 {
    unsafe { G_PORT_COUNT }
}

pub fn get_port_num(index: i32) -> i32 {
    unsafe {
        if index < 0 || index >= G_PORT_COUNT { return -1; }
        match G_PORTS[index as usize].as_ref() {
            Some(p) => p.port_num,
            None => -1,
        }
    }
}

pub fn get_port_sector_count(index: i32) -> u64 {
    unsafe {
        if index < 0 || index >= G_PORT_COUNT { return 0; }
        match G_PORTS[index as usize].as_ref() {
            Some(p) => p.sector_count,
            None => 0,
        }
    }
}

pub fn is_port_ready(index: i32) -> i32 {
    unsafe {
        if index < 0 || index >= G_PORT_COUNT { return 0; }
        match G_PORTS[index as usize].as_ref() {
            Some(p) => p.is_ready,
            None => 0,
        }
    }
}

pub fn get_port_ssd(index: i32) -> i32 {
    unsafe {
        if index < 0 || index >= G_PORT_COUNT { return 0; }
        match G_PORTS[index as usize].as_ref() {
            Some(p) => p.is_ssd,
            None => 0,
        }
    }
}

pub fn get_port_sector_size(index: i32) -> u32 {
    unsafe {
        if index < 0 || index >= G_PORT_COUNT { return 512; }
        match G_PORTS[index as usize].as_ref() {
            Some(p) => p.sector_size,
            None => 512,
        }
    }
}

pub fn get_sector_count() -> u64 {
    unsafe {
        if G_PORT_COUNT < 1 { return 0; }
        match G_PORTS[0].as_ref() {
            Some(p) => p.sector_count,
            None => 0,
        }
    }
}

pub fn is_ready() -> i32 {
    unsafe { if G_PORT_COUNT > 0 { 1 } else { 0 } }
}
