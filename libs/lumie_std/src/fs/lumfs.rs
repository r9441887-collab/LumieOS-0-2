use core::ptr;
use core::mem::MaybeUninit;
use super::diskio::{DiskIo, FatReadFn, FatWriteFn, AllocFn, FreeFn, TimeFn};
use super::types::LumieDirEnt;

const MAGIC: u32 = 0x4C554D32;
const VERSION: u16 = 2;
const CELL_SIZE: u32 = 4096;
const SECTORS_PER_CELL: u32 = CELL_SIZE / 512;
const HDR_SIZE: u32 = 48;
const DATA_SIZE: u32 = CELL_SIZE - HDR_SIZE;
const ROOT_DIR_ID: u32 = 1;
const CRC_POLY: u32 = 0xEDB88320;

const TYPE_FREE: u8 = 0;
const TYPE_DEAD: u8 = 1;
const TYPE_FILE_HDR: u8 = 2;
const TYPE_FILE_DATA: u8 = 3;
const TYPE_DIR: u8 = 4;

#[repr(C, packed)]
struct Superblock {
    magic: u32,
    version: u16,
    flags: u16,
    total_cells: u64,
    write_ptr: u64,
    gc_ptr: u64,
    root_dir_cell: u32,
    file_count: u32,
    free_count: u64,
    volume_label: [u8; 32],
    checksum: u32,
}

#[repr(C, packed)]
struct CellHdr {
    magic: u32,
    chain_id: u32,
    seq: u32,
    next_cell: u32,
    cell_type: u8,
    flags: u8,
    name_hash: u16,
    data_len: u16,
    reserved: u16,
    crc: u32,
    parent_chain: u32,
    total_size: u32,
    created: u32,
    modified: u32,
}

struct LumFs {
    initialized: bool,
    disk_io: DiskIo,
    total_cells: u64,
    write_ptr: u64,
    gc_ptr: u64,
    root_dir_cell: u32,
    file_count: u32,
    free_count: u64,
    next_chain_id: u32,
}

static mut LUM_DRIVER: MaybeUninit<LumFs> = MaybeUninit::uninit();

unsafe fn driver() -> &'static mut LumFs {
    LUM_DRIVER.assume_init_mut()
}

unsafe fn alloc_c(size: usize) -> *mut u8 {
    match driver().disk_io.alloc_cb {
        Some(f) => f(size),
        None => core::ptr::null_mut(),
    }
}

unsafe fn free_c(ptr: *mut u8) {
    if let Some(f) = driver().disk_io.free_cb {
        f(ptr, 0);
    }
}

unsafe fn disk_read(lba: u32, count: u32, buf: *mut u8) -> i32 {
    if let Some(f) = driver().disk_io.read_cb {
        f(lba, count, buf)
    } else {
        -1
    }
}

unsafe fn disk_write(lba: u32, count: u32, buf: *const u8) -> i32 {
    if let Some(f) = driver().disk_io.write_cb {
        f(lba, count, buf)
    } else {
        -1
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut c: u32 = 0xFFFFFFFF;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ CRC_POLY } else { c >> 1 };
        }
    }
    c ^ 0xFFFFFFFF
}

fn fnv1a(name: &str) -> u16 {
    let mut h: u32 = 0x811C9DC5;
    for &b in name.as_bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    (h & 0xFFFF) as u16
}

unsafe fn cell_lba(cell_id: u32) -> u32 {
    cell_id * SECTORS_PER_CELL
}

unsafe fn read_cell(cell_id: u32, buf: *mut u8) -> i32 {
    disk_read(cell_lba(cell_id), SECTORS_PER_CELL, buf)
}

unsafe fn write_cell_raw(cell_id: u32, buf: *const u8) -> i32 {
    disk_write(cell_lba(cell_id), SECTORS_PER_CELL, buf)
}

unsafe fn read_hdr(cell_id: u32, hdr: *mut CellHdr) -> i32 {
    let buf = alloc_c(CELL_SIZE as usize);
    if buf.is_null() { return -1; }
    if read_cell(cell_id, buf) != 0 { free_c(buf); return -1; }
    ptr::copy_nonoverlapping(buf, hdr as *mut u8, HDR_SIZE as usize);
    free_c(buf);
    0
}

unsafe fn write_cell_full(cell_id: u32, hdr: *const CellHdr, data: *const u8, data_len: u16) -> i32 {
    let buf = alloc_c(CELL_SIZE as usize);
    if buf.is_null() { return -1; }
    ptr::write_bytes(buf, 0, CELL_SIZE as usize);
    ptr::copy_nonoverlapping(hdr as *const u8, buf, HDR_SIZE as usize);
    if data_len > 0 && !data.is_null() {
        ptr::copy_nonoverlapping(data, buf.add(HDR_SIZE as usize), data_len as usize);
    }
    let data_slice = core::slice::from_raw_parts(buf.add(HDR_SIZE as usize), DATA_SIZE as usize);
    let fresh_crc = crc32(data_slice);
    *(buf.add(32) as *mut u32) = fresh_crc;
    let rc = write_cell_raw(cell_id, buf);
    free_c(buf);
    rc
}

unsafe fn alloc_cell() -> u32 {
    let d = driver();
    let mut pos = d.gc_ptr;
    while pos < d.write_ptr {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) == 0 {
            if hdr.magic == MAGIC && (hdr.cell_type == TYPE_FREE || hdr.cell_type == TYPE_DEAD) {
                d.gc_ptr = pos + 1;
                d.free_count = d.free_count.wrapping_sub(1);
                return pos as u32;
            }
        }
        pos += 1;
    }
    if d.write_ptr >= d.total_cells {
        return 0;
    }
    let cell = d.write_ptr as u32;
    d.write_ptr += 1;
    d.free_count = d.free_count.wrapping_sub(1);
    cell
}

unsafe fn kill_cell(cell_id: u32) {
    let buf = alloc_c(CELL_SIZE as usize);
    if buf.is_null() { return; }
    ptr::write_bytes(buf, 0, CELL_SIZE as usize);
    *(buf as *mut u32) = MAGIC;
    *(buf.add(4) as *mut u32) = 0;
    *(buf.add(8) as *mut u32) = 0;
    *(buf.add(12) as *mut u32) = 0;
    *(buf as *mut u8).add(16) = TYPE_DEAD;
    write_cell_raw(cell_id, buf);
    free_c(buf);
}

pub unsafe fn gc_run() -> i32 {
    let d = driver();
    let mut reclaimed = 0u32;
    let mut pos = 1u64;
    while pos < d.write_ptr {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) != 0 { break; }
        if hdr.magic == MAGIC && hdr.cell_type == TYPE_DEAD {
            kill_cell(pos as u32);
            reclaimed += 1;
        }
        pos += 1;
    }
    if reclaimed > 0 {
        let sb_buf = alloc_c(CELL_SIZE as usize);
        if !sb_buf.is_null() {
            if read_cell(0, sb_buf) == 0 {
                let sb = sb_buf as *mut Superblock;
                (*sb).free_count = d.free_count;
                write_cell_raw(0, sb_buf);
            }
            free_c(sb_buf);
        }
    }
    reclaimed as i32
}

unsafe fn read_sb(sb: *mut Superblock) -> i32 {
    let buf = alloc_c(CELL_SIZE as usize);
    if buf.is_null() { return -1; }
    if read_cell(0, buf) != 0 { free_c(buf); return -1; }
    ptr::copy_nonoverlapping(buf, sb as *mut u8, core::mem::size_of::<Superblock>());
    free_c(buf);
    0
}

unsafe fn write_sb(sb: *const Superblock) -> i32 {
    let buf = alloc_c(CELL_SIZE as usize);
    if buf.is_null() { return -1; }
    ptr::write_bytes(buf, 0, CELL_SIZE as usize);
    ptr::copy_nonoverlapping(sb as *const u8, buf, core::mem::size_of::<Superblock>());
    let _sb_slice = core::slice::from_raw_parts(sb as *const u8, core::mem::size_of::<Superblock>());
    let rc = write_cell_raw(0, buf);
    free_c(buf);
    rc
}

unsafe fn find_chain_tail(chain_id: u32) -> u32 {
    let d = driver();
    let mut found = 0u32;
    let mut pos = 1u64;
    while pos < d.write_ptr {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) != 0 { break; }
        if hdr.magic == MAGIC && hdr.chain_id == chain_id && hdr.cell_type != TYPE_FREE && hdr.cell_type != TYPE_DEAD {
            found = pos as u32;
        }
        pos += 1;
    }
    found
}

unsafe fn find_latest_in_chain(chain_id: u32, cell_type: u8) -> u32 {
    let d = driver();
    let mut best_cell = 0u32;
    let mut best_seq = 0u32;
    let mut pos = 1u64;
    while pos < d.write_ptr {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) != 0 { break; }
        if hdr.magic == MAGIC && hdr.chain_id == chain_id && hdr.cell_type == cell_type
            && hdr.flags == 0 && hdr.seq >= best_seq {
            best_cell = pos as u32;
            best_seq = hdr.seq;
        }
        pos += 1;
    }
    best_cell
}

unsafe fn chain_max_seq(chain_id: u32) -> u32 {
    let d = driver();
    let mut best = 0u32;
    let mut pos = 1u64;
    while pos < d.write_ptr {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) != 0 { break; }
        if hdr.magic == MAGIC && hdr.chain_id == chain_id && hdr.flags == 0 && hdr.seq >= best {
            best = hdr.seq;
        }
        pos += 1;
    }
    best
}

unsafe fn make_hdr(chain_id: u32, seq: u32, cell_type: u8, data_len: u16, name: &str, parent_chain: u32, total_size: u32) -> CellHdr {
    let mut h: CellHdr = core::mem::zeroed();
    h.magic = MAGIC;
    h.chain_id = chain_id;
    h.seq = seq;
    h.cell_type = cell_type;
    h.flags = 0;
    h.name_hash = fnv1a(name);
    h.data_len = data_len;
    h.crc = 0;
    h.parent_chain = parent_chain;
    h.total_size = total_size;
    let now = match driver().disk_io.time_cb {
        Some(f) => f(),
        None => 0,
    };
    h.created = now;
    h.modified = now;
    h
}

unsafe fn append_to_chain(parent_chain: u32, child_chain: u32, cell_type: u8, data: *const u8, data_len: u16, name: &str, total_size: u32) -> i32 {
    let new_cell = alloc_cell();
    if new_cell == 0 { return -1; }

    let seq = chain_max_seq(child_chain).wrapping_add(1);
    let mut hdr = make_hdr(child_chain, seq, cell_type, data_len, name, parent_chain, total_size);

    let tail = find_chain_tail(child_chain);
    if tail != 0 {
        let tail_buf = alloc_c(CELL_SIZE as usize);
        if !tail_buf.is_null() {
            if read_cell(tail, tail_buf) == 0 {
                *(tail_buf.add(12) as *mut u32) = new_cell;
                write_cell_raw(tail, tail_buf);
            }
            free_c(tail_buf);
        }
        hdr.next_cell = 0;
    }

    write_cell_full(new_cell, &hdr, data, data_len)
}

unsafe fn dir_add_entry(parent_chain: u32, child_chain: u32, name: &str, is_dir: bool) -> i32 {
    let name_bytes = name.as_bytes();
    let nlen = name_bytes.len().min(60) as u16;
    let entry_size = 8u16 + nlen;
    let padded = ((entry_size + 3) & !3) as usize;

    let entry_buf = alloc_c(padded);
    if entry_buf.is_null() { return -1; }
    ptr::write_bytes(entry_buf, 0, padded);
    *(entry_buf as *mut u32) = child_chain;
    *(entry_buf.add(4) as *mut u16) = nlen;
    *(entry_buf.add(6) as *mut u8) = if is_dir { 1 } else { 0 };
    ptr::copy_nonoverlapping(name_bytes.as_ptr(), entry_buf.add(8), nlen as usize);

    let seq = chain_max_seq(parent_chain).wrapping_add(1);
    let new_cell = alloc_cell();
    if new_cell == 0 { free_c(entry_buf); return -1; }

    let hdr = make_hdr(parent_chain, seq, TYPE_DIR, padded as u16, name, parent_chain, 0);

    let tail = find_chain_tail(parent_chain);
    if tail != 0 {
        let tail_buf = alloc_c(CELL_SIZE as usize);
        if !tail_buf.is_null() {
            if read_cell(tail, tail_buf) == 0 {
                *(tail_buf.add(12) as *mut u32) = new_cell;
                write_cell_raw(tail, tail_buf);
            }
            free_c(tail_buf);
        }
    }

    let rc = write_cell_full(new_cell, &hdr, entry_buf, padded as u16);
    free_c(entry_buf);
    rc
}

unsafe fn dir_remove_entry(parent_chain: u32, name: &str) -> i32 {
    let d = driver();
    let mut pos = 1u64;
    while pos < d.write_ptr {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) != 0 { break; }
        if hdr.magic == MAGIC && hdr.chain_id == parent_chain && hdr.cell_type == TYPE_DIR && hdr.flags == 0 {
            let buf = alloc_c(CELL_SIZE as usize);
            if buf.is_null() { break; }
            if read_cell(pos as u32, buf) != 0 { free_c(buf); break; }
            let data = buf.add(HDR_SIZE as usize);
            let mut off = 0u32;
            while (off + 8) <= hdr.data_len as u32 {
                let _child = *(data.add(off as usize) as *const u32);
                let nlen = *(data.add(off as usize + 4) as *const u16) as u32;
                if off + 8 + nlen > hdr.data_len as u32 { break; }
                let entry_name = core::str::from_utf8(core::slice::from_raw_parts(data.add(off as usize + 8), nlen as usize)).unwrap_or("");
                if entry_name == name {
                    kill_cell(pos as u32);
                    free_c(buf);
                    return 0;
                }
                off += 8 + nlen;
                off = (off + 3) & !3;
            }
            free_c(buf);
        }
        pos += 1;
    }
    -1
}

unsafe fn dir_find_entry(parent_chain: u32, name: &str) -> u32 {
    let d = driver();
    let mut result = 0u32;
    let mut pos = 1u64;
    while pos < d.write_ptr {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) != 0 { break; }
        if hdr.magic == MAGIC && hdr.chain_id == parent_chain && hdr.cell_type == TYPE_DIR && hdr.flags == 0 {
            let buf = alloc_c(CELL_SIZE as usize);
            if buf.is_null() { break; }
            if read_cell(pos as u32, buf) != 0 { free_c(buf); break; }
            let data = buf.add(HDR_SIZE as usize);
            let mut off = 0u32;
            while (off + 8) <= hdr.data_len as u32 {
                let child = *(data.add(off as usize) as *const u32);
                let nlen = *(data.add(off as usize + 4) as *const u16) as u32;
                if off + 8 + nlen > hdr.data_len as u32 { break; }
                let entry_name = core::str::from_utf8(core::slice::from_raw_parts(data.add(off as usize + 8), nlen as usize)).unwrap_or("");
                if entry_name == name && child != 0 {
                    result = child;
                    free_c(buf);
                    return result;
                }
                off += 8 + nlen;
                off = (off + 3) & !3;
            }
            free_c(buf);
        }
        pos += 1;
    }
    result
}

unsafe fn resolve_path_inner(path: &str) -> u32 {
    if path == "/" || path.is_empty() { return ROOT_DIR_ID; }
    let mut current = ROOT_DIR_ID;
    let bytes = path.as_bytes();
    let mut start = 0;
    if bytes[0] == b'/' { start = 1; }
    while start < bytes.len() {
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'/' { end += 1; }
        if end == start { start += 1; continue; }
        let component = core::str::from_utf8(&bytes[start..end]).unwrap_or("");
        if component == "." { start = end + 1; continue; }
        if component == ".." {
            let d = driver();
            let mut pos = 1u64;
            let mut found_parent = current;
            while pos < d.write_ptr {
                let mut hdr: CellHdr = core::mem::zeroed();
                if read_hdr(pos as u32, &mut hdr) != 0 { break; }
                if hdr.magic == MAGIC && hdr.chain_id == current && hdr.flags == 0
                    && (hdr.cell_type == TYPE_FILE_HDR || hdr.cell_type == TYPE_DIR)
                    && hdr.seq == chain_max_seq(current) {
                    found_parent = hdr.parent_chain;
                    break;
                }
                pos += 1;
            }
            current = found_parent;
            start = end + 1;
            continue;
        }
        let found = dir_find_entry(current, component);
        if found == 0 { return 0; }
        current = found;
        start = end + 1;
    }
    current
}

pub unsafe fn init() -> i32 {
    let mut sb: Superblock = core::mem::zeroed();
    if read_sb(&mut sb) != 0 { return -1; }
    if sb.magic != MAGIC { return -1; }
    if sb.version != VERSION { return -1; }
    let d = driver();
    d.initialized = true;
    d.total_cells = sb.total_cells;
    d.write_ptr = sb.write_ptr;
    d.gc_ptr = sb.gc_ptr;
    d.root_dir_cell = sb.root_dir_cell;
    d.file_count = sb.file_count;
    d.free_count = sb.free_count;
    let mut max_id = 0u32;
    let mut pos = 1u64;
    while pos < sb.write_ptr {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) != 0 { break; }
        if hdr.magic == MAGIC && hdr.chain_id > max_id {
            max_id = hdr.chain_id;
        }
        pos += 1;
    }
    d.next_chain_id = max_id + 1;
    0
}

pub unsafe fn set_drive(read_cb: Option<FatReadFn>, write_cb: Option<FatWriteFn>) {
    let d = driver();
    d.disk_io.read_cb = read_cb;
    d.disk_io.write_cb = write_cb;
    d.initialized = false;
}

pub unsafe fn set_alloc(alloc_cb: Option<AllocFn>, free_cb: Option<FreeFn>) {
    let d = driver();
    d.disk_io.alloc_cb = alloc_cb;
    d.disk_io.free_cb = free_cb;
}

pub unsafe fn set_time(time_cb: Option<TimeFn>) {
    let d = driver();
    d.disk_io.time_cb = time_cb;
}

pub unsafe fn reinit() -> i32 {
    if driver().disk_io.read_cb.is_none() { return -1; }
    driver().initialized = false;
    init()
}

pub unsafe fn format_at(_start_lba: u64, total_sectors: u64) -> i32 {
    let total_cells = total_sectors / SECTORS_PER_CELL as u64;
    if total_cells < 16 { return -1; }

    let zero_buf = alloc_c(CELL_SIZE as usize);
    if zero_buf.is_null() { return -1; }
    ptr::write_bytes(zero_buf, 0, CELL_SIZE as usize);
    let mut c = 0u64;
    while c < total_cells.min(256) {
        disk_write((c * SECTORS_PER_CELL as u64) as u32, SECTORS_PER_CELL, zero_buf);
        c += 1;
    }
    free_c(zero_buf);

    let _root_chain = 1u32;
    let root_dir_id = ROOT_DIR_ID;

    let mut root_hdr: CellHdr = core::mem::zeroed();
    root_hdr.magic = MAGIC;
    root_hdr.chain_id = root_dir_id;
    root_hdr.seq = 0;
    root_hdr.next_cell = 0;
    root_hdr.cell_type = TYPE_DIR;
    root_hdr.flags = 0;
    root_hdr.name_hash = fnv1a("/");
    root_hdr.data_len = 0;
    root_hdr.parent_chain = root_dir_id;
    root_hdr.total_size = 0;
    let empty: [u8; 0] = [];
    write_cell_full(1, &root_hdr, empty.as_ptr(), 0);

    let mut sb: Superblock = core::mem::zeroed();
    sb.magic = MAGIC;
    sb.version = VERSION;
    sb.flags = 0;
    sb.total_cells = total_cells;
    sb.write_ptr = 2;
    sb.gc_ptr = 2;
    sb.root_dir_cell = root_dir_id;
    sb.file_count = 0;
    sb.free_count = total_cells - 2;
    let label = b"LumFS CellChain       ";
    let mut i = 0;
    while i < 32 && i < label.len() { sb.volume_label[i] = label[i]; i += 1; }
    write_sb(&sb);

    let d = driver();
    d.initialized = true;
    d.total_cells = total_cells;
    d.write_ptr = 2;
    d.gc_ptr = 2;
    d.root_dir_cell = root_dir_id;
    d.file_count = 0;
    d.free_count = total_cells - 2;
    d.next_chain_id = 2;

    0
}

pub unsafe fn read_file(path: *const u8, buffer: *mut u8, max_size: u32) -> i32 {
    let path_str = path_to_str(path);
    let chain_id = resolve_path_inner(path_str);
    if chain_id == 0 { return -1; }

    let hdr_cell = find_latest_in_chain(chain_id, TYPE_FILE_HDR);
    if hdr_cell == 0 { return -1; }

    let mut hdr: CellHdr = core::mem::zeroed();
    if read_hdr(hdr_cell, &mut hdr) != 0 { return -1; }
    if hdr.cell_type != TYPE_FILE_HDR { return -1; }

    let total = hdr.total_size as u32;
    let to_read = total.min(max_size);

    if to_read == 0 { return 0; }

    let tmp = alloc_c(CELL_SIZE as usize);
    if tmp.is_null() { return -1; }
    if read_cell(hdr_cell, tmp) != 0 { free_c(tmp); return -1; }
    let hdr_data_len = hdr.data_len as u32;
    let hdr_inline = hdr_data_len.min(DATA_SIZE).min(to_read);
    if hdr_inline > 0 {
        ptr::copy_nonoverlapping(tmp.add(HDR_SIZE as usize), buffer, hdr_inline as usize);
    }
    free_c(tmp);

    let mut total_read = hdr_inline;
    let mut next = hdr.next_cell;

    while total_read < to_read && next != 0 {
        let mut dhdr: CellHdr = core::mem::zeroed();
        if read_hdr(next, &mut dhdr) != 0 { break; }
        if dhdr.cell_type != TYPE_FILE_DATA || dhdr.flags != 0 { break; }

        let dbuf = alloc_c(CELL_SIZE as usize);
        if dbuf.is_null() { break; }
        if read_cell(next, dbuf) != 0 { free_c(dbuf); break; }
        let chunk = (dhdr.data_len as u32).min(to_read - total_read).min(DATA_SIZE);
        ptr::copy_nonoverlapping(dbuf.add(HDR_SIZE as usize), buffer.add(total_read as usize), chunk as usize);
        free_c(dbuf);
        total_read += chunk;
        next = dhdr.next_cell;
    }

    total_read as i32
}

pub unsafe fn write_file(path: *const u8, data: *const u8, size: u32) -> i32 {
    let path_str = path_to_str(path);
    let parts = split_path(path_str);
    let parent_path = parts.0;
    let filename = parts.1;
    if filename.is_empty() { return -1; }

    let parent_chain = resolve_path_inner(parent_path);
    if parent_chain == 0 { return -1; }

    let existing = dir_find_entry(parent_chain, filename);
    if existing != 0 {
        dir_remove_entry(parent_chain, filename);
        let d = driver();
        let mut pos = 1u64;
        while pos < d.write_ptr {
            let mut hdr: CellHdr = core::mem::zeroed();
            if read_hdr(pos as u32, &mut hdr) != 0 { break; }
            if hdr.magic == MAGIC && hdr.chain_id == existing && hdr.flags == 0 {
                kill_cell(pos as u32);
            }
            pos += 1;
        }
        d.file_count = d.file_count.wrapping_sub(1);
    }

    let d = driver();
    let new_chain = d.next_chain_id;
    d.next_chain_id += 1;

    let inline_data_len = size.min(DATA_SIZE);
    let hdr_cell = alloc_cell();
    if hdr_cell == 0 { return -1; }

    let hdr = make_hdr(new_chain, 0, TYPE_FILE_HDR, inline_data_len as u16, filename, parent_chain, size);

    let _tail = find_chain_tail(new_chain);

    let hdr_buf = alloc_c(CELL_SIZE as usize);
    if hdr_buf.is_null() { return -1; }
    ptr::write_bytes(hdr_buf, 0, CELL_SIZE as usize);
    ptr::copy_nonoverlapping(&hdr as *const CellHdr as *const u8, hdr_buf, HDR_SIZE as usize);
    if inline_data_len > 0 && !data.is_null() {
        ptr::copy_nonoverlapping(data, hdr_buf.add(HDR_SIZE as usize), inline_data_len as usize);
    }
    let data_slice = core::slice::from_raw_parts(hdr_buf.add(HDR_SIZE as usize), DATA_SIZE as usize);
    *(hdr_buf.add(32) as *mut u32) = crc32(data_slice);
    if write_cell_raw(hdr_cell, hdr_buf) != 0 { free_c(hdr_buf); return -1; }
    free_c(hdr_buf);

    let mut written = inline_data_len;
    let mut prev_cell = hdr_cell;

    while written < size {
        let data_cell = alloc_cell();
        if data_cell == 0 { return -1; }

        let remaining = size - written;
        let chunk = remaining.min(DATA_SIZE);

        let mut dhdr = make_hdr(new_chain, (written / DATA_SIZE + 1) as u32, TYPE_FILE_DATA, chunk as u16, filename, parent_chain, size);
        dhdr.next_cell = 0;

        let prev_buf = alloc_c(CELL_SIZE as usize);
        if !prev_buf.is_null() {
            if read_cell(prev_cell, prev_buf) == 0 {
                *(prev_buf.add(12) as *mut u32) = data_cell;
                write_cell_raw(prev_cell, prev_buf);
            }
            free_c(prev_buf);
        }

        let dbuf = alloc_c(CELL_SIZE as usize);
        if dbuf.is_null() { return -1; }
        ptr::write_bytes(dbuf, 0, CELL_SIZE as usize);
        ptr::copy_nonoverlapping(&dhdr as *const CellHdr as *const u8, dbuf, HDR_SIZE as usize);
        ptr::copy_nonoverlapping(data.add(written as usize), dbuf.add(HDR_SIZE as usize), chunk as usize);
        let data_slice = core::slice::from_raw_parts(dbuf.add(HDR_SIZE as usize), DATA_SIZE as usize);
        *(dbuf.add(32) as *mut u32) = crc32(data_slice);
        if write_cell_raw(data_cell, dbuf) != 0 { free_c(dbuf); return -1; }
        free_c(dbuf);

        prev_cell = data_cell;
        written += chunk;
    }

    dir_add_entry(parent_chain, new_chain, filename, false);
    d.file_count = d.file_count.wrapping_add(1);

    0
}

pub unsafe fn list_dir(path: *const u8, entries: *mut LumieDirEnt, max_entries: i32) -> i32 {
    let path_str = path_to_str(path);
    let chain_id = resolve_path_inner(path_str);
    if chain_id == 0 { return -1; }

    let d = driver();
    let mut count: i32 = 0;
    let mut pos = 1u64;

    while pos < d.write_ptr && count < max_entries {
        let mut hdr: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut hdr) != 0 { break; }
        if hdr.magic == MAGIC && hdr.chain_id == chain_id && hdr.cell_type == TYPE_DIR && hdr.flags == 0 {
            let buf = alloc_c(CELL_SIZE as usize);
            if buf.is_null() { break; }
            if read_cell(pos as u32, buf) != 0 { free_c(buf); break; }
            let data = buf.add(HDR_SIZE as usize);
            let mut off = 0u32;
            while (off + 8) <= hdr.data_len as u32 && count < max_entries {
                let child_chain = *(data.add(off as usize) as *const u32);
                let nlen = *(data.add(off as usize + 4) as *const u16) as u32;
                let is_dir = *(data.add(off as usize + 6) as *const u8);
                if off + 8 + nlen > hdr.data_len as u32 { break; }

                let entry = entries.add(count as usize);
                let mut j = 0;
                while j < nlen as usize && j < 255 {
                    (*entry).name[j] = *data.add(off as usize + 8 + j);
                    j += 1;
                }
                (*entry).name[j] = 0;
                (*entry).is_dir = is_dir;

                if is_dir == 0 {
                    let child_hdr_cell = find_latest_in_chain(child_chain, TYPE_FILE_HDR);
                    if child_hdr_cell != 0 {
                        let mut ch: CellHdr = core::mem::zeroed();
                        if read_hdr(child_hdr_cell, &mut ch) == 0 {
                            (*entry).size = ch.total_size;
                        }
                    }
                } else {
                    (*entry).size = 0;
                }
                count += 1;
                off += 8 + nlen;
                off = (off + 3) & !3;
            }
            free_c(buf);
        }
        pos += 1;
    }
    count
}

pub unsafe fn exists(path: *const u8) -> bool {
    resolve_path_inner(path_to_str(path)) != 0
}

pub unsafe fn get_file_size(path: *const u8) -> i32 {
    let chain_id = resolve_path_inner(path_to_str(path));
    if chain_id == 0 { return -1; }
    let hdr_cell = find_latest_in_chain(chain_id, TYPE_FILE_HDR);
    if hdr_cell == 0 { return -1; }
    let mut hdr: CellHdr = core::mem::zeroed();
    if read_hdr(hdr_cell, &mut hdr) != 0 { return -1; }
    hdr.total_size as i32
}

pub unsafe fn delete(path: *const u8) -> i32 {
    let path_str = path_to_str(path);
    if path_str == "/" { return -1; }
    let chain_id = resolve_path_inner(path_str);
    if chain_id == 0 { return -1; }

    let hdr_cell = find_latest_in_chain(chain_id, TYPE_FILE_HDR);
    if hdr_cell == 0 { return -1; }
    let mut hdr: CellHdr = core::mem::zeroed();
    if read_hdr(hdr_cell, &mut hdr) != 0 { return -1; }

    let parent_chain = hdr.parent_chain;
    let name_ptr = path_to_str(path);
    let name = path_name(name_ptr);

    dir_remove_entry(parent_chain, name);

    let d = driver();
    let mut pos = 1u64;
    while pos < d.write_ptr {
        let mut h: CellHdr = core::mem::zeroed();
        if read_hdr(pos as u32, &mut h) != 0 { break; }
        if h.magic == MAGIC && h.chain_id == chain_id && h.flags == 0 {
            kill_cell(pos as u32);
        }
        pos += 1;
    }
    d.file_count = d.file_count.wrapping_sub(1);
    0
}

pub unsafe fn mkdir(path: *const u8) -> i32 {
    let path_str = path_to_str(path);
    let parts = split_path(path_str);
    let parent_path = parts.0;
    let dirname = parts.1;
    if dirname.is_empty() { return -1; }

    let parent_chain = resolve_path_inner(parent_path);
    if parent_chain == 0 { return -1; }
    if dir_find_entry(parent_chain, dirname) != 0 { return -1; }

    let d = driver();
    let new_chain = d.next_chain_id;
    d.next_chain_id += 1;

    let hdr_cell = alloc_cell();
    if hdr_cell == 0 { return -1; }

    let mut hdr = make_hdr(new_chain, 0, TYPE_DIR, 0, dirname, parent_chain, 0);
    hdr.next_cell = 0;

    let hdr_buf = alloc_c(CELL_SIZE as usize);
    if hdr_buf.is_null() { return -1; }
    ptr::write_bytes(hdr_buf, 0, CELL_SIZE as usize);
    ptr::copy_nonoverlapping(&hdr as *const CellHdr as *const u8, hdr_buf, HDR_SIZE as usize);
    let data_slice = core::slice::from_raw_parts(hdr_buf.add(HDR_SIZE as usize), DATA_SIZE as usize);
    *(hdr_buf.add(32) as *mut u32) = crc32(data_slice);
    write_cell_raw(hdr_cell, hdr_buf);
    free_c(hdr_buf);

    dir_add_entry(parent_chain, new_chain, dirname, true);
    0
}

pub unsafe fn rename(old_path: *const u8, new_path: *const u8) -> i32 {
    let old_str = path_to_str(old_path);
    let new_str = path_to_str(new_path);
    if old_str.is_empty() || new_str.is_empty() || old_str == "/" { return -1; }

    let chain_id = resolve_path_inner(old_str);
    if chain_id == 0 { return -1; }

    let new_parts = split_path(new_str);
    let new_parent_chain = resolve_path_inner(new_parts.0);
    if new_parent_chain == 0 { return -1; }
    let new_name = new_parts.1;
    if new_name.is_empty() { return -1; }

    let old_parts = split_path(old_str);
    let old_parent_chain = resolve_path_inner(old_parts.0);
    if old_parent_chain == 0 { return -1; }

    dir_remove_entry(old_parent_chain, old_parts.1);

    let is_dir = find_latest_in_chain(chain_id, TYPE_DIR) != 0;
    let hdr_cell = find_latest_in_chain(chain_id, TYPE_FILE_HDR);
    if hdr_cell != 0 {
        let buf = alloc_c(CELL_SIZE as usize);
        if !buf.is_null() {
            if read_cell(hdr_cell, buf) == 0 {
                let nb = new_name.as_bytes();
                let _nlen = nb.len().min(60) as u16;
                *(buf.add(HDR_SIZE as usize) as *mut u16) = 0;
                ptr::write_bytes(buf.add(HDR_SIZE as usize), 0, DATA_SIZE as usize);
                let mut h: CellHdr = core::mem::zeroed();
                ptr::copy_nonoverlapping(buf, &mut h as *mut CellHdr as *mut u8, HDR_SIZE as usize);
                h.name_hash = fnv1a(new_name);
                h.parent_chain = new_parent_chain;
                let new_seq = chain_max_seq(chain_id).wrapping_add(1);
                h.seq = new_seq;
                let hdr2_buf = alloc_c(CELL_SIZE as usize);
                if !hdr2_buf.is_null() {
                    ptr::write_bytes(hdr2_buf, 0, CELL_SIZE as usize);
                    ptr::copy_nonoverlapping(&h as *const CellHdr as *const u8, hdr2_buf, HDR_SIZE as usize);
                    *(hdr2_buf.add(32) as *mut u32) = crc32(core::slice::from_raw_parts(hdr2_buf.add(HDR_SIZE as usize), DATA_SIZE as usize));
                    write_cell_raw(hdr_cell, hdr2_buf);
                    free_c(hdr2_buf);
                }
            }
            free_c(buf);
        }
    }

    dir_add_entry(new_parent_chain, chain_id, new_name, is_dir);
    0
}

pub unsafe fn copy_file(src_path: *const u8, dst_path: *const u8) -> i32 {
    let src_str = path_to_str(src_path);
    let dst_str = path_to_str(dst_path);
    if src_str.is_empty() || dst_str.is_empty() { return -1; }

    let src_chain = resolve_path_inner(src_str);
    if src_chain == 0 { return -1; }

    let hdr_cell = find_latest_in_chain(src_chain, TYPE_FILE_HDR);
    if hdr_cell == 0 { return -1; }
    let mut hdr: CellHdr = core::mem::zeroed();
    if read_hdr(hdr_cell, &mut hdr) != 0 { return -1; }

    let total = hdr.total_size as u32;
    if total == 0 {
        return write_file(dst_path, core::ptr::null(), 0);
    }

    let buf = alloc_c(total as usize);
    if buf.is_null() { return -1; }
    let mut cur = hdr_cell;
    let mut read_off = 0u32;

    {
        let tmp = alloc_c(CELL_SIZE as usize);
        if !tmp.is_null() {
            if read_cell(cur, tmp) == 0 {
                let inline = hdr.data_len as u32;
                if inline > 0 {
                    ptr::copy_nonoverlapping(tmp.add(HDR_SIZE as usize), buf, inline.min(total) as usize);
                    read_off = inline.min(total);
                }
            }
            free_c(tmp);
        }
    }

    cur = hdr.next_cell;
    while read_off < total && cur != 0 {
        let mut dhdr: CellHdr = core::mem::zeroed();
        if read_hdr(cur, &mut dhdr) != 0 { break; }
        if dhdr.flags != 0 { break; }
        let tmp = alloc_c(CELL_SIZE as usize);
        if tmp.is_null() { break; }
        if read_cell(cur, tmp) != 0 { free_c(tmp); break; }
        let chunk = (dhdr.data_len as u32).min(total - read_off).min(DATA_SIZE);
        ptr::copy_nonoverlapping(tmp.add(HDR_SIZE as usize), buf.add(read_off as usize), chunk as usize);
        free_c(tmp);
        read_off += chunk;
        cur = dhdr.next_cell;
    }

    let ret = write_file(dst_path, buf, read_off);
    free_c(buf);
    ret
}



fn split_path(path: &str) -> (&str, &str) {
    let bytes = path.as_bytes();
    let mut last_slash = path.len();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'/' {
            last_slash = i;
            break;
        }
    }
    if last_slash == 0 {
        (&path[..1], &path[1..])
    } else {
        (&path[..last_slash], &path[last_slash + 1..])
    }
}

fn path_name<'a>(path: &'a str) -> &'a str {
    let parts = split_path(path);
    parts.1
}

unsafe fn path_to_str<'a>(path: *const u8) -> &'a str {
    if path.is_null() { return ""; }
    let mut len = 0;
    while len < 256 && *path.add(len) != 0 { len += 1; }
    core::str::from_utf8(core::slice::from_raw_parts(path, len)).unwrap_or("")
}
