#![allow(dead_code)]
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::ptr;

macro_rules! read_packed {
    ($e:expr) => { ptr::read_unaligned(core::ptr::addr_of!($e)) }
}

use crate::uefi::guid::EfiGuid;
use crate::uefi::guid::EFI_BLOCK_IO_GUID;
use crate::uefi::protocols::block_io::EfiBlockIoProtocol;
use crate::uefi::tables::EfiBootServices;
use crate::uefi::types::*;
use crate::fs::types::LumieDirEnt;
use crate::fs::diskio::DiskIo;
use crate::drivers::ahci as ahci_drv;

const OEM_NTFS: [u8; 8] = *b"NTFS    ";

const ATTR_STANDARD_INFORMATION: u32 = 0x10;
const ATTR_ATTRIBUTE_LIST: u32 = 0x20;
const ATTR_FILE_NAME: u32 = 0x30;
const ATTR_OBJECT_ID: u32 = 0x40;
const ATTR_SECURITY_DESCRIPTOR: u32 = 0x50;
const ATTR_VOLUME_NAME: u32 = 0x60;
const ATTR_VOLUME_INFO: u32 = 0x70;
const ATTR_DATA: u32 = 0x80;
const ATTR_INDEX_ROOT: u32 = 0x90;
const ATTR_INDEX_ALLOCATION: u32 = 0xA0;
const ATTR_BITMAP: u32 = 0xB0;
const ATTR_REPARSE_POINT: u32 = 0xC0;
const ATTR_EA_INFORMATION: u32 = 0xD0;
const ATTR_EA: u32 = 0xE0;
const ATTR_PROPERTY_SET: u32 = 0xF0;
const ATTR_LOGGED_UTILITY_STREAM: u32 = 0x100;

const MFT_ENTRY_MFT: u64 = 0;
const MFT_ENTRY_ROOT: u64 = 5;

const INDEX_ENTRY_LAST: u16 = 0x01;
const INDEX_ENTRY_SUBNODE: u16 = 0x02;

const MFT_RECORD_IN_USE: u16 = 0x0001;
const MFT_RECORD_DIR: u16 = 0x0002;

const NAMESPACE_POSIX: u8 = 0;
const NAMESPACE_WIN32: u8 = 1;
const NAMESPACE_DOS: u8 = 2;
const NAMESPACE_WIN32_DOS: u8 = 3;

const FILE_ATTR_READONLY: u32 = 0x00000001;
const FILE_ATTR_HIDDEN: u32 = 0x00000002;
const FILE_ATTR_SYSTEM: u32 = 0x00000004;
const FILE_ATTR_DIRECTORY: u32 = 0x00000010;
const FILE_ATTR_ARCHIVE: u32 = 0x00000020;
const FILE_ATTR_NORMAL: u32 = 0x00000080;
const FILE_ATTR_TEMPORARY: u32 = 0x00000100;

#[repr(C, packed)]
struct NtfsBpb {
    jmp: [u8; 3],
    oem: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    unused1: [u8; 3],
    unused2: u16,
    media_descriptor: u8,
    unused3: [u8; 2],
    sectors_per_track: u16,
    num_heads: u16,
    hidden_sectors: u32,
    unused4: [u8; 8],
    total_sectors: u64,
    mft_lcn: u64,
    mft_mirror_lcn: u64,
    clusters_per_mft_record: i8,
    clusters_per_index_block: i8,
    volume_serial: u32,
    checksum: u32,
}

#[repr(C, packed)]
struct MftRecordHeader {
    signature: [u8; 4],
    fixup_off: u16,
    fixup_count: u16,
    lsn: u64,
    seq_number: u16,
    link_count: u16,
    attr_off: u16,
    flags: u16,
    used_size: u32,
    alloc_size: u32,
    base_record: u64,
    next_attr_id: u16,
    _pad: u16,
    record_number: u32,
}

#[repr(C, packed)]
struct AttrHeader {
    type_code: u32,
    length: u32,
    non_resident: u8,
    name_len: u8,
    name_off: u16,
    flags: u16,
    attr_id: u16,
}

#[repr(C, packed)]
struct ResidentAttr {
    hdr: AttrHeader,
    value_len: u32,
    value_off: u16,
    flags: u8,
    _pad: u8,
}

#[repr(C, packed)]
struct NonResidentAttr {
    hdr: AttrHeader,
    first_vcn: u64,
    last_vcn: u64,
    run_off: u16,
    compr_size: u16,
    _pad: [u8; 4],
    alloc_size: u64,
    data_size: u64,
    init_size: u64,
}

#[repr(C, packed)]
struct IndexRoot {
    attr_type: u32,
    collation: u32,
    index_block_size: u32,
    clusters_per_index: u8,
    _pad: [u8; 3],
    entries_off: u32,
    entries_size: u32,
    flags: u32,
}

#[repr(C, packed)]
struct IndexEntryHeader {
    mft_ref: u64,
    entry_len: u16,
    stream_len: u16,
    flags: u16,
    _pad: u16,
}

#[repr(C, packed)]
struct FileNameAttr {
    parent_ref: u64,
    ctime: u64,
    mtime: u64,
    mft_mtime: u64,
    atime: u64,
    alloc_size: u64,
    data_size: u64,
    file_flags: u32,
    _pad: u32,
    name_len: u8,
    namespace: u8,
}

pub struct Ntfs {
    pub initialized: bool,
    bytes_per_sector: u32,
    sectors_per_cluster: u8,
    bytes_per_cluster: u32,
    mft_lcn: u64,
    clusters_per_mft_record: i8,
    clusters_per_index_block: i8,
    mft_record_size: u32,
    index_block_size: u32,
    total_sectors: u64,
    pub disk_io: DiskIo,
}

static mut NTFS_DRIVER: MaybeUninit<Ntfs> = MaybeUninit::uninit();
static mut G_BS: *mut EfiBootServices = ptr::null_mut();

unsafe fn ntfs() -> &'static mut Ntfs {
    &mut *NTFS_DRIVER.as_mut_ptr()
}

unsafe fn compute_record_size(clusters_per: i8, bpc: u32) -> u32 {
    if clusters_per > 0 {
        (clusters_per as u32) * bpc
    } else {
        1u32 << (-(clusters_per as i32)) as u32
    }
}

unsafe fn read_sectors(lba: u64, count: u32, buffer: *mut u8) -> i32 {
    let disk = (*ntfs()).disk_io;
    if let Some(cb) = disk.read_cb {
        return cb(lba as u32, count, buffer);
    }
    if disk.use_ahci {
        return ahci_drv::read_sectors(lba as u32, count, buffer);
    }
    if let Some(block_io) = disk.block_io {
        let sector_size = 512u64;
        let media = (*block_io).media;
        let status = ((*block_io).read_blocks.unwrap())(
            block_io as *mut c_void,
            (*media).media_id,
            lba,
            (count as u64) * sector_size,
            buffer as *mut c_void,
        );
        if efi_error(status) {
            return -1;
        }
        return 0;
    }
    -1
}

unsafe fn read_clusters(vcn: u64, buffer: *mut u8) -> i32 {
    let ntfs = &(*ntfs());
    let lba = vcn * (ntfs.sectors_per_cluster as u64);
    read_sectors(lba, ntfs.sectors_per_cluster as u32, buffer)
}

unsafe fn read_bytes(lba: u64, byte_off: u32, buffer: *mut u8, len: u32) -> i32 {
    let ntfs = &(*ntfs());
    let sector_buf_size = ntfs.bytes_per_sector as usize;
    let mut tmp = [0u8; 4096];
    if len as usize > tmp.len() {
        return -1;
    }
    if (byte_off as usize + len as usize) > sector_buf_size {
        let mut off = byte_off;
        let mut remaining = len;
        let mut dst = buffer;
        while remaining > 0 {
            let sector_lba = lba + (off / ntfs.bytes_per_sector) as u64;
            let sector_off = (off % ntfs.bytes_per_sector) as usize;
            let chunk = (ntfs.bytes_per_sector - sector_off as u32).min(remaining);
            if read_sectors(sector_lba, 1, tmp.as_mut_ptr()) != 0 {
                return -1;
            }
            ptr::copy_nonoverlapping(tmp.as_ptr().add(sector_off), dst, chunk as usize);
            off += chunk;
            remaining -= chunk;
            dst = dst.add(chunk as usize);
        }
        0
    } else {
        let sector_lba = lba + (byte_off / ntfs.bytes_per_sector) as u64;
        if read_sectors(sector_lba, 1, tmp.as_mut_ptr()) != 0 {
            return -1;
        }
        let off = (byte_off % ntfs.bytes_per_sector) as usize;
        ptr::copy_nonoverlapping(tmp.as_ptr().add(off), buffer, len as usize);
        0
    }
}

#[repr(C, packed)]
struct Fixup {
    sig: u16,
    entries: [u16; 0],
}

unsafe fn apply_fixup(record: *mut u8, record_size: u32) -> i32 {
    let fixup_off = ptr::read_unaligned(core::ptr::addr_of!((*(record as *const MftRecordHeader)).fixup_off)) as usize;
    let fixup_count = ptr::read_unaligned(core::ptr::addr_of!((*(record as *const MftRecordHeader)).fixup_count)) as usize;
    if fixup_off == 0 || fixup_count < 2 || fixup_off >= record_size as usize {
        return -1;
    }
    let fixup_sig = ptr::read_unaligned(record.add(fixup_off) as *const u16);
    let entry_count = fixup_count - 1;
    let sector_size = (*ntfs()).bytes_per_sector as u32;
    for i in 0..entry_count {
        let sector_end = ((i as u32 + 1) * sector_size) - 2;
        if sector_end + 2 > record_size {
            return -1;
        }
        let fixup_entry = ptr::read_unaligned(
            record.add(fixup_off + 2 + i * 2) as *const u16,
        );
        let target = record.add(sector_end as usize) as *mut u16;
        ptr::write_unaligned(target, fixup_entry);
    }
    let end_sig = record.add(record_size as usize - 2) as *mut u16;
    ptr::write_unaligned(end_sig, fixup_sig);
    0
}

unsafe fn mft_read_record(record_num: u64, buffer: *mut u8) -> i32 {
    let ntfs = &(*ntfs());
    let mft_cluster = ntfs.mft_lcn;
    let record_size = ntfs.mft_record_size;
    let records_per_cluster = ntfs.bytes_per_cluster / record_size;
    let cluster_off = record_num / records_per_cluster as u64;
    let cluster = mft_cluster + cluster_off;
    let cluster_lba = cluster * (ntfs.sectors_per_cluster as u64);
    let intra_off = (record_num % records_per_cluster as u64) * (record_size as u64);
    let sector_lba = cluster_lba + intra_off / (ntfs.bytes_per_sector as u64);
    let sector_count = ((intra_off % ntfs.bytes_per_sector as u64) + record_size as u64 + ntfs.bytes_per_sector as u64 - 1) / ntfs.bytes_per_sector as u64;
    if read_sectors(sector_lba, sector_count as u32, buffer) != 0 {
        return -1;
    }
    let intra_sector_off = (intra_off % ntfs.bytes_per_sector as u64) as usize;
    if intra_sector_off > 0 {
        let mut aligned = [0u8; 4096];
        let copy_size = record_size.min(4096);
        ptr::copy_nonoverlapping(buffer.add(intra_sector_off), aligned.as_mut_ptr(), copy_size as usize);
        ptr::copy_nonoverlapping(aligned.as_ptr(), buffer, copy_size as usize);
    }
    let sig = ptr::read_unaligned(buffer as *const [u8; 4]);
    if &sig != b"FILE" {
        return -2;
    }
    apply_fixup(buffer, record_size)
}

unsafe fn attr_find(record: *const u8, record_size: u32, type_code: u32, index: u32) -> *const u8 {
    let attr_off = read_packed!((*(record as *const MftRecordHeader)).attr_off) as usize;
    if attr_off as u32 >= record_size {
        return ptr::null();
    }
    let mut pos = record.add(attr_off);
    let mut idx = 0u32;
    loop {
        let hdr = pos as *const AttrHeader;
        let t = read_packed!((*hdr).type_code);
        let len = read_packed!((*hdr).length) as usize;
        if t == 0xFFFFFFFF || len == 0 || (pos as usize + len) > (record as usize + record_size as usize) {
            break;
        }
        if t == type_code {
            if idx == index {
                return pos;
            }
            idx += 1;
        }
        pos = pos.add(len);
    }
    ptr::null()
}

unsafe fn attr_is_non_resident(attr: *const u8) -> bool {
    read_packed!((*(attr as *const AttrHeader)).non_resident) != 0
}

unsafe fn attr_name(attr: *const u8) -> *const u16 {
    let hdr = attr as *const AttrHeader;
    let off = read_packed!((*hdr).name_off) as usize;
    attr.add(off) as *const u16
}

unsafe fn attr_name_len(attr: *const u8) -> u8 {
    read_packed!((*(attr as *const AttrHeader)).name_len)
}

unsafe fn resident_value(attr: *const u8) -> (*const u8, u32) {
    let r = attr as *const ResidentAttr;
    let off = read_packed!((*r).value_off) as usize;
    let len = read_packed!((*r).value_len);
    (attr.add(off), len)
}

unsafe fn resident_value_mut(attr: *mut u8) -> (*mut u8, u32) {
    let r = attr as *mut ResidentAttr;
    let off = read_packed!((*r).value_off) as usize;
    let len = read_packed!((*r).value_len);
    (attr.add(off), len)
}

unsafe fn non_resident_info(attr: *const u8) -> (u64, u64, u64, u64, u16) {
    let nr = attr as *const NonResidentAttr;
    (
        read_packed!((*nr).first_vcn),
        read_packed!((*nr).last_vcn),
        read_packed!((*nr).data_size),
        read_packed!((*nr).init_size),
        read_packed!((*nr).run_off),
    )
}

unsafe fn parse_dataruns(runs: *const u8, run_size: u32, lcns: &mut [u64], counts: &mut [u64]) -> i32 {
    let mut pos = runs;
    let end = runs.add(run_size as usize);
    let mut current_vcn: i64 = 0;
    let mut idx = 0usize;
    while (pos as usize) < (end as usize) {
        let header = ptr::read_unaligned(pos);
        if header == 0 {
            break;
        }
        let count_bytes = (header & 0x0F) as usize;
        let offset_bytes = ((header >> 4) & 0x0F) as usize;
        pos = pos.add(1);
        if (pos as usize + count_bytes + offset_bytes) > (end as usize) {
            return -1;
        }
        let mut cluster_count: u64 = 0;
        for i in 0..count_bytes {
            cluster_count |= (ptr::read_unaligned(pos.add(i)) as u64) << (i * 8);
        }
        pos = pos.add(count_bytes);
        let mut lcn_delta: i64 = 0;
        let sign_extend = if offset_bytes > 0 && (ptr::read_unaligned(pos.add(offset_bytes - 1)) & 0x80) != 0 {
            -1i64
        } else {
            0i64
        };
        for i in 0..offset_bytes {
            lcn_delta |= (ptr::read_unaligned(pos.add(i)) as i64) << (i * 8);
        }
        if offset_bytes > 0 {
            lcn_delta |= sign_extend << (offset_bytes * 8);
        }
        pos = pos.add(offset_bytes);
        current_vcn = current_vcn.wrapping_add(lcn_delta);
        if idx < lcns.len() {
            lcns[idx] = current_vcn as u64;
        }
        if idx < counts.len() {
            counts[idx] = cluster_count;
        }
        idx += 1;
    }
    idx as i32
}

unsafe fn get_data_runs(attr: *const u8) -> ([u64; 8], usize) {
    let nr = attr as *const NonResidentAttr;
    let run_off = read_packed!((*nr).run_off) as usize;
    let runs = attr.add(run_off);
    let mut lcns = [0u64; 8];
    let mut counts = [0u64; 8];
    let mut pos = runs;
    let mut current_vcn: u64 = 0;
    let mut idx = 0usize;
    loop {
        if idx >= 8 { break; }
        let header = ptr::read_unaligned(pos);
        if header == 0 { break; }
        let count_bytes = (header & 0x0F) as usize;
        let offset_bytes = ((header >> 4) & 0x0F) as usize;
        pos = pos.add(1);
        let mut cluster_count: u64 = 0;
        for i in 0..count_bytes {
            if pos.add(i).is_null() { break; }
            cluster_count |= (ptr::read_unaligned(pos.add(i)) as u64) << (i * 8);
        }
        pos = pos.add(count_bytes);
        if count_bytes == 0 { break; }
        let mut lcn_delta: i64 = 0;
        if offset_bytes > 0 {
            for i in 0..offset_bytes {
                if pos.add(i).is_null() { break; }
                lcn_delta |= (ptr::read_unaligned(pos.add(i)) as i64) << (i * 8);
            }
            let sign_extend = if (ptr::read_unaligned(pos.add(offset_bytes - 1)) & 0x80) != 0 {
                -1i64
            } else {
                0i64
            };
            lcn_delta |= sign_extend << (offset_bytes * 8);
        }
        pos = pos.add(offset_bytes);
        if offset_bytes == 0 { break; }
        let lcn = (current_vcn as i64).wrapping_add(lcn_delta) as u64;
        if idx < 8 {
            lcns[idx] = lcn;
            counts[idx] = cluster_count;
        }
        current_vcn = current_vcn.wrapping_add(cluster_count);
        idx += 1;
    }
    (lcns, idx)
}

unsafe fn read_non_resident_data(
    attr: *const u8,
    buffer: *mut u8,
    offset: u64,
    length: u32,
) -> i32 {
    let (first_vcn, _last_vcn, data_size, _init_size, run_off) = non_resident_info(attr);
    if offset >= data_size {
        return 0;
    }
    let ntfs = &(*ntfs());
    let bpc = ntfs.bytes_per_cluster as u64;
    let mut lcns = [0u64; 256];
    let mut counts = [0u64; 256];
    let nr = attr as *const NonResidentAttr;
    let runs = attr.add(read_packed!((*nr).run_off) as usize);
    let run_area_size = read_packed!((*nr).hdr.length) as u32 - run_off as u32;
    let num_runs = parse_dataruns(runs, run_area_size, &mut lcns, &mut counts);
    if num_runs <= 0 {
        return -1;
    }
    let mut remaining = length;
    let mut dst = buffer;
    let mut file_off = offset;
    for i in 0..num_runs as usize {
        let run_start_vcn = if i == 0 { first_vcn } else { lcns[i - 1] + counts[i - 1] };
        let run_end_vcn = run_start_vcn + counts[i];
        let run_lcn = lcns[i];
        let run_start_off = run_start_vcn * bpc;
        let run_end_off = run_end_vcn * bpc;
        if file_off >= run_end_off {
            continue;
        }
        let read_start = if file_off > run_start_off { file_off } else { run_start_off };
        let read_end = data_size.min(run_end_off);
        if read_start >= read_end {
            break;
        }
        let chunk = (read_end - read_start).min(remaining as u64) as u32;
        if chunk == 0 {
            break;
        }
        let cluster_off = read_start - run_start_off;
        let lcn = run_lcn + cluster_off / bpc;
        let intra_cluster_off = (cluster_off % bpc) as u32;
        let sector_lba = lcn * (ntfs.sectors_per_cluster as u64) + (intra_cluster_off as u64) / (ntfs.bytes_per_sector as u64);
        let sector_off = (intra_cluster_off % ntfs.bytes_per_sector) as u16;
        if sector_off == 0 && (chunk % ntfs.bytes_per_sector) == 0 {
            let sector_count = chunk / ntfs.bytes_per_sector;
            if read_sectors(sector_lba, sector_count, dst) != 0 {
                return -1;
            }
        } else {
            let mut tmp = [0u8; 4096];
            let to_read = chunk.min(4096);
            let sectors_needed = (sector_off as u32 + to_read + ntfs.bytes_per_sector - 1) / ntfs.bytes_per_sector;
            if read_sectors(sector_lba, sectors_needed, tmp.as_mut_ptr()) != 0 {
                return -1;
            }
            ptr::copy_nonoverlapping(tmp.as_ptr().add(sector_off as usize), dst, to_read as usize);
        }
        remaining -= chunk;
        dst = dst.add(chunk as usize);
        file_off = read_start + chunk as u64;
        if remaining == 0 {
            break;
        }
    }
    (length - remaining) as i32
}

pub unsafe fn init() -> i32 {
    let drv = &mut *NTFS_DRIVER.as_mut_ptr();
    if drv.initialized {
        return 0;
    }
    let mut bpb_buf = [0u8; 512];
    if read_sectors(0, 1, bpb_buf.as_mut_ptr()) != 0 {
        return -1;
    }
    let bpb = bpb_buf.as_ptr() as *const NtfsBpb;
    let oem = read_packed!((*bpb).oem);
    if oem != OEM_NTFS {
        return -2;
    }
    let bytes_per_sector = read_packed!((*bpb).bytes_per_sector) as u32;
    let sectors_per_cluster = read_packed!((*bpb).sectors_per_cluster);
    let mft_lcn = read_packed!((*bpb).mft_lcn);
    let clusters_per_mft_record = read_packed!((*bpb).clusters_per_mft_record);
    let clusters_per_index_block = read_packed!((*bpb).clusters_per_index_block);
    let total_sectors = read_packed!((*bpb).total_sectors);
    let bytes_per_cluster = bytes_per_sector * (sectors_per_cluster as u32);
    let mft_record_size = compute_record_size(clusters_per_mft_record, bytes_per_cluster);
    let index_block_size = compute_record_size(clusters_per_index_block, bytes_per_cluster);
    drv.bytes_per_sector = bytes_per_sector;
    drv.sectors_per_cluster = sectors_per_cluster;
    drv.bytes_per_cluster = bytes_per_cluster;
    drv.mft_lcn = mft_lcn;
    drv.clusters_per_mft_record = clusters_per_mft_record;
    drv.clusters_per_index_block = clusters_per_index_block;
    drv.mft_record_size = mft_record_size;
    drv.index_block_size = index_block_size;
    drv.total_sectors = total_sectors;
    drv.initialized = true;
    0
}

unsafe fn get_attr_value(record: *const u8, record_size: u32, attr_type: u32, index: u32) -> i32 {
    let attr = attr_find(record, record_size, attr_type, index);
    if attr.is_null() {
        return -1;
    }
    0
}

unsafe fn read_file_name(attr: *const u8, name_buf: &mut [u8]) -> i32 {
    let (value, value_len) = resident_value(attr);
    if value_len < 66 {
        return -1;
    }
    let fn_attr = value as *const FileNameAttr;
    let name_len = read_packed!((*fn_attr).name_len) as usize;
    let _namespace = read_packed!((*fn_attr).namespace);
    if name_len == 0 {
        return 0;
    }
    let src = value.add(66) as *const u16;
    let max_dst = name_buf.len() - 1;
    let copy_len = name_len.min(max_dst);
    let mut written = 0;
    for i in 0..copy_len {
        let uc = ptr::read_unaligned(src.add(i));
        if uc < 0x80 {
            name_buf[written] = uc as u8;
            written += 1;
        } else if uc < 0x800 {
            if written + 2 > max_dst { break; }
            name_buf[written] = 0xC0 | ((uc >> 6) as u8);
            name_buf[written + 1] = 0x80 | ((uc & 0x3F) as u8);
            written += 2;
        } else {
            if written + 3 > max_dst { break; }
            name_buf[written] = 0xE0 | ((uc >> 12) as u8);
            name_buf[written + 1] = 0x80 | (((uc >> 6) & 0x3F) as u8);
            name_buf[written + 2] = 0x80 | ((uc & 0x3F) as u8);
            written += 3;
        }
    }
    name_buf[written] = 0;
    written as i32
}

unsafe fn get_mft_ref_name(record: *const u8, record_size: u32, name_buf: &mut [u8]) -> i32 {
    let attr = attr_find(record, record_size, ATTR_FILE_NAME, 0);
    if attr.is_null() {
        return -1;
    }
    read_file_name(attr, name_buf)
}

unsafe fn is_directory(record: *const u8) -> bool {
    let flags = read_packed!((*(record as *const MftRecordHeader)).flags);
    (flags & MFT_RECORD_DIR) != 0
}

unsafe fn get_mft_ref_data_size(record: *const u8, record_size: u32) -> i32 {
    let attr = attr_find(record, record_size, ATTR_DATA, 0);
    if attr.is_null() {
        return 0;
    }
    if attr_is_non_resident(attr) {
        let (_fv, _lv, data_size, _is, _ro) = non_resident_info(attr);
        data_size as i32
    } else {
        let (_value, value_len) = resident_value(attr);
        value_len as i32
    }
}

unsafe fn index_entry_name(entry: *const u8, name_buf: &mut [u8]) -> i32 {
    let hdr = entry as *const IndexEntryHeader;
    let _entry_len = read_packed!((*hdr).entry_len) as usize;
    let stream_len = read_packed!((*hdr).stream_len) as usize;
    if stream_len < 66 {
        return -1;
    }
    let fn_attr = entry.add(16) as *const FileNameAttr;
    let name_len = read_packed!((*fn_attr).name_len) as usize;
    let _namespace = read_packed!((*fn_attr).namespace);
    if name_len == 0 {
        return 0;
    }
    let src = entry.add(16 + 66) as *const u16;
    let max_dst = name_buf.len() - 1;
    let copy_len = name_len.min(max_dst);
    let mut written = 0;
    for i in 0..copy_len {
        let uc = ptr::read_unaligned(src.add(i));
        if uc < 0x80 {
            name_buf[written] = uc as u8;
            written += 1;
        } else if uc < 0x800 {
            if written + 2 > max_dst { break; }
            name_buf[written] = 0xC0 | ((uc >> 6) as u8);
            name_buf[written + 1] = 0x80 | ((uc & 0x3F) as u8);
            written += 2;
        } else {
            if written + 3 > max_dst { break; }
            name_buf[written] = 0xE0 | ((uc >> 12) as u8);
            name_buf[written + 1] = 0x80 | (((uc >> 6) & 0x3F) as u8);
            name_buf[written + 2] = 0x80 | ((uc & 0x3F) as u8);
            written += 3;
        }
    }
    name_buf[written] = 0;
    written as i32
}

unsafe fn index_entry_mft_ref(entry: *const u8) -> u64 {
    read_packed!((*(entry as *const IndexEntryHeader)).mft_ref)
}

unsafe fn index_entry_flags(entry: *const u8) -> u16 {
    read_packed!((*(entry as *const IndexEntryHeader)).flags)
}

unsafe fn index_entry_subnode_vcn(entry: *const u8) -> u64 {
    let entry_len = read_packed!((*(entry as *const IndexEntryHeader)).entry_len) as usize;
    let vcn_ptr = entry.add(entry_len - 8) as *const u64;
    if (entry_len as usize) >= 16 + 8 {
        ptr::read_unaligned(vcn_ptr)
    } else {
        0
    }
}

unsafe fn read_index_block(vcn: u64, buffer: *mut u8) -> i32 {
    let ntfs = &(*ntfs());
    let ibs = ntfs.index_block_size;
    let bpc = ntfs.bytes_per_cluster;
    let spc = ntfs.sectors_per_cluster as u64;
    if ibs <= bpc {
        read_clusters(vcn, buffer)
    } else {
        let clusters = ibs / bpc;
        let start_lba = vcn * spc;
        read_sectors(start_lba, clusters * spc as u32, buffer)
    }
}

unsafe fn iter_index_entries(
    record: *const u8,
    record_size: u32,
    entries: &mut [LumieDirEnt],
    max_entries: i32,
) -> i32 {
    let mut count = 0i32;
    let root_attr = attr_find(record, record_size, ATTR_INDEX_ROOT, 0);
    if root_attr.is_null() {
        return -1;
    }
    let (root_value, _root_len) = resident_value(root_attr);
    let index_root = root_value as *const IndexRoot;
    let entries_off = read_packed!((*index_root).entries_off) as usize;
    let entries_size = read_packed!((*index_root).entries_size) as usize;
    let mut pos = root_value.add(entries_off);
    let end = root_value.add(16 + entries_size);
    while (pos as usize) < (end as usize) {
        let hdr = pos as *const IndexEntryHeader;
        let entry_len = read_packed!((*hdr).entry_len) as usize;
        let flags = read_packed!((*hdr).flags);
        if entry_len == 0 {
            break;
        }
        if (flags & INDEX_ENTRY_LAST) != 0 {
            if (flags & INDEX_ENTRY_SUBNODE) != 0 && count < max_entries {
                let vcn = index_entry_subnode_vcn(pos);
                count += read_index_alloc_entries(vcn, entries, count, max_entries);
            }
            break;
        }
        if count < max_entries {
            let mut name_buf = [0u8; 256];
            let name_len = index_entry_name(pos, &mut name_buf);
            if name_len > 0 {
                entries[count as usize].name = [0u8; 256];
                let nl = (name_len as usize).min(255);
                entries[count as usize].name[..nl].copy_from_slice(&name_buf[..nl]);
                let mft_ref = index_entry_mft_ref(pos);
                let mut child_rec = [0u8; 4096];
                let crs = record_size.min(4096);
                if mft_read_record(mft_ref & 0x0000_FFFF_FFFF_FFFF, child_rec.as_mut_ptr()) == 0 {
                    if is_directory(child_rec.as_ptr()) {
                        entries[count as usize].is_dir = 1;
                    } else {
                        entries[count as usize].size = get_mft_ref_data_size(child_rec.as_ptr(), crs) as u32;
                    }
                }
            }
            count += 1;
        }
        if (flags & INDEX_ENTRY_SUBNODE) != 0 {
            let vcn = index_entry_subnode_vcn(pos);
            count += read_index_alloc_entries(vcn, entries, count, max_entries);
        }
        pos = pos.add(entry_len);
    }
    count
}

unsafe fn read_index_alloc_entries(
    vcn: u64,
    entries: &mut [LumieDirEnt],
    start: i32,
    max_entries: i32,
) -> i32 {
    let ntfs = &(*ntfs());
    let ibs = ntfs.index_block_size as usize;
    let mut block = [0u8; 16384];
    if ibs > block.len() {
        return 0;
    }
    if read_index_block(vcn, block.as_mut_ptr()) != 0 {
        return 0;
    }
    let sig = ptr::read_unaligned(block.as_ptr() as *const [u8; 4]);
    if &sig != b"INDX" {
        return 0;
    }
    let fixup_off = ptr::read_unaligned(block.as_ptr().add(4) as *const u16) as usize;
    let fixup_count = ptr::read_unaligned(block.as_ptr().add(6) as *const u16) as usize;
    if fixup_off > 0 && fixup_count >= 2 {
        let sig_val = ptr::read_unaligned(block.as_ptr().add(fixup_off) as *const u16);
        let sector_size = ntfs.bytes_per_sector as usize;
        for i in 0..(fixup_count - 1) {
            let sector_end = ((i + 1) * sector_size) - 2;
            if sector_end + 2 > ibs {
                break;
            }
            let fixup_entry = ptr::read_unaligned(block.as_ptr().add(fixup_off + 2 + i * 2) as *const u16);
            let target = block.as_mut_ptr().add(sector_end) as *mut u16;
            ptr::write_unaligned(target, fixup_entry);
        }
        let end_sig = block.as_mut_ptr().add(ibs - 2) as *mut u16;
        ptr::write_unaligned(end_sig, sig_val);
    }
    let entries_off = ptr::read_unaligned(block.as_ptr().add(24) as *const u32) as usize;
    let entries_size = ptr::read_unaligned(block.as_ptr().add(28) as *const u32) as usize;
    let mut pos = block.as_ptr().add(entries_off);
    let end = block.as_ptr().add(entries_off + entries_size);
    let mut count = start;
    while (pos as usize) < (end as usize) && count < max_entries {
        let hdr = pos as *const IndexEntryHeader;
        let entry_len = read_packed!((*hdr).entry_len) as usize;
        let flags = read_packed!((*hdr).flags);
        if entry_len == 0 {
            break;
        }
        if (flags & INDEX_ENTRY_LAST) != 0 {
            if (flags & INDEX_ENTRY_SUBNODE) != 0 {
                let cvcn = index_entry_subnode_vcn(pos);
                count += read_index_alloc_entries(cvcn, entries, count, max_entries);
            }
            break;
        }
        let mut name_buf = [0u8; 256];
        let name_len = index_entry_name(pos, &mut name_buf);
        if name_len > 0 {
            let idx = count as usize;
            if idx < entries.len() {
                entries[idx].name = [0u8; 256];
                let nl = (name_len as usize).min(255);
                entries[idx].name[..nl].copy_from_slice(&name_buf[..nl]);
                let mft_ref = index_entry_mft_ref(pos);
                let mut child = [0u8; 4096];
                let crs = record_size().min(4096);
                if mft_read_record(mft_ref & 0x0000_FFFF_FFFF_FFFF, child.as_mut_ptr()) == 0 {
                    if is_directory(child.as_ptr()) {
                        entries[idx].is_dir = 1;
                    } else {
                        entries[idx].size = get_mft_ref_data_size(child.as_ptr(), crs) as u32;
                    }
                }
            }
            count += 1;
        }
        if (flags & INDEX_ENTRY_SUBNODE) != 0 {
            let cvcn = index_entry_subnode_vcn(pos);
            count += read_index_alloc_entries(cvcn, entries, count, max_entries);
        }
        pos = pos.add(entry_len);
    }
    count - start
}

unsafe fn record_size() -> u32 {
    (*ntfs()).mft_record_size
}

unsafe fn find_in_dir(dir_mft_ref: u64, target_name: &[u8]) -> u64 {
    let rsize = record_size();
    let mut rec = [0u8; 4096];
    let rs = rsize.min(4096);
    if mft_read_record(dir_mft_ref, rec.as_mut_ptr()) != 0 {
        return u64::MAX;
    }
    let root_attr = attr_find(rec.as_ptr(), rs, ATTR_INDEX_ROOT, 0);
    if root_attr.is_null() {
        return u64::MAX;
    }
    let (root_value, _root_len) = resident_value(root_attr);
    let index_root = root_value as *const IndexRoot;
    let entries_off = read_packed!((*index_root).entries_off) as usize;
    let entries_size = read_packed!((*index_root).entries_size) as usize;
    let mut pos = root_value.add(entries_off);
    let end = root_value.add(16 + entries_size);
    loop {
        if (pos as usize) >= (end as usize) {
            break;
        }
        let hdr = pos as *const IndexEntryHeader;
        let entry_len = read_packed!((*hdr).entry_len) as usize;
        let flags = read_packed!((*hdr).flags);
        if entry_len == 0 {
            break;
        }
        if (flags & INDEX_ENTRY_LAST) != 0 {
            if (flags & INDEX_ENTRY_SUBNODE) != 0 {
                let vcn = index_entry_subnode_vcn(pos);
                let result = find_in_index_alloc(vcn, target_name);
                if result != u64::MAX {
                    return result;
                }
            }
            break;
        }
        let mut entry_name = [0u8; 256];
        let name_len = index_entry_name(pos, &mut entry_name);
        if name_len > 0 {
            let en = core::str::from_utf8(&entry_name[..name_len as usize]);
            let tn = core::str::from_utf8(target_name);
            if let (Ok(e), Ok(t)) = (en, tn) {
                if e.eq_ignore_ascii_case(t) {
                    return index_entry_mft_ref(pos);
                }
            }
        }
        if (flags & INDEX_ENTRY_SUBNODE) != 0 {
            let vcn = index_entry_subnode_vcn(pos);
            let result = find_in_index_alloc(vcn, target_name);
            if result != u64::MAX {
                return result;
            }
        }
        pos = pos.add(entry_len);
    }
    u64::MAX
}

unsafe fn find_in_index_alloc(vcn: u64, target_name: &[u8]) -> u64 {
    let ibs = (*ntfs()).index_block_size as usize;
    let mut block = [0u8; 16384];
    if ibs > block.len() {
        return u64::MAX;
    }
    if read_index_block(vcn, block.as_mut_ptr()) != 0 {
        return u64::MAX;
    }
    let sig = ptr::read_unaligned(block.as_ptr() as *const [u8; 4]);
    if &sig != b"INDX" {
        return u64::MAX;
    }
    let fixup_off = ptr::read_unaligned(block.as_ptr().add(4) as *const u16) as usize;
    let fixup_count = ptr::read_unaligned(block.as_ptr().add(6) as *const u16) as usize;
    if fixup_off > 0 && fixup_count >= 2 {
        let sector_size = (*ntfs()).bytes_per_sector as usize;
        for i in 0..(fixup_count - 1) {
            let sector_end = ((i + 1) * sector_size) - 2;
            if sector_end + 2 > ibs {
                break;
            }
            let fixup_entry = ptr::read_unaligned(block.as_ptr().add(fixup_off + 2 + i * 2) as *const u16);
            let target = block.as_mut_ptr().add(sector_end) as *mut u16;
            ptr::write_unaligned(target, fixup_entry);
        }
    }
    let entries_off = ptr::read_unaligned(block.as_ptr().add(24) as *const u32) as usize;
    let entries_size = ptr::read_unaligned(block.as_ptr().add(28) as *const u32) as usize;
    let mut pos = block.as_ptr().add(entries_off);
    let end = block.as_ptr().add(entries_off + entries_size);
    loop {
        if (pos as usize) >= (end as usize) {
            break;
        }
        let hdr = pos as *const IndexEntryHeader;
        let entry_len = read_packed!((*hdr).entry_len) as usize;
        let flags = read_packed!((*hdr).flags);
        if entry_len == 0 {
            break;
        }
        if (flags & INDEX_ENTRY_LAST) != 0 {
            if (flags & INDEX_ENTRY_SUBNODE) != 0 {
                let vcn2 = index_entry_subnode_vcn(pos);
                let result = find_in_index_alloc(vcn2, target_name);
                if result != u64::MAX {
                    return result;
                }
            }
            break;
        }
        let mut entry_name = [0u8; 256];
        let name_len = index_entry_name(pos, &mut entry_name);
        if name_len > 0 {
            let en = core::str::from_utf8(&entry_name[..name_len as usize]);
            let tn = core::str::from_utf8(target_name);
            if let (Ok(e), Ok(t)) = (en, tn) {
                if e.eq_ignore_ascii_case(t) {
                    return index_entry_mft_ref(pos);
                }
            }
        }
        if (flags & INDEX_ENTRY_SUBNODE) != 0 {
            let vcn2 = index_entry_subnode_vcn(pos);
            let result = find_in_index_alloc(vcn2, target_name);
            if result != u64::MAX {
                return result;
            }
        }
        pos = pos.add(entry_len);
    }
    u64::MAX
}

fn is_absolute_path(path: &str) -> bool {
    path.as_bytes().first() == Some(&b'/') || path.as_bytes().first() == Some(&b'\\')
}

unsafe fn resolve_path(path: &str) -> u64 {
    if !is_absolute_path(path) {
        return u64::MAX;
    }
    let trimmed = path.trim_start_matches('/').trim_start_matches('\\');
    if trimmed.is_empty() {
        return MFT_ENTRY_ROOT;
    }
    let mut current = MFT_ENTRY_ROOT;
    for component in trimmed.split(|c| c == '/' || c == '\\') {
        if component.is_empty() || component == "." {
            continue;
        }
        if component == ".." {
            let mut rec = [0u8; 4096];
            let rsize = record_size().min(4096);
            if mft_read_record(current, rec.as_mut_ptr()) != 0 {
                return u64::MAX;
            }
            let fn_attr = attr_find(rec.as_ptr(), rsize, ATTR_FILE_NAME, 0);
            if fn_attr.is_null() {
                return u64::MAX;
            }
            let (val, _vl) = resident_value(fn_attr);
            let parent_ref = ptr::read_unaligned(val as *const u64);
            current = parent_ref & 0x0000_FFFF_FFFF_FFFF;
            continue;
        }
        current = find_in_dir(current, component.as_bytes());
        if current == u64::MAX {
            return u64::MAX;
        }
    }
    current
}

pub unsafe fn read_file(path: &str, buffer: *mut u8, max_size: u32) -> i32 {
    let mft_ref = resolve_path(path);
    if mft_ref == u64::MAX {
        return -1;
    }
    let rsize = record_size();
    let mut rec = [0u8; 4096];
    let rs = rsize.min(4096);
    if mft_read_record(mft_ref, rec.as_mut_ptr()) != 0 {
        return -1;
    }
    let attr = attr_find(rec.as_ptr(), rs, ATTR_DATA, 0);
    if attr.is_null() {
        return -1;
    }
    if attr_is_non_resident(attr) {
        read_non_resident_data(attr, buffer, 0, max_size)
    } else {
        let (value, value_len) = resident_value(attr);
        let copy_len = value_len.min(max_size);
        ptr::copy_nonoverlapping(value, buffer, copy_len as usize);
        copy_len as i32
    }
}

pub unsafe fn list_dir(path: &str, entries: &mut [LumieDirEnt], max_entries: i32) -> i32 {
    let mft_ref = resolve_path(path);
    if mft_ref == u64::MAX {
        return -1;
    }
    let rsize = record_size();
    let mut rec = [0u8; 4096];
    let rs = rsize.min(4096);
    if mft_read_record(mft_ref, rec.as_mut_ptr()) != 0 {
        return -1;
    }
    if !is_directory(rec.as_ptr()) {
        return -2;
    }
    iter_index_entries(rec.as_ptr(), rs, entries, max_entries)
}

pub unsafe fn exists(path: &str) -> bool {
    resolve_path(path) != u64::MAX
}

pub unsafe fn get_file_size(path: &str) -> i32 {
    let mft_ref = resolve_path(path);
    if mft_ref == u64::MAX {
        return -1;
    }
    let rsize = record_size();
    let mut rec = [0u8; 4096];
    let rs = rsize.min(4096);
    if mft_read_record(mft_ref, rec.as_mut_ptr()) != 0 {
        return -1;
    }
    get_mft_ref_data_size(rec.as_ptr(), rs)
}

unsafe fn allocate_mft_record() -> u64 {
    let ntfs = &(*ntfs());
    let mft_bmp_ref = MFT_ENTRY_MFT + 6;
    let rsize = record_size();
    let mut rec = [0u8; 4096];
    let rs = rsize.min(4096);
    if mft_read_record(mft_bmp_ref, rec.as_mut_ptr()) != 0 {
        return u64::MAX;
    }
    let attr = attr_find(rec.as_ptr(), rs, ATTR_BITMAP, 0);
    if attr.is_null() {
        return u64::MAX;
    }
    if !attr_is_non_resident(attr) {
        return u64::MAX;
    }
    let (lcns, run_count) = get_data_runs(attr);
    if run_count == 0 {
        return u64::MAX;
    }
    let bmp_lcn = lcns[0];
    let clusters_per_mft = ntfs.sectors_per_cluster as u64;
    let total_mft_records = (ntfs.mft_lcn * clusters_per_mft) * 8;
    let mut sector_buf = [0u8; 4096];
    for i in 0..total_mft_records {
        let byte_idx = (i / 8) as u64;
        let bit_idx = i % 8;
        if byte_idx >= clusters_per_mft * 512 {
            break;
        }
        let sector = bmp_lcn * clusters_per_mft + byte_idx / (ntfs.bytes_per_sector as u64);
        let sector_off = (byte_idx % ntfs.bytes_per_sector as u64) as usize;
        if read_sectors(sector, 1, sector_buf.as_mut_ptr()) != 0 {
            continue;
        }
        let byte = sector_buf[sector_off];
        if (byte & (1 << bit_idx)) == 0 {
            sector_buf[sector_off] = byte | (1 << bit_idx);
            if write_sectors(sector, 1, sector_buf.as_ptr()) != 0 {
                continue;
            }
            return i;
        }
    }
    u64::MAX
}

unsafe fn allocate_clusters(count: u32) -> Option<u64> {
    let ntfs = &(*ntfs());
    let bmp_mft_ref = MFT_ENTRY_MFT + 6;
    let rsize = record_size();
    let mut rec = [0u8; 4096];
    let rs = rsize.min(4096);
    if mft_read_record(bmp_mft_ref, rec.as_mut_ptr()) != 0 {
        return None;
    }
    let attr = attr_find(rec.as_ptr(), rs, ATTR_BITMAP, 0);
    if attr.is_null() {
        return None;
    }
    if !attr_is_non_resident(attr) {
        return None;
    }
    let (lcns, run_count) = get_data_runs(attr);
    if run_count == 0 {
        return None;
    }
    let bmp_lcn = lcns[0];
    let total_clusters = ntfs.total_sectors / ntfs.sectors_per_cluster as u64;
    let mut sector_buf = [0u8; 4096];
    let mut consecutive = 0;
    let mut start_cluster = 0;
    for i in 0..total_clusters {
        let byte_idx = (i / 8) as u64;
        let bit_idx = i % 8;
        let sector = bmp_lcn * ntfs.sectors_per_cluster as u64 + byte_idx / (ntfs.bytes_per_sector as u64);
        let sector_off = (byte_idx % ntfs.bytes_per_sector as u64) as usize;
        if read_sectors(sector, 1, sector_buf.as_mut_ptr()) != 0 {
            consecutive = 0;
            continue;
        }
        let byte = sector_buf[sector_off];
        if (byte & (1 << bit_idx)) == 0 {
            if consecutive == 0 {
                start_cluster = i;
            }
            consecutive += 1;
            if consecutive == count {
                for j in 0..count {
                    let ci = start_cluster + (j as u64);
                    let b_idx = (ci / 8) as u64;
                    let b_bit = ci % 8;
                    let sec = bmp_lcn * ntfs.sectors_per_cluster as u64 + b_idx / (ntfs.bytes_per_sector as u64);
                    let sec_off = (b_idx % ntfs.bytes_per_sector as u64) as usize;
                    if read_sectors(sec, 1, sector_buf.as_mut_ptr()) != 0 {
                        return None;
                    }
                    sector_buf[sec_off] |= 1 << b_bit;
                    if write_sectors(sec, 1, sector_buf.as_ptr()) != 0 {
                        return None;
                    }
                }
                return Some(start_cluster);
            }
        } else {
            consecutive = 0;
        }
    }
    None
}

unsafe fn write_sectors(lba: u64, count: u32, buffer: *const u8) -> i32 {
    let ntfs = &(*ntfs());
    if let Some(cb) = ntfs.disk_io.write_cb {
        return cb(lba as u32, count, buffer as *mut u8);
    }
    if ntfs.disk_io.use_ahci {
        return ahci_drv::write_sectors(lba as u32, count, buffer as *mut u8);
    }
    if let Some(block_io) = ntfs.disk_io.block_io {
        let sector_size = 512u64;
        let media = (*block_io).media;
        let status = ((*block_io).write_blocks.unwrap())(
            block_io as *mut c_void,
            (*media).media_id,
            lba,
            (count as u64) * sector_size,
            buffer as *mut c_void,
        );
        if efi_error(status) {
            return -1;
        }
        return 0;
    }
    -1
}

unsafe fn write_mft_record(record_num: u64, buffer: *const u8) -> i32 {
    let ntfs = &(*ntfs());
    let clusters_per_record = ntfs.clusters_per_mft_record;
    let record_size = record_size();
    let clusters_needed = if clusters_per_record > 0 {
        clusters_per_record as u64
    } else {
        1u64 << (-(clusters_per_record as i32)) as u64
    };
    let start_vcn = record_num * clusters_needed;
    let mut cluster_buf = [0u8; 4096];
    ptr::copy_nonoverlapping(buffer, cluster_buf.as_mut_ptr(), record_size as usize);
    for i in 0..clusters_needed {
        let cluster_lcn = start_vcn + i;
        let lba = cluster_lcn * ntfs.sectors_per_cluster as u64;
        if write_sectors(lba, ntfs.sectors_per_cluster as u32, cluster_buf.as_ptr()) != 0 {
            return -1;
        }
    }
    0
}

unsafe fn create_file_record(name: &str, parent_ref: u64, is_dir: bool, data: *const u8, data_len: u32) -> u64 {
    let mft_ref = allocate_mft_record();
    if mft_ref == u64::MAX {
        return u64::MAX;
    }
    let rsize = record_size();
    let mut rec = [0u8; 4096];
    let record_ptr = rec.as_mut_ptr();
    let header = record_ptr as *mut MftRecordHeader;
    ptr::write_bytes(header, 0, 1);
    ptr::write(header as *mut [u8; 4], *b"FILE");
    ptr::write_unaligned(core::ptr::addr_of_mut!((*header).fixup_off), 0x30u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*header).fixup_count), 3u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*header).attr_off), 0x30u16);
    let flags = if is_dir { MFT_RECORD_IN_USE | MFT_RECORD_DIR } else { MFT_RECORD_IN_USE };
    ptr::write_unaligned(core::ptr::addr_of_mut!((*header).flags), flags);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*header).used_size), 0x50u32);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*header).alloc_size), rsize);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*header).next_attr_id), 4u16);
    
    let mut attr_ptr = record_ptr.add(0x30);
    
let mut std_info_buf = [0u8; 72];
    let si = std_info_buf.as_mut_ptr() as *mut u8;
    ptr::write_unaligned(si.add(0) as *mut u64, 0);
    ptr::write_unaligned(si.add(8) as *mut u64, 0);
    ptr::write_unaligned(si.add(16) as *mut u64, 0);
    ptr::write_unaligned(si.add(24) as *mut u64, 0);
    let file_attr = if is_dir { FILE_ATTR_DIRECTORY | FILE_ATTR_ARCHIVE } else { FILE_ATTR_ARCHIVE };
    ptr::write_unaligned(si.add(32) as *mut u32, file_attr);
    ptr::write_unaligned(si.add(36) as *mut u32, 0);
    ptr::write_unaligned(si.add(40) as *mut u64, 0);
    ptr::write_unaligned(si.add(48) as *mut u64, 0);
    ptr::write_unaligned(si.add(56) as *mut u32, 0);
    ptr::write_unaligned(si.add(60) as *mut u32, 0);
    let si_hdr = si as *mut AttrHeader;
    ptr::write_unaligned(core::ptr::addr_of_mut!((*si_hdr).type_code), ATTR_STANDARD_INFORMATION);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*si_hdr).length), 72u32);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*si_hdr).non_resident), 0u8);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*si_hdr).name_len), 0u8);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*si_hdr).name_off), 0u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*si_hdr).flags), 0u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*si_hdr).attr_id), 0u16);
    ptr::copy_nonoverlapping(si, attr_ptr, 72);
    attr_ptr = attr_ptr.add(72);
    
    let mut name_utf16: [u16; 256] = [0; 256];
    let mut name_len = 0;
    for (i, c) in name.encode_utf16().enumerate() {
        if i >= 256 { break; }
        name_utf16[i] = c;
        name_len = i + 1;
    }
    let fn_size = 66 + name_len * 2;
    let _fn_attr_off = attr_ptr as usize - record_ptr as usize;
    let mut fn_buf = [0u8; 512];
    let fn_ptr = fn_buf.as_mut_ptr();
    let fn_hdr = fn_ptr as *mut AttrHeader;
    ptr::write_unaligned(core::ptr::addr_of_mut!((*fn_hdr).type_code), ATTR_FILE_NAME);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*fn_hdr).length), fn_size as u32);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*fn_hdr).non_resident), 0u8);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*fn_hdr).name_len), 0u8);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*fn_hdr).name_off), 0u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*fn_hdr).flags), 0u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*fn_hdr).attr_id), 1u16);
    ptr::write_unaligned(fn_ptr.add(16) as *mut u64, parent_ref);
    ptr::write_unaligned(fn_ptr.add(24) as *mut u64, 0);
    ptr::write_unaligned(fn_ptr.add(32) as *mut u64, 0);
    ptr::write_unaligned(fn_ptr.add(40) as *mut u64, 0);
    ptr::write_unaligned(fn_ptr.add(48) as *mut u64, 0);
    ptr::write_unaligned(fn_ptr.add(56) as *mut u64, 0);
    ptr::write_unaligned(fn_ptr.add(64) as *mut u64, 0);
    ptr::write_unaligned(fn_ptr.add(72) as *mut u32, if is_dir { FILE_ATTR_DIRECTORY | FILE_ATTR_ARCHIVE } else { FILE_ATTR_ARCHIVE });
    ptr::write_unaligned(fn_ptr.add(76) as *mut u32, 0);
    ptr::write_unaligned(fn_ptr.add(80) as *mut u8, name_len as u8);
    ptr::write_unaligned(fn_ptr.add(81) as *mut u8, NAMESPACE_WIN32);
    for i in 0..name_len {
        ptr::write_unaligned(fn_ptr.add(82 + i * 2) as *mut u16, name_utf16[i]);
    }
    ptr::copy_nonoverlapping(fn_ptr, attr_ptr, fn_size);
    attr_ptr = attr_ptr.add(fn_size);
    
    if !is_dir && data_len > 0 {
        let clusters_needed = ((data_len as u64 + (*ntfs()).bytes_per_cluster as u64 - 1) / (*ntfs()).bytes_per_cluster as u64) as u32;
        if let Some(start_lcn) = allocate_clusters(clusters_needed) {
            let ntfs = &(*ntfs());
            let mut run_buf = [0u8; 16];
            run_buf[0] = clusters_needed as u8;
            run_buf[1] = start_lcn as u8;
            run_buf[2] = (start_lcn >> 8) as u8;
            run_buf[3] = (start_lcn >> 16) as u8;
let data_attr_size = 56 + 4;
            let mut data_buf = [0u8; 1024]; // Max size for simplicity
            let data_ptr = data_buf.as_mut_ptr();
            let data_hdr = data_ptr as *mut AttrHeader;
            ptr::write_unaligned(core::ptr::addr_of_mut!((*data_hdr).type_code), ATTR_DATA);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*data_hdr).length), data_attr_size as u32);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*data_hdr).non_resident), 1u8);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*data_hdr).name_len), 0u8);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*data_hdr).name_off), 0u16);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*data_hdr).flags), 0u16);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*data_hdr).attr_id), 2u16);
            let nr = data_ptr.add(16) as *mut NonResidentAttr;
            ptr::write_unaligned(core::ptr::addr_of_mut!((*nr).first_vcn), 0u64);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*nr).last_vcn), (clusters_needed - 1) as u64);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*nr).run_off), 56u16);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*nr).compr_size), 0u16);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*nr).alloc_size), data_len as u64);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*nr).data_size), data_len as u64);
            ptr::write_unaligned(core::ptr::addr_of_mut!((*nr).init_size), data_len as u64);
            ptr::copy_nonoverlapping(run_buf.as_ptr(), data_ptr.add(56), 4);
            ptr::copy_nonoverlapping(data_ptr, attr_ptr, data_attr_size);
            attr_ptr = attr_ptr.add(data_attr_size);
            let bytes_per_cluster = ntfs.bytes_per_cluster;
            let mut written = 0u32;
            let mut src = data;
            let mut cluster_idx = 0u64;
            while written < data_len {
                let chunk = (bytes_per_cluster as u32).min(data_len - written);
                let mut cluster_buf = [0u8; 4096];
                ptr::copy_nonoverlapping(src, cluster_buf.as_mut_ptr(), chunk as usize);
                let vcn = start_lcn + cluster_idx;
                let lba = vcn * ntfs.sectors_per_cluster as u64;
                if write_sectors(lba, ntfs.sectors_per_cluster as u32, cluster_buf.as_ptr()) != 0 {
                    break;
                }
                src = src.add(chunk as usize);
                written += chunk;
                cluster_idx += 1;
            }
        }
    }
    
    ptr::write_unaligned(attr_ptr as *mut u32, 0xFFFFFFFF);
    let used_size = attr_ptr as usize - record_ptr as usize + 4;
    ptr::write_unaligned(core::ptr::addr_of_mut!((*header).used_size), used_size as u32);
    apply_fixup(record_ptr, rsize);
    if write_mft_record(mft_ref, record_ptr) != 0 {
        return u64::MAX;
    }
    mft_ref
}

unsafe fn add_dir_entry(dir_mft_ref: u64, name: &str, file_mft_ref: u64) -> i32 {
    let rsize = record_size();
    let mut rec = [0u8; 4096];
    if mft_read_record(dir_mft_ref, rec.as_mut_ptr()) != 0 {
        return -1;
    }
    let attr = attr_find(rec.as_ptr(), rsize, ATTR_INDEX_ROOT, 0);
    if attr.is_null() {
        return -1;
    }
    if attr_is_non_resident(attr) {
        return -1;
    }
    let (val, _vl) = resident_value(attr);
    let index_root = val as *mut IndexRoot;
    let entries_off = read_packed!((*index_root).entries_off) as usize;
    let entries_size = read_packed!((*index_root).entries_size) as usize;
    let entries_ptr = val.add(entries_off);
    let mut entry_buf = [0u8; 256];
    let mut name_utf16: [u16; 256] = [0; 256];
    let mut name_len = 0;
    for (i, c) in name.encode_utf16().enumerate() {
        if i >= 256 { break; }
        name_utf16[i] = c;
        name_len = i + 1;
    }
    let entry_len = 16 + name_len * 2;
    let entry_len = (entry_len + 7) & !7;
    let ieh = entry_buf.as_mut_ptr() as *mut IndexEntryHeader;
    ptr::write_unaligned(core::ptr::addr_of_mut!((*ieh).mft_ref), file_mft_ref);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*ieh).entry_len), entry_len as u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*ieh).stream_len), (name_len * 2) as u16);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*ieh).flags), 0u16);
    for i in 0..name_len {
        ptr::write_unaligned(entry_buf.as_mut_ptr().add(16 + i * 2) as *mut u16, name_utf16[i]);
    }
    let last_entry = entries_ptr.add(entries_size);
    let new_entry_ptr = entries_ptr.add(entries_size - 16);
    ptr::copy_nonoverlapping(entry_buf.as_ptr(), new_entry_ptr as *mut u8, entry_len);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*(new_entry_ptr as *mut IndexEntryHeader)).flags), INDEX_ENTRY_LAST);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*(last_entry as *mut IndexEntryHeader)).flags), INDEX_ENTRY_LAST);
    ptr::write_unaligned(core::ptr::addr_of_mut!((*index_root).entries_size), (entries_size + entry_len) as u32);
    apply_fixup(rec.as_mut_ptr(), rsize);
    write_mft_record(dir_mft_ref, rec.as_ptr())
}

pub unsafe fn write_file(path: &str, data: *const u8, size: u32) -> i32 {
    if size == 0 {
        return 0;
    }
    let trimmed = path.trim_start_matches('/').trim_start_matches('\\');
    if trimmed.is_empty() {
        return -1;
    }
    let mut components: [&str; 32] = [&"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &""];
    let mut comp_count = 0;
    for part in trimmed.split(|c| c == '/' || c == '\\') {
        if comp_count < 32 && !part.is_empty() {
            components[comp_count] = part;
            comp_count += 1;
        }
    }
    let filename = if comp_count > 0 {
        components[comp_count - 1]
    } else {
        return -1;
    };
    let parent_ref = if comp_count <= 1 {
        resolve_path("/")
    } else {
        let mut p = [0u8; 256];
        p[0] = b'/';
        let mut pos = 1;
        for i in 0..comp_count - 1 {
            let part = components[i];
            let len = part.len().min(255 - pos);
            if pos + len >= 255 { break; }
            p[pos..pos+len].copy_from_slice(&part.as_bytes()[..len]);
            pos += len;
            if pos < 255 {
                p[pos] = b'/';
                pos += 1;
            }
        }
        let s = core::str::from_utf8(&p[..pos]).unwrap_or("/");
        resolve_path(s)
    };
    if parent_ref == u64::MAX {
        return -1;
    }
    let file_ref = create_file_record(filename, parent_ref, false, data, size);
    if file_ref == u64::MAX {
        return -1;
    }
    if add_dir_entry(parent_ref, filename, file_ref) != 0 {
        return -1;
    }
    size as i32
}

pub unsafe fn mkdir(path: &str) -> i32 {
    let trimmed = path.trim_start_matches('/').trim_start_matches('\\');
    if trimmed.is_empty() {
        return -1;
    }
    let mut components: [&str; 32] = [&"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &"", &""];
    let mut comp_count = 0;
    for part in trimmed.split(|c| c == '/' || c == '\\') {
        if comp_count < 32 && !part.is_empty() {
            components[comp_count] = part;
            comp_count += 1;
        }
    }
    let dirname = if comp_count > 0 {
        components[comp_count - 1]
    } else {
        return -1;
    };
    let parent_ref = if comp_count <= 1 {
        resolve_path("/")
    } else {
        let mut p = [0u8; 256];
        p[0] = b'/';
        let mut pos = 1;
        for i in 0..comp_count - 1 {
            let part = components[i];
            let len = part.len().min(255 - pos);
            if pos + len >= 255 { break; }
            p[pos..pos+len].copy_from_slice(&part.as_bytes()[..len]);
            pos += len;
            if pos < 255 {
                p[pos] = b'/';
                pos += 1;
            }
        }
        let s = core::str::from_utf8(&p[..pos]).unwrap_or("/");
        resolve_path(s)
    };
    if parent_ref == u64::MAX {
        return -1;
    }
    let dir_ref = create_file_record(dirname, parent_ref, true, ptr::null(), 0);
    if dir_ref == u64::MAX {
        return -1;
    }
    if add_dir_entry(parent_ref, dirname, dir_ref) != 0 {
        return -1;
    }
    0
}

pub unsafe fn delete(_path: &str) -> i32 {
    -1
}

pub unsafe fn format(_total_sectors: u64) -> i32 {
    -1
}

pub unsafe fn set_device(device_handle: efi_handle) -> i32 {
    let bs = G_BS;
    if bs.is_null() {
        return -1;
    }
    let block_io_guid = EFI_BLOCK_IO_GUID;
    let mut block_io: *mut EfiBlockIoProtocol = ptr::null_mut();
    let status = ((*bs).handle_protocol.unwrap())(
        device_handle,
        &block_io_guid as *const EfiGuid as *mut EfiGuid,
        &mut block_io as *mut *mut EfiBlockIoProtocol as *mut *mut c_void,
    );
    if efi_error(status) || block_io.is_null() {
        return -1;
    }
    (*ntfs()).disk_io.block_io = Some(block_io);
    init()
}

pub unsafe fn set_bs(bs: *mut EfiBootServices) {
    G_BS = bs;
}

pub unsafe fn reinit() -> i32 {
    init()
}

pub unsafe fn use_ahci() -> i32 {
    if ahci_drv::is_ready() == 0 {
        return -1;
    }
    (*ntfs()).disk_io.use_ahci = true;
    init()
}
