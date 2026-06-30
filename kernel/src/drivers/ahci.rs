#![no_std]

extern "C" {
    fn ahci_init();
    fn ahci_is_ready() -> i32;
    fn ahci_get_port_count() -> i32;
    fn ahci_is_port_ready(port: i32) -> i32;
    fn ahci_get_port_sector_count(port: i32) -> u64;
    fn ahci_get_port_sector_size(port: i32) -> u32;
    fn ahci_get_port_ssd(port: i32) -> i32;
    fn ahci_get_port_num(port: i32) -> i32;
    fn ahci_get_sector_count() -> u64;
}

pub unsafe fn init() {
    ahci_init();
}

pub fn is_ready() -> i32 {
    unsafe { ahci_is_ready() }
}

pub fn get_port_count() -> i32 {
    unsafe { ahci_get_port_count() }
}

pub fn is_port_ready(port: i32) -> i32 {
    unsafe { ahci_is_port_ready(port) }
}

pub fn get_port_sector_count(port: i32) -> u64 {
    unsafe { ahci_get_port_sector_count(port) }
}

pub fn get_port_sector_size(port: i32) -> u32 {
    unsafe { ahci_get_port_sector_size(port) }
}

pub fn get_port_ssd(port: i32) -> i32 {
    unsafe { ahci_get_port_ssd(port) }
}

pub fn get_port_num(port: i32) -> i32 {
    unsafe { ahci_get_port_num(port) }
}

pub fn get_sector_count() -> u64 {
    unsafe { ahci_get_sector_count() }
}
