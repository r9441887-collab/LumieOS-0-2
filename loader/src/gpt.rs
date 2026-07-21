
use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;

const GPT_SIGNATURE: [u8; 8] = *b"EFI PART";
const GPT_REVISION: u32 = 0x00010000;
const GPT_HEADER_SIZE: u32 = 92;
const GPT_PARTITION_ENTRY_SIZE: u32 = 128;
const GPT_MAX_PARTITIONS: u32 = 128;
const GPT_PARTITIONS_START_LBA: u64 = 2;
const _GPT_PARTITIONS_END_LBA: u64 = 33;
const GPT_FIRST_USABLE_LBA: u64 = 34;
const GPT_ALIGNMENT: u64 = 2048;

#[repr(C)]
struct GptHeader {
    signature: [u8; 8],
    revision: u32,
    header_size: u32,
    crc32: u32,
    reserved: u32,
    my_lba: u64,
    alternate_lba: u64,
    first_usable_lba: u64,
    last_usable_lba: u64,
    disk_guid: [u8; 16],
    partition_entry_lba: u64,
    num_partition_entries: u32,
    size_of_partition_entry: u32,
    partition_entries_crc: u32,
}

#[repr(C)]
struct GptPartitionEntry {
    partition_type_guid: [u8; 16],
    unique_partition_guid: [u8; 16],
    starting_lba: u64,
    ending_lba: u64,
    attributes: u64,
    name: [u8; 72],
}

const LUMIEOS_PARTITION_GUID: [u8; 16] = [
    0xAF, 0xE4, 0x3C, 0xE8, 0x65, 0xCF, 0x4C, 0x4A,
    0xB5, 0x7D, 0x5A, 0x4A, 0x7F, 0x4B, 0xEE, 0x2B,
];

const EFI_SYSTEM_PARTITION_GUID: [u8; 16] = [
    0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11,
    0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
];

type EfiBlockIoReadBlocks =
    Option<unsafe extern "efiapi" fn(*mut c_void, u32, u64, u64, *mut c_void) -> u64>;
type EfiBlockIoWriteBlocks =
    Option<unsafe extern "efiapi" fn(*mut c_void, u32, u64, u64, *mut c_void) -> u64>;

#[repr(C)]
struct EfiBlockIoMedia {
    pub media_id: u32,
    pub removable_media: u8,
    pub media_present: u8,
    pub logical_partition: u8,
    pub read_only: u8,
    pub write_caching: u8,
    pub pad: [u8; 3],
    pub block_size: u32,
    pub io_align: u32,
    pub last_block: u64,
    pub lowest_aligned_lba: u64,
    pub logical_blocks_per_physical_block: u32,
    pub optimal_transfer_length_granularity: u32,
}

#[repr(C)]
struct EfiBlockIoProtocol {
    pub revision: u64,
    pub media: *mut EfiBlockIoMedia,
    pub reset: *mut c_void,
    pub read_blocks: EfiBlockIoReadBlocks,
    pub write_blocks: EfiBlockIoWriteBlocks,
    pub flush_blocks: *mut c_void,
}

fn get_block_io(device_handle: efi_handle) -> Option<*mut EfiBlockIoProtocol> {
    let bs = match crate::input::get_ld_st() {
        st if !st.is_null() => unsafe { &*st }.boot_services,
        _ => return None,
    };
    if bs.is_null() { return None; }
    let bio_guid = &EFI_BLOCK_IO_GUID as *const EfiGuid;
    let mut bio: *mut EfiBlockIoProtocol = ptr::null_mut();
    let st = unsafe {
        if let Some(hp) = (*bs).handle_protocol {
            hp(device_handle, bio_guid, &mut bio as *mut *mut EfiBlockIoProtocol as *mut *mut c_void)
        } else { return None; }
    };
    if st != 0 || bio.is_null() { None } else { Some(bio) }
}

fn _read_sectors(bio: *mut EfiBlockIoProtocol, lba: u64, count: u64, buf: *mut u8) -> i32 {
    unsafe {
        let media = (*bio).media;
        if media.is_null() { return -1; }
        let rb = match (*bio).read_blocks { Some(r) => r, None => return -1 };
        let st = rb(bio as *mut c_void, (*media).media_id, lba, count * (*media).block_size as u64, buf as *mut c_void);
        if st == 0 { 0 } else { -1 }
    }
}

fn write_sectors(bio: *mut EfiBlockIoProtocol, lba: u64, count: u64, buf: *const u8) -> i32 {
    unsafe {
        let media = (*bio).media;
        if media.is_null() { return -1; }
        let wb = match (*bio).write_blocks { Some(w) => w, None => return -1 };
        let st = wb(bio as *mut c_void, (*media).media_id, lba, count * (*media).block_size as u64, buf as *mut c_void);
        if st == 0 { 0 } else { -1 }
    }
}

fn read_sector(bio: *mut EfiBlockIoProtocol, lba: u64, buf: *mut u8) -> i32 {
    unsafe {
        let media = (*bio).media;
        if media.is_null() { return -1; }
        let rb = match (*bio).read_blocks { Some(r) => r, None => return -1 };
        let st = rb(bio as *mut c_void, (*media).media_id, lba, (*media).block_size as u64, buf as *mut c_void);
        if st == 0 { 0 } else { -1 }
    }
}

fn flush_sectors(bio: *mut EfiBlockIoProtocol) -> i32 {
    unsafe {
        let fl_ptr = (*bio).flush_blocks;
        if fl_ptr.is_null() { return 0; }
        type FlushFn = unsafe extern "efiapi" fn(*mut c_void) -> efi_status;
        let fl: FlushFn = core::mem::transmute(fl_ptr);
        let st = fl(bio as *mut c_void);
        if st == 0 { 0 } else { -1 }
    }
}

pub fn check_writable(device_handle: efi_handle) -> Result<(), &'static str> {
    let bio = match get_block_io(device_handle) { Some(b) => b, None => return Err("Block I/O protocol not found") };
    unsafe {
        let media = (*bio).media;
        if media.is_null() { return Err("Media info unavailable"); }
        if (*media).read_only != 0 { return Err("Device is read-only"); }
        if (*media).media_present == 0 { return Err("No media present"); }
    }

    let mut orig: [u8; 512] = [0u8; 512];
    let mut backup: [u8; 512] = [0u8; 512];
    if read_sector(bio, 0, orig.as_mut_ptr()) != 0 {
        return Err("Cannot read sector 0");
    }
    backup.copy_from_slice(&orig);

    if write_sectors(bio, 0, 1, orig.as_ptr()) != 0 {
        return Err("Write test failed - device may be locked");
    }
    flush_sectors(bio);

    if write_sectors(bio, 0, 1, backup.as_ptr()) != 0 {
        return Err("Restore after write test failed");
    }
    flush_sectors(bio);

    Ok(())
}

pub fn disk_total_sectors(device_handle: efi_handle) -> u64 {
    let bio = match get_block_io(device_handle) { Some(b) => b, None => return 0 };
    unsafe {
        let media = (*bio).media;
        if media.is_null() { return 0; }
        (*media).last_block + 1
    }
}

pub fn disk_sector_size(device_handle: efi_handle) -> u64 {
    let bio = match get_block_io(device_handle) { Some(b) => b, None => return 512 };
    unsafe {
        let media = (*bio).media;
        if media.is_null() { return 512; }
        (*media).block_size as u64
    }
}

fn crc32(buf: *const u8, len: u64) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for i in 0..len as usize {
        let byte = unsafe { *buf.add(i) };
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc ^ 0xFFFFFFFF
}

fn make_protective_mbr(sector: &mut [u8; 512], total_sectors: u64) {
    for b in sector.iter_mut() { *b = 0; }
    sector[0] = 0x00;
    sector[510] = 0x55;
    sector[511] = 0xAA;
    let total_lba = if total_sectors > 0xFFFFFFFF { 0xFFFFFFFF } else { total_sectors as u32 };
    sector[446] = 0x00;
    sector[447] = 0x00;
    sector[448] = 0x02;
    sector[449] = 0x00;
    sector[450] = 0xEE;
    sector[451] = 0x00;
    sector[452] = 0x00;
    sector[453] = 0x00;
    sector[454] = 0x00;
    sector[455] = 0x00;
    sector[456] = 0x00;
    sector[457] = 0x00;
    sector[458] = 0x00;
    sector[459] = 0x02;
    sector[460] = (total_lba & 0xFF) as u8;
    sector[461] = ((total_lba >> 8) & 0xFF) as u8;
    sector[462] = ((total_lba >> 16) & 0xFF) as u8;
    sector[463] = ((total_lba >> 24) & 0xFF) as u8;
}

fn make_gpt_header(
    header: &mut GptHeader,
    total_sectors: u64,
    part_start_lba: u64,
    part_end_lba: u64,
    disk_guid: &[u8; 16],
    partition_entries_crc: u32,
) {
    let last_lba = total_sectors - 1;
    let backup_header_lba = last_lba;
    let _backup_partitions_lba = last_lba - 32;

    header.signature = GPT_SIGNATURE;
    header.revision = GPT_REVISION;
    header.header_size = GPT_HEADER_SIZE;
    header.crc32 = 0;
    header.reserved = 0;
    header.my_lba = 1;
    header.alternate_lba = backup_header_lba;
    header.first_usable_lba = part_start_lba;
    header.last_usable_lba = part_end_lba;
    header.disk_guid = *disk_guid;
    header.partition_entry_lba = GPT_PARTITIONS_START_LBA;
    header.num_partition_entries = GPT_MAX_PARTITIONS;
    header.size_of_partition_entry = GPT_PARTITION_ENTRY_SIZE;
    header.partition_entries_crc = partition_entries_crc;
}

fn make_partition_entry(
    entry: &mut GptPartitionEntry,
    start_lba: u64,
    end_lba: u64,
    part_guid: &[u8; 16],
    is_esp: bool,
) {
    entry.partition_type_guid = if is_esp { EFI_SYSTEM_PARTITION_GUID } else { LUMIEOS_PARTITION_GUID };
    entry.unique_partition_guid = *part_guid;
    entry.starting_lba = start_lba;
    entry.ending_lba = end_lba;
    entry.attributes = 0;
    entry.name = [0u8; 72];
    if is_esp {
        let nb = b"EFI System Partition\0";
        let mut i = 0;
        while i < nb.len() && i < 71 {
            entry.name[i] = nb[i];
            i += 1;
        }
    } else {
        let nb = b"LumieOS\0";
        let mut i = 0;
        while i < nb.len() && i < 71 {
            entry.name[i] = nb[i];
            i += 1;
        }
    }
}

pub type GptProgressCb = Option<unsafe extern "efiapi" fn(*const u8, i32)>;

/// Result of creating a dual-partition GPT layout:
/// - ESP partition (FAT32) for bootloader
/// - LumFS partition for the OS
pub struct DualPartitionResult {
    pub esp_start: u64,
    pub esp_sectors: u64,
    pub lumfs_start: u64,
    pub lumfs_sectors: u64,
}

pub fn create_gpt_dual_partitions(
    device_handle: efi_handle,
    size_gb: u64,
    esp_size_mb: u64,
    progress: GptProgressCb,
) -> Option<DualPartitionResult> {
    let bio = match get_block_io(device_handle) { Some(b) => b, None => return None };
    let total_sectors = disk_total_sectors(device_handle);
    let sector_size = disk_sector_size(device_handle);
    if total_sectors < GPT_ALIGNMENT + 100 { return None; }

    let last_usable_lba = total_sectors - GPT_ALIGNMENT - 33 - 1;

    /* Calculate ESP size in sectors (default 260 MB, minimum 100 MB) */
    let esp_mb = if esp_size_mb < 100 { 260 } else { esp_size_mb };
    let esp_sectors_raw = (esp_mb * 1024 * 1024) / sector_size;
    let esp_sectors = (esp_sectors_raw / GPT_ALIGNMENT) * GPT_ALIGNMENT;
    if esp_sectors < GPT_ALIGNMENT { return None; }

    /* Calculate LumFS partition size */
    let requested_sectors = (size_gb * 1024 * 1024 * 1024) / sector_size;
    let max_lumfs = last_usable_lba - GPT_ALIGNMENT - esp_sectors;
    let lumfs_sectors_raw = if requested_sectors > max_lumfs { max_lumfs } else { requested_sectors };
    let lumfs_sectors = (lumfs_sectors_raw / GPT_ALIGNMENT) * GPT_ALIGNMENT;
    if lumfs_sectors < GPT_ALIGNMENT { return None; }

    /* Partition layout:
     *   LBA 0:       Protective MBR
     *   LBA 1:       GPT Header
     *   LBA 2-33:    Partition entries (128 entries * 128 bytes = 32 sectors)
     *   LBA 34..:    ESP starts at GPT_ALIGNMENT (2048)
     *   After ESP:   LumFS starts
     */
    let esp_start = GPT_ALIGNMENT;
    let esp_end = esp_start + esp_sectors - 1;
    let lumfs_start = esp_end + 1;
    let lumfs_end = lumfs_start + lumfs_sectors - 1;

    let disk_guid: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
    ];

    let esp_guid: [u8; 16] = [
        0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11,
        0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B,
    ];

    let lumfs_guid: [u8; 16] = [
        0xAF, 0xE4, 0x3C, 0xE8, 0x65, 0xCF, 0x4C, 0x4A,
        0xB5, 0x7D, 0x5A, 0x4A, 0x7F, 0x4B, 0xEE, 0x2B,
    ];

    /* Build partition entries (2 entries: ESP + LumFS) */
    let mut partition_entries: [u8; 128 * 128] = [0u8; 128 * 128];

    /* Entry 0: ESP */
    let mut esp_entry: GptPartitionEntry = unsafe { core::mem::zeroed() };
    esp_entry.partition_type_guid = EFI_SYSTEM_PARTITION_GUID;
    esp_entry.unique_partition_guid = esp_guid;
    esp_entry.starting_lba = esp_start;
    esp_entry.ending_lba = esp_end;
    esp_entry.attributes = 0x0000000000000001; /* Required for ESP */
    let esp_name = b"E\0F\0I\0 \0S\0y\0s\0t\0e\0m\0 \0P\0a\0r\0t\0i\0t\0i\0o\0n\0\0\0";
    let mut ni = 0;
    while ni < esp_name.len() && ni < 72 {
        partition_entries[0 * 128 + 56 + ni] = esp_name[ni];
        ni += 1;
    }
    /* Copy ESP entry fields */
    let esp_entry_bytes = unsafe {
        core::slice::from_raw_parts(
            &esp_entry as *const GptPartitionEntry as *const u8,
            core::mem::size_of::<GptPartitionEntry>(),
        )
    };
    /* Write partition type GUID, unique GUID, start, end, attributes */
    partition_entries[0 * 128..0 * 128 + 16].copy_from_slice(&esp_entry.partition_type_guid);
    partition_entries[0 * 128 + 16..0 * 128 + 32].copy_from_slice(&esp_entry.unique_partition_guid);
    let start_bytes = esp_start.to_le_bytes();
    let end_bytes = esp_end.to_le_bytes();
    let attr_bytes = esp_entry.attributes.to_le_bytes();
    partition_entries[0 * 128 + 32..0 * 128 + 40].copy_from_slice(&start_bytes);
    partition_entries[0 * 128 + 40..0 * 128 + 48].copy_from_slice(&end_bytes);
    partition_entries[0 * 128 + 48..0 * 128 + 56].copy_from_slice(&attr_bytes);

    /* Entry 1: LumFS */
    let mut lumfs_entry: GptPartitionEntry = unsafe { core::mem::zeroed() };
    lumfs_entry.partition_type_guid = LUMIEOS_PARTITION_GUID;
    lumfs_entry.unique_partition_guid = lumfs_guid;
    lumfs_entry.starting_lba = lumfs_start;
    lumfs_entry.ending_lba = lumfs_end;
    lumfs_entry.attributes = 0;
    let lumfs_name = b"L\0u\0m\0i\0e\0O\0S\0\0\0";
    let mut ni2 = 0;
    while ni2 < lumfs_name.len() && ni2 < 72 {
        partition_entries[1 * 128 + 56 + ni2] = lumfs_name[ni2];
        ni2 += 1;
    }
    partition_entries[1 * 128..1 * 128 + 16].copy_from_slice(&lumfs_entry.partition_type_guid);
    partition_entries[1 * 128 + 16..1 * 128 + 32].copy_from_slice(&lumfs_entry.unique_partition_guid);
    let start_bytes2 = lumfs_start.to_le_bytes();
    let end_bytes2 = lumfs_end.to_le_bytes();
    partition_entries[1 * 128 + 32..1 * 128 + 40].copy_from_slice(&start_bytes2);
    partition_entries[1 * 128 + 40..1 * 128 + 48].copy_from_slice(&end_bytes2);
    partition_entries[1 * 128 + 48..1 * 128 + 56].copy_from_slice(&[0u8; 8]);

    let entries_crc = crc32(partition_entries.as_ptr(), (128 * 128) as u64);

    if let Some(cb) = progress {
        unsafe { cb(b"Writing protective MBR...\\0" as *const u8, 10); }
    }
    let mut mbr: [u8; 512] = [0u8; 512];
    make_protective_mbr(&mut mbr, total_sectors);
    if write_sectors(bio, 0, 1, mbr.as_ptr()) != 0 {
        if let Some(cb) = progress {
            unsafe { cb(b"ERROR: Failed to write MBR.\\0" as *const u8, 0); }
        }
        return None;
    }

    if let Some(cb) = progress {
        unsafe { cb(b"Writing GPT header...\\0" as *const u8, 40); }
    }
    let mut header: GptHeader = unsafe { core::mem::zeroed() };
    make_gpt_header(&mut header, total_sectors, GPT_FIRST_USABLE_LBA, last_usable_lba, &disk_guid, entries_crc);
    header.crc32 = crc32(&header as *const GptHeader as *const u8, GPT_HEADER_SIZE as u64);
    let mut header_sector: [u8; 512] = [0u8; 512];
    unsafe {
        ptr::copy_nonoverlapping(
            &header as *const GptHeader as *const u8,
            header_sector.as_mut_ptr(),
            core::mem::size_of::<GptHeader>() as usize,
        );
    }
    if write_sectors(bio, 1, 1, header_sector.as_ptr()) != 0 {
        if let Some(cb) = progress {
            unsafe { cb(b"ERROR: Failed to write GPT header.\\0" as *const u8, 0); }
        }
        return None;
    }

    if let Some(cb) = progress {
        unsafe { cb(b"Writing partition entries...\\0" as *const u8, 70); }
    }
    let entries_bytes: usize = 128 * 128;
    let entries_sectors = (entries_bytes + sector_size as usize - 1) / sector_size as usize;
    if write_sectors(bio, GPT_PARTITIONS_START_LBA, entries_sectors as u64, partition_entries.as_ptr()) != 0 {
        if let Some(cb) = progress {
            unsafe { cb(b"ERROR: Failed to write partition entries.\\0" as *const u8, 0); }
        }
        return None;
    }

    flush_sectors(bio);

    if let Some(cb) = progress {
        unsafe { cb(b"GPT dual partition table created.\\0" as *const u8, 100); }
    }

    Some(DualPartitionResult {
        esp_start,
        esp_sectors,
        lumfs_start,
        lumfs_sectors,
    })
}

pub fn create_gpt_partition(
    device_handle: efi_handle,
    size_gb: u64,
    is_esp: bool,
    progress: GptProgressCb,
) -> Option<(u64, u64)> {
    let bio = match get_block_io(device_handle) { Some(b) => b, None => return None };
    let total_sectors = disk_total_sectors(device_handle);
    let sector_size = disk_sector_size(device_handle);
    if total_sectors < GPT_ALIGNMENT + 100 { return None; }

    let last_usable_lba = total_sectors - GPT_ALIGNMENT - 33 - 1;

    let requested_sectors = (size_gb * 1024 * 1024 * 1024) / sector_size;
    let part_sectors = if requested_sectors > last_usable_lba - GPT_ALIGNMENT + 1 {
        last_usable_lba - GPT_ALIGNMENT + 1
    } else {
        requested_sectors
    };
    let part_sectors = (part_sectors / GPT_ALIGNMENT) * GPT_ALIGNMENT;
    if part_sectors < GPT_ALIGNMENT { return None; }

    let part_start = GPT_ALIGNMENT;
    let part_end = part_start + part_sectors - 1;

    let disk_guid: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
    ];

    let part_guid: [u8; 16] = [
        0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88,
        0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00,
    ];

    let mut part_entry: GptPartitionEntry = unsafe { core::mem::zeroed() };
    make_partition_entry(&mut part_entry, part_start, part_end, &part_guid, is_esp);

    let mut partition_entries: [u8; 128 * 128] = [0u8; 128 * 128];
    let pe_slice = &mut partition_entries[..core::mem::size_of::<GptPartitionEntry>()];
    unsafe {
        ptr::copy_nonoverlapping(
            &part_entry as *const GptPartitionEntry as *const u8,
            pe_slice.as_mut_ptr(),
            core::mem::size_of::<GptPartitionEntry>(),
        );
    }
    let entries_crc = crc32(partition_entries.as_ptr(), (128 * 128) as u64);

    if let Some(cb) = progress {
        unsafe { cb(b"Writing protective MBR...\0" as *const u8, 10); }
    }
    let mut mbr: [u8; 512] = [0u8; 512];
    make_protective_mbr(&mut mbr, total_sectors);
    if write_sectors(bio, 0, 1, mbr.as_ptr()) != 0 {
        if let Some(cb) = progress {
            unsafe { cb(b"ERROR: Failed to write MBR.\0" as *const u8, 0); }
        }
        return None;
    }

    if let Some(cb) = progress {
        unsafe { cb(b"Writing GPT header...\0" as *const u8, 40); }
    }
    let mut header: GptHeader = unsafe { core::mem::zeroed() };
    make_gpt_header(&mut header, total_sectors, GPT_FIRST_USABLE_LBA, last_usable_lba, &disk_guid, entries_crc);
    header.crc32 = crc32(&header as *const GptHeader as *const u8, GPT_HEADER_SIZE as u64);
    let mut header_sector: [u8; 512] = [0u8; 512];
    unsafe {
        ptr::copy_nonoverlapping(
            &header as *const GptHeader as *const u8,
            header_sector.as_mut_ptr(),
            core::mem::size_of::<GptHeader>() as usize,
        );
    }
    if write_sectors(bio, 1, 1, header_sector.as_ptr()) != 0 {
        if let Some(cb) = progress {
            unsafe { cb(b"ERROR: Failed to write GPT header.\0" as *const u8, 0); }
        }
        return None;
    }

    if let Some(cb) = progress {
        unsafe { cb(b"Writing partition entries...\0" as *const u8, 70); }
    }
    let entries_bytes: usize = 128 * 128;
    let entries_sectors = (entries_bytes + sector_size as usize - 1) / sector_size as usize;
    if write_sectors(bio, GPT_PARTITIONS_START_LBA, entries_sectors as u64, partition_entries.as_ptr()) != 0 {
        if let Some(cb) = progress {
            unsafe { cb(b"ERROR: Failed to write partition entries.\0" as *const u8, 0); }
        }
        return None;
    }

    flush_sectors(bio);

    if let Some(cb) = progress {
        unsafe { cb(b"GPT partition table created.\0" as *const u8, 100); }
    }

    Some((part_start, part_sectors))
}
