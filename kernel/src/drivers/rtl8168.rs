use core::arch::asm;
use core::ptr::{copy_nonoverlapping, read_volatile, write_bytes, write_volatile};

const RTL_VENDOR_ID: u16 = 0x10EC;
const RTL_DEVICE_8168: u16 = 0x8168;
const RTL_DEVICE_8411: u16 = 0x8411;

const RTL_IDR0: u32 = 0x00;
const RTL_CHIP_CMD: u32 = 0x37;
const RTL_TXPOLL: u32 = 0x38;
const RTL_INTR_MASK: u32 = 0x3C;
const RTL_INTR_STAT: u32 = 0x3E;
const RTL_TX_CONFIG: u32 = 0x40;
const RTL_RX_CONFIG: u32 = 0x44;
const RTL_CFG_9346: u32 = 0x50;
const RTL_CONFIG1: u32 = 0x52;
const RTL_CONFIG5: u32 = 0x56;
const RTL_PHY_STATUS: u32 = 0x6C;
const RTL_TX_START_LO: u32 = 0x20;
const RTL_TX_START_HI: u32 = 0x24;
const RTL_EARLY_TX_THRES: u32 = 0x2C;
const RTL_RX_START_LO: u32 = 0xE4;
const RTL_RX_START_HI: u32 = 0xE8;
const RTL_RX_RING_LEN: u32 = 0xEA;
const RTL_TX_RING_LEN: u32 = 0xEE;
const RTL_CPLUS_CMD: u32 = 0xE0;

const RTL_9346_UNLOCK: u8 = 0xC0;
const RTL_9346_LOCK: u8 = 0x00;

const RTL_CMD_TX_ENB: u8 = 0x04;
const RTL_CMD_RX_ENB: u8 = 0x08;
const RTL_CMD_RESET: u8 = 0x10;

const RTL_TXCFG_MXDMA_UNLIMITED: u32 = 7 << 8;

const RTL_RXCFG_MXDMA_1024: u32 = 6 << 8;
const RTL_RXCFG_ACCEPT_BROADCAST: u32 = 1 << 3;
const RTL_RXCFG_ACCEPT_MYPHYS: u32 = 1 << 1;

const RTL_CPLUS_RXCHKSUM: u16 = 1 << 5;
#[allow(dead_code)]
const RTL_CPLUS_DAC: u32 = 1 << 24;
const RTL_CPLUS_VER_MAGIC_V1: u16 = 0x03 << 12;
const RTL_CPLUS_VER_MAGIC_V2: u16 = 0x07 << 12;

const RTL_HW_VER_THRESH_G: u16 = 0x40;

const RTL_CFG5_PHY_PWRDN: u8 = 1 << 3;

const RTL_PHY_LINK_UP: u8 = 1 << 0;

const RTL_RX_ERR_SUM: u32 = 1 << 22;
const RTL_RX_ERR_MASK: u32 = RTL_RX_ERR_SUM;

const RTL_OWN_BIT: u32 = 1 << 31;
const RTL_EOR_BIT: u32 = 1 << 30;
const RTL_FS_BIT: u32 = 1 << 29;
const RTL_LS_BIT: u32 = 1 << 28;

const RTL_TX_RING_SZ: usize = 4;
const RTL_RX_RING_SZ: usize = 4;
const RTL_PKT_BUF_SZ: u32 = 2048;

#[repr(C)]
#[derive(Clone, Copy)]
struct RtlDesc {
    status: u32,
    buf_lo: u32,
    buf_hi: u32,
    opts: u32,
}

const RTL_DESC_ZERO: RtlDesc = RtlDesc {
    status: 0,
    buf_lo: 0,
    buf_hi: 0,
    opts: 0,
};

#[repr(C, align(256))]
struct TxRing([RtlDesc; RTL_TX_RING_SZ]);

#[repr(C, align(256))]
struct RxRing([RtlDesc; RTL_RX_RING_SZ]);

#[repr(C, align(16))]
struct TxBuf([[u8; RTL_PKT_BUF_SZ as usize]; RTL_TX_RING_SZ]);

#[repr(C, align(16))]
struct RxBuf([[u8; RTL_PKT_BUF_SZ as usize]; RTL_RX_RING_SZ]);

static mut G_TX_RING: TxRing = TxRing([RTL_DESC_ZERO; RTL_TX_RING_SZ]);
static mut G_RX_RING: RxRing = RxRing([RTL_DESC_ZERO; RTL_RX_RING_SZ]);
static mut G_TX_BUF: TxBuf = TxBuf([[0; RTL_PKT_BUF_SZ as usize]; RTL_TX_RING_SZ]);
static mut G_RX_BUF: RxBuf = RxBuf([[0; RTL_PKT_BUF_SZ as usize]; RTL_RX_RING_SZ]);
static mut G_TX_CUR: usize = 0;
static mut G_RX_CUR: usize = 0;
static mut G_RTL_MAC: [u8; 6] = [0; 6];
static mut G_RTL_READY: i32 = 0;
static mut G_RTL_MMIO_BASE: u64 = 0;

fn pci_outl(port: u16, val: u32) {
    unsafe {
        asm!("outl %eax, %dx", in("eax") val, in("dx") port, options(att_syntax));
    }
}

fn pci_inl(port: u16) -> u32 {
    let val: u32;
    unsafe {
        asm!("inl %dx, %eax", out("eax") val, in("dx") port, options(att_syntax));
    }
    val
}

fn pci_cfg_read(bus: u8, dev: u8, func: u8, off: u8) -> u32 {
    let addr: u32 = 0x80000000
        | ((bus as u32) << 16)
        | (((dev as u32) & 0x1F) << 11)
        | (((func as u32) & 7) << 8)
        | ((off as u32) & 0xFC);
    pci_outl(0xCF8, addr);
    pci_inl(0xCFC)
}

fn pci_cfg_write(bus: u8, dev: u8, func: u8, off: u8, val: u32) {
    let addr: u32 = 0x80000000
        | ((bus as u32) << 16)
        | (((dev as u32) & 0x1F) << 11)
        | (((func as u32) & 7) << 8)
        | ((off as u32) & 0xFC);
    pci_outl(0xCF8, addr);
    pci_outl(0xCFC, val);
}

#[inline]
fn mmio_write8(val: u8, reg: u32) {
    unsafe {
        let base = G_RTL_MMIO_BASE;
        if base == 0 {
            return;
        }
        write_volatile((base + reg as u64) as *mut u8, val);
    }
}

#[inline]
fn mmio_write16(val: u16, reg: u32) {
    unsafe {
        let base = G_RTL_MMIO_BASE;
        if base == 0 {
            return;
        }
        write_volatile((base + reg as u64) as *mut u16, val);
    }
}

#[inline]
fn mmio_write32(val: u32, reg: u32) {
    unsafe {
        let base = G_RTL_MMIO_BASE;
        if base == 0 {
            return;
        }
        write_volatile((base + reg as u64) as *mut u32, val);
    }
}

#[inline]
fn mmio_read8(reg: u32) -> u8 {
    unsafe {
        let base = G_RTL_MMIO_BASE;
        if base == 0 {
            return 0;
        }
        read_volatile((base + reg as u64) as *const u8)
    }
}

#[inline]
fn mmio_read16(reg: u32) -> u16 {
    unsafe {
        let base = G_RTL_MMIO_BASE;
        if base == 0 {
            return 0;
        }
        read_volatile((base + reg as u64) as *const u16)
    }
}

#[inline]
fn mmio_read32(reg: u32) -> u32 {
    unsafe {
        let base = G_RTL_MMIO_BASE;
        if base == 0 {
            return 0;
        }
        read_volatile((base + reg as u64) as *const u32)
    }
}

fn get_pci_bar(bus: u8, dev: u8, func: u8, bar_index: i32) -> u64 {
    let bar_off: u8 = (0x10 + bar_index * 4) as u8;
    let bar = pci_cfg_read(bus, dev, func, bar_off);
    if bar == 0xFFFFFFFF || bar == 0 {
        return 0;
    }
    if (bar & 0x01) != 0 {
        return 0;
    }
    let mut addr: u64 = (bar as u64) & !0xF;
    if (bar & 0x06) == 0x04 {
        let bar_upper = pci_cfg_read(bus, dev, func, bar_off + 4);
        addr |= (bar_upper as u64) << 32;
    }
    addr
}

fn rtl_eeprom_unlock() {
    mmio_write8(RTL_9346_UNLOCK, RTL_CFG_9346);
    stall_us(150);
}

fn rtl_eeprom_lock() {
    mmio_write8(RTL_9346_LOCK, RTL_CFG_9346);
    stall_us(150);
}

unsafe fn rtl_init(bus: u8, dev: u8, func: u8, bar2: u64) -> i32 {
    G_RTL_READY = 0;
    G_RTL_MMIO_BASE = bar2;

    let cmd = (pci_cfg_read(bus, dev, func, 4) & 0xFFFF) as u16;
    pci_cfg_write(bus, dev, func, 4, (cmd | 0x107) as u32);

    if bar2 == 0 {
        return -1;
    }

    mmio_write8(RTL_CMD_RESET, RTL_CHIP_CMD);
    let mut timeout = 0;
    loop {
        if mmio_read8(RTL_CHIP_CMD) & RTL_CMD_RESET == 0 {
            break;
        }
        stall_us(1000);
        timeout += 1;
        if timeout > 100 {
            return -1;
        }
    }

    rtl_eeprom_unlock();

    for i in 0..6 {
        G_RTL_MAC[i as usize] = mmio_read8(RTL_IDR0 + i);
    }

    let cfg1 = mmio_read8(RTL_CONFIG1);
    mmio_write8((cfg1 & !0x30) | 0x01, RTL_CONFIG1);

    let cfg5 = mmio_read8(RTL_CONFIG5);
    mmio_write8(cfg5 & !RTL_CFG5_PHY_PWRDN, RTL_CONFIG5);

    stall_us(10000);
    rtl_eeprom_lock();
    stall_us(1000);

    let mut _mac_valid = 0;
    for i in 0..6 {
        if G_RTL_MAC[i] != 0x00 && G_RTL_MAC[i] != 0xFF {
            _mac_valid = 1;
            break;
        }
    }

    let mut link_timeout = 0;
    loop {
        if mmio_read8(RTL_PHY_STATUS) & RTL_PHY_LINK_UP != 0 {
            break;
        }
        stall_us(10000);
        link_timeout += 1;
        if link_timeout > 500 {
            break;
        }
    }

    let txc = mmio_read32(RTL_TX_CONFIG);
    let hw_ver = ((txc >> 22) & 0x3F) as u16;
    let cplus_magic = if hw_ver >= RTL_HW_VER_THRESH_G {
        RTL_CPLUS_VER_MAGIC_V2
    } else {
        RTL_CPLUS_VER_MAGIC_V1
    };

    mmio_write32(RTL_TXCFG_MXDMA_UNLIMITED, RTL_TX_CONFIG);
    mmio_write32(
        RTL_RXCFG_MXDMA_1024 | RTL_RXCFG_ACCEPT_BROADCAST | RTL_RXCFG_ACCEPT_MYPHYS,
        RTL_RX_CONFIG,
    );

    let cplus = mmio_read16(RTL_CPLUS_CMD);
    mmio_write16(cplus | cplus_magic | RTL_CPLUS_RXCHKSUM, RTL_CPLUS_CMD);

    mmio_write8(0x10, RTL_EARLY_TX_THRES);
    mmio_write16(0x0000, RTL_INTR_MASK);

    write_bytes(G_TX_RING.0.as_mut_ptr() as *mut u8, 0, core::mem::size_of::<TxRing>());
    write_bytes(G_RX_RING.0.as_mut_ptr() as *mut u8, 0, core::mem::size_of::<RxRing>());

    for i in 0..RTL_TX_RING_SZ {
        let addr = G_TX_BUF.0.as_ptr() as u64 + (i * RTL_PKT_BUF_SZ as usize) as u64;
        G_TX_RING.0[i] = RtlDesc {
            status: 0,
            buf_lo: addr as u32,
            buf_hi: (addr >> 32) as u32,
            opts: 0,
        };
    }
    G_TX_RING.0[RTL_TX_RING_SZ - 1].status |= RTL_EOR_BIT;

    for i in 0..RTL_RX_RING_SZ {
        let addr = G_RX_BUF.0.as_ptr() as u64 + (i * RTL_PKT_BUF_SZ as usize) as u64;
        write_bytes(G_RX_BUF.0[i].as_mut_ptr(), 0, RTL_PKT_BUF_SZ as usize);
        G_RX_RING.0[i] = RtlDesc {
            status: RTL_OWN_BIT | RTL_PKT_BUF_SZ,
            buf_lo: addr as u32,
            buf_hi: (addr >> 32) as u32,
            opts: 0,
        };
    }
    G_RX_RING.0[RTL_RX_RING_SZ - 1].status |= RTL_EOR_BIT;

    let tx_ring_addr = G_TX_RING.0.as_ptr() as u64;
    let rx_ring_addr = G_RX_RING.0.as_ptr() as u64;
    mmio_write32(tx_ring_addr as u32, RTL_TX_START_LO);
    mmio_write32((tx_ring_addr >> 32) as u32, RTL_TX_START_HI);
    mmio_write32(rx_ring_addr as u32, RTL_RX_START_LO);
    mmio_write32((rx_ring_addr >> 32) as u32, RTL_RX_START_HI);

    mmio_write16(RTL_TX_RING_SZ as u16, RTL_TX_RING_LEN);
    mmio_write16(RTL_RX_RING_SZ as u16, RTL_RX_RING_LEN);

    G_TX_CUR = 0;
    G_RX_CUR = 0;

    let txrx = mmio_read8(RTL_CHIP_CMD);
    mmio_write8(txrx | RTL_CMD_TX_ENB | RTL_CMD_RX_ENB, RTL_CHIP_CMD);

    mmio_write16(0xFFFF, RTL_INTR_STAT);

    G_RTL_READY = 1;
    0
}

unsafe fn rtl_send(buf: *const u8, len: u32) -> i32 {
    if G_RTL_READY == 0 {
        return -1;
    }
    if len > RTL_PKT_BUF_SZ {
        return -1;
    }

    let idx = G_TX_CUR;

    let mut timeout = 50000;
    loop {
        if G_TX_RING.0[idx].status & RTL_OWN_BIT == 0 {
            break;
        }
        if mmio_read8(RTL_PHY_STATUS) & RTL_PHY_LINK_UP == 0 {
            G_TX_RING.0[idx].status = 0;
            return -1;
        }
        stall_us(5);
        timeout -= 1;
        if timeout <= 0 {
            G_TX_RING.0[idx].status = 0;
            return -1;
        }
    }

    copy_nonoverlapping(buf, G_TX_BUF.0[idx].as_mut_ptr(), len as usize);
    asm!("", options(nostack)); // compiler barrier

    let mut status = RTL_OWN_BIT | RTL_FS_BIT | RTL_LS_BIT | len;
    if idx == RTL_TX_RING_SZ - 1 {
        status |= RTL_EOR_BIT;
    }
    G_TX_RING.0[idx].status = status;

    asm!("sfence", options(nostack));

    mmio_write8(0xF0, RTL_TXPOLL);

    G_TX_CUR = (idx + 1) % RTL_TX_RING_SZ;
    0
}

unsafe fn rtl_recv(buf: *mut u8, len: *mut u32) -> i32 {
    if G_RTL_READY == 0 {
        return -1;
    }

    let idx = G_RX_CUR;

    if G_RX_RING.0[idx].status & RTL_OWN_BIT != 0 {
        return -1;
    }

    let status_word = G_RX_RING.0[idx].status;
    let mut pkt_len = status_word & 0x3FFF;

    if status_word & RTL_RX_ERR_MASK != 0 {
        write_bytes(G_RX_BUF.0[idx].as_mut_ptr(), 0, RTL_PKT_BUF_SZ as usize);
        asm!("mfence", options(nostack));
        G_RX_RING.0[idx].status = RTL_OWN_BIT | RTL_PKT_BUF_SZ;
        if idx == RTL_RX_RING_SZ - 1 {
            G_RX_RING.0[idx].status |= RTL_EOR_BIT;
        }
        G_RX_CUR = (idx + 1) % RTL_RX_RING_SZ;
        return -1;
    }

    if pkt_len > RTL_PKT_BUF_SZ {
        pkt_len = RTL_PKT_BUF_SZ;
    }

    if pkt_len < 12 {
        write_bytes(G_RX_BUF.0[idx].as_mut_ptr(), 0, RTL_PKT_BUF_SZ as usize);
        asm!("mfence", options(nostack));
        G_RX_RING.0[idx].status = RTL_OWN_BIT | RTL_PKT_BUF_SZ;
        if idx == RTL_RX_RING_SZ - 1 {
            G_RX_RING.0[idx].status |= RTL_EOR_BIT;
        }
        G_RX_CUR = (idx + 1) % RTL_RX_RING_SZ;
        return -1;
    }

    if !len.is_null() {
        *len = pkt_len;
    }
    if !buf.is_null() && pkt_len > 0 {
        copy_nonoverlapping(G_RX_BUF.0[idx].as_ptr(), buf, pkt_len as usize);
    }

    write_bytes(G_RX_BUF.0[idx].as_mut_ptr(), 0, RTL_PKT_BUF_SZ as usize);
    asm!("mfence", options(nostack));
    G_RX_RING.0[idx].status = RTL_OWN_BIT | RTL_PKT_BUF_SZ;
    if idx == RTL_RX_RING_SZ - 1 {
        G_RX_RING.0[idx].status |= RTL_EOR_BIT;
    }
    G_RX_CUR = (idx + 1) % RTL_RX_RING_SZ;
    0
}

fn rtl_probe() -> i32 {
    let mut bus: u8 = 0;
    loop {
        let hdr0 = pci_cfg_read(bus, 0, 0, 0);
        if hdr0 != 0xFFFFFFFF {
            let mut dev: u8 = 0;
            loop {
                let id = pci_cfg_read(bus, dev, 0, 0);
                if id != 0xFFFFFFFF && id != 0 {
                    let ven = (id & 0xFFFF) as u16;
                    let dev_id = (id >> 16) as u16;
                    if ven == RTL_VENDOR_ID
                        && (dev_id == RTL_DEVICE_8168 || dev_id == RTL_DEVICE_8411)
                    {
                        let mf = (pci_cfg_read(bus, dev, 0, 0xC) >> 23) & 1;
                        let maxf: u8 = if mf != 0 { 8 } else { 1 };
                        let mut func: u8 = 0;
                        loop {
                            let fid = if func == 0 {
                                id
                            } else {
                                pci_cfg_read(bus, dev, func, 0)
                            };
                            if (fid & 0xFFFF) as u16 == RTL_VENDOR_ID {
                                let cr = pci_cfg_read(bus, dev, func, 8);
                                if (cr & 0xFFFFFF) == 0x020000 {
                                    let bar2 = get_pci_bar(bus, dev, func, 2);
                                    let bar0 = get_pci_bar(bus, dev, func, 0);
                                    let mmio_addr = if bar2 != 0 { bar2 } else { bar0 };
                                    if mmio_addr != 0 {
                                        unsafe {
                                            return rtl_init(bus, dev, func, mmio_addr);
                                        }
                                    }
                                }
                            }
                            func += 1;
                            if func >= maxf {
                                break;
                            }
                        }
                    }
                }
                dev += 1;
                if dev >= 32 {
                    break;
                }
            }
        }
        bus += 1;
        if bus >= 255 {
            break;
        }
    }
    -1
}

fn stall_us(micros: u32) {
    for _ in 0..micros {
        for _ in 0..50 {
            unsafe {
                asm!("pause", options(nostack));
            }
        }
    }
}

pub unsafe fn rtl8168_init() -> i32 {
    rtl_probe()
}

pub unsafe fn rtl8168_is_ready() -> i32 {
    G_RTL_READY
}

pub unsafe fn rtl8168_send_packet(data: *const u8, len: u32) -> i32 {
    rtl_send(data, len)
}

pub unsafe fn rtl8168_receive_packet(buf: *mut u8, max_len: *mut u32) -> i32 {
    rtl_recv(buf, max_len)
}

pub unsafe fn rtl8168_get_mac(mac: *mut u8) -> i32 {
    if mac.is_null() {
        return -1;
    }
    if G_RTL_READY == 0 {
        return -1;
    }
    copy_nonoverlapping(G_RTL_MAC.as_ptr(), mac, 6);
    0
}
