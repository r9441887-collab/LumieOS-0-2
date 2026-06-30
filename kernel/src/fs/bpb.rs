#![no_std]

pub const FAT_ATTR_READ_ONLY: u8 = 0x01;
pub const FAT_ATTR_HIDDEN: u8 = 0x02;
pub const FAT_ATTR_SYSTEM: u8 = 0x04;
pub const FAT_ATTR_VOLUME_ID: u8 = 0x08;
pub const FAT_ATTR_DIRECTORY: u8 = 0x10;
pub const FAT_ATTR_ARCHIVE: u8 = 0x20;
pub const FAT_ATTR_LFN: u8 = 0x0F;

pub const FAT_END_OF_CHAIN: u32 = 0x0FFFFFF8;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct FatBpb {
    pub jmp: [u8; 3],
    pub oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub num_fats: u8,
    pub root_entries: u16,
    pub total_sectors_16: u16,
    pub media_descriptor: u8,
    pub sectors_per_fat_16: u16,
    pub sectors_per_track: u16,
    pub num_heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    pub sectors_per_fat_32: u32,
    pub ext_flags: u16,
    pub fs_version: u16,
    pub root_cluster: u32,
    pub fs_info: u16,
    pub backup_boot_sector: u16,
    pub reserved: [u8; 12],
    pub drive_number: u8,
    pub reserved1: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fs_type: [u8; 8],
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct FatDirEnt {
    pub name: [u8; 11],
    pub attr: u8,
    pub nt_reserved: u8,
    pub tenths: u8,
    pub time_created: u16,
    pub date_created: u16,
    pub date_accessed: u16,
    pub cluster_high: u16,
    pub time_modified: u16,
    pub date_modified: u16,
    pub cluster_low: u16,
    pub size: u32,
}
