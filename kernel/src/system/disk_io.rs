use core::ffi::c_void;
use core::ptr;
use crate::globals;
use crate::drivers::ahci;

pub const MAX_DISKS: usize = 16;
pub const DISK_NAME_LEN: usize = 64;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct DiskInfo {
    pub present: bool,
    pub name: [u8; DISK_NAME_LEN],
    pub sector_count: u64,
    pub sector_size: u32,
    pub is_ahci: bool,
    pub is_removable: bool,
    pub is_ramdisk: bool,
    pub is_ssd: bool,
    pub ahci_port: i32,
    pub priv_ptr: *mut c_void,
}

static mut G_DISKS: [DiskInfo; MAX_DISKS] = [DiskInfo {
    present: false,
    name: [0u8; DISK_NAME_LEN],
    sector_count: 0,
    sector_size: 0,
    is_ahci: false,
    is_removable: false,
    is_ramdisk: false,
    is_ssd: false,
    ahci_port: -1,
    priv_ptr: ptr::null_mut(),
}; MAX_DISKS];
static mut G_DISK_COUNT: i32 = 0;

static mut G_CACHE_DISKS: [DiskInfo; MAX_DISKS] = [DiskInfo {
    present: false,
    name: [0u8; DISK_NAME_LEN],
    sector_count: 0,
    sector_size: 0,
    is_ahci: false,
    is_removable: false,
    is_ramdisk: false,
    is_ssd: false,
    ahci_port: -1,
    priv_ptr: ptr::null_mut(),
}; MAX_DISKS];
static mut G_CACHE_COUNT: i32 = 0;
static mut G_CACHE_VALID: bool = false;

unsafe fn disk_name_with_size(buf: &mut [u8; DISK_NAME_LEN], type_: &[u8], sector_count: u64, sector_size: u32) {
    let total_bytes = sector_count * (sector_size as u64);
    buf.fill(0);
    let mut pos = 0;
    let tlen = type_.len();
    buf[pos..pos + tlen].copy_from_slice(type_);
    pos += tlen;
    buf[pos] = b' ';
    pos += 1;
    if total_bytes >= (1024 * 1024 * 1024) {
        crate::system::util::lumie_itoa((total_bytes / (1024 * 1024 * 1024)) as i64, buf[pos..].as_mut_ptr(), 10);
        while buf[pos] != 0 { pos += 1; }
        let gb = b"GB (";
        buf[pos..pos + 4].copy_from_slice(gb);
        pos += 4;
        crate::system::util::lumie_itoa(sector_count as i64, buf[pos..].as_mut_ptr(), 10);
        while buf[pos] != 0 { pos += 1; }
        let sec = b" sectors)";
        buf[pos..pos + 9].copy_from_slice(sec);
    } else if total_bytes >= (1024 * 1024) {
        crate::system::util::lumie_itoa((total_bytes / (1024 * 1024)) as i64, buf[pos..].as_mut_ptr(), 10);
        while buf[pos] != 0 { pos += 1; }
        let mb = b"MB (";
        buf[pos..pos + 4].copy_from_slice(mb);
        pos += 4;
        crate::system::util::lumie_itoa(sector_count as i64, buf[pos..].as_mut_ptr(), 10);
        while buf[pos] != 0 { pos += 1; }
        let sec = b" sectors)";
        buf[pos..pos + 9].copy_from_slice(sec);
    } else {
        crate::system::util::lumie_itoa((total_bytes / 1024) as i64, buf[pos..].as_mut_ptr(), 10);
        while buf[pos] != 0 { pos += 1; }
        let kb = b"KB (";
        buf[pos..pos + 4].copy_from_slice(kb);
        pos += 4;
        crate::system::util::lumie_itoa(sector_count as i64, buf[pos..].as_mut_ptr(), 10);
        while buf[pos] != 0 { pos += 1; }
        let sec = b" sectors)";
        buf[pos..pos + 9].copy_from_slice(sec);
    }
}

unsafe fn add_ahci_disk(i: i32) -> i32 {
    let sc = ahci::get_port_sector_count(i);
    if sc == 0 {
        return -1;
    }
    let mut ss = ahci::get_port_sector_size(i);
    if ss == 0 {
        ss = 512;
    }
    let idx = G_DISK_COUNT as usize;
    G_DISK_COUNT += 1;
    let is_ssd = ahci::get_port_ssd(i) != 0;
    let type_name = if is_ssd { b"SSD" } else { b"HDD" };
    disk_name_with_size(&mut G_DISKS[idx].name, type_name, sc, ss);
    G_DISKS[idx].sector_count = sc;
    G_DISKS[idx].sector_size = ss;
    G_DISKS[idx].is_ahci = true;
    G_DISKS[idx].is_ssd = is_ssd;
    G_DISKS[idx].ahci_port = ahci::get_port_num(i);
    G_DISKS[idx].present = true;
    G_DISKS[idx].is_removable = false;
    0
}

unsafe fn disk_enum_ahci_only() -> i32 {
    let port_count = ahci::get_port_count();
    for i in 0..port_count {
        if (G_DISK_COUNT as usize) >= MAX_DISKS {
            break;
        }
        if ahci::is_port_ready(i) == 0 {
            continue;
        }
        let sc = ahci::get_port_sector_count(i);
        if sc == 0 {
            continue;
        }
        let ahci_port_num = ahci::get_port_num(i);
        let mut already_present = false;
        for j in 0..(G_DISK_COUNT as usize) {
            if G_DISKS[j].is_ahci && G_DISKS[j].ahci_port == ahci_port_num {
                already_present = true;
                break;
            }
        }
        if already_present {
            continue;
        }

        let mut replaced = false;
        for j in 0..(G_DISK_COUNT as usize) {
            if G_DISKS[j].present && G_DISKS[j].sector_count == sc
                && !G_DISKS[j].is_removable && !G_DISKS[j].is_ahci
            {
                G_DISKS[j].is_ahci = true;
                G_DISKS[j].ahci_port = ahci_port_num;
                G_DISKS[j].is_ssd = ahci::get_port_ssd(i) != 0;
                G_DISKS[j].is_removable = false;
                G_DISKS[j].priv_ptr = ptr::null_mut();
                let mut ss = ahci::get_port_sector_size(i);
                if ss == 0 {
                    ss = 512;
                }
                G_DISKS[j].sector_size = ss;
                let type_name = if G_DISKS[j].is_ssd { b"SSD" } else { b"HDD" };
                disk_name_with_size(&mut G_DISKS[j].name, type_name, sc, ss);
                replaced = true;
                break;
            }
        }
        if replaced {
            continue;
        }
        add_ahci_disk(i);
    }
    G_DISK_COUNT
}

pub unsafe fn disk_enum_all() -> i32 {
    G_DISK_COUNT = 0;
    for d in &mut G_DISKS {
        *d = DiskInfo {
            present: false,
            name: [0u8; DISK_NAME_LEN],
            sector_count: 0,
            sector_size: 0,
            is_ahci: false,
            is_removable: false,
            is_ramdisk: false,
            is_ssd: false,
            ahci_port: -1,
            priv_ptr: ptr::null_mut(),
        };
    }

    let bs = globals::get_bs();
    if bs.is_none() {
        if G_CACHE_VALID {
            for i in 0..(G_CACHE_COUNT as usize) {
                G_DISKS[i] = G_CACHE_DISKS[i];
            }
            G_DISK_COUNT = G_CACHE_COUNT;
            return G_DISK_COUNT;
        }
        return disk_enum_ahci_only();
    }

    let bs = bs.unwrap();
    let block_io_guid = crate::uefi::guid::EFI_BLOCK_IO_GUID;
    let loc = bs.locate_handle_buffer;
    let handle_proto = bs.handle_protocol;

    if let (Some(loc_fn), Some(hp_fn)) = (loc, handle_proto) {
        let mut handle_count: u64 = 0;
        let mut handles: *mut crate::uefi::types::efi_handle = ptr::null_mut();
        let st = loc_fn(
            0,
            &block_io_guid as *const crate::uefi::guid::EfiGuid,
            ptr::null_mut(),
            &mut handle_count,
            &mut handles as *mut *mut crate::uefi::types::efi_handle,
        );
        if st == crate::uefi::types::EFI_SUCCESS && !handles.is_null() && handle_count > 0 {
            for i in 0..handle_count {
                let idx = G_DISK_COUNT as usize;
                if idx >= MAX_DISKS {
                    break;
                }
                let mut bio: *mut crate::uefi::protocols::block_io::EfiBlockIoProtocol = ptr::null_mut();
                let st = hp_fn(
                    *handles.add(i as usize),
                    &block_io_guid as *const crate::uefi::guid::EfiGuid,
                    &mut bio as *mut *mut crate::uefi::protocols::block_io::EfiBlockIoProtocol as *mut *mut c_void,
                );
                if st != crate::uefi::types::EFI_SUCCESS || bio.is_null() {
                    continue;
                }
                let media = (*bio).media;
                if media.is_null() {
                    continue;
                }
                if (*media).logical_partition != 0 {
                    continue;
                }
                let sc = (*media).last_block + 1;
                if sc == 0 {
                    continue;
                }
                let ss = (*media).block_size as u32;
                let ss = if ss == 0 { 512 } else { ss };

                G_DISKS[idx].present = true;
                G_DISKS[idx].sector_count = sc;
                G_DISKS[idx].sector_size = ss;
                G_DISKS[idx].is_removable = (*media).removable_media != 0;
                G_DISKS[idx].priv_ptr = bio as *mut c_void;
                G_DISKS[idx].is_ahci = false;
                G_DISKS[idx].is_ssd = false;
                G_DISKS[idx].ahci_port = -1;

                let type_name = if (*media).removable_media != 0 { b"USB" } else { b"HDD" };
                disk_name_with_size(&mut G_DISKS[idx].name, type_name, sc, ss);
                G_DISK_COUNT += 1;
            }
            if handle_count > 0 && !handles.is_null() {
                if let Some(fp) = bs.free_pool {
                    fp(handles as *mut c_void);
                }
            }
        }
    }

    disk_enum_ahci_only();

    G_CACHE_COUNT = G_DISK_COUNT;
    for i in 0..(G_DISK_COUNT as usize) {
        G_CACHE_DISKS[i] = G_DISKS[i];
    }
    G_CACHE_VALID = true;

    G_DISK_COUNT
}

pub unsafe fn disk_get_info(index: i32) -> *const DiskInfo {
    if index < 0 || (index as usize) >= (G_DISK_COUNT as usize) {
        return ptr::null();
    }
    &G_DISKS[index as usize] as *const DiskInfo
}

pub unsafe fn disk_get_drive_letter(index: i32) -> u8 {
    if index < 0 || (index as usize) >= (G_DISK_COUNT as usize) {
        return 0;
    }
    let mut rem_before: i32 = 0;
    let mut nonrem_before: i32 = 0;
    for i in 0..(index as usize) {
        if G_DISKS[i].present {
            if G_DISKS[i].is_removable {
                rem_before += 1;
            } else {
                nonrem_before += 1;
            }
        }
    }
    if G_DISKS[index as usize].is_removable {
        if rem_before < 2 {
            return b'A' + rem_before as u8;
        }
        0
    } else {
        if nonrem_before < 24 {
            return b'C' + nonrem_before as u8;
        }
        0
    }
}
