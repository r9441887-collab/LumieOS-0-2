
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiMemoryDescriptor {
    pub type_: u32,
    pub padding: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

pub const EFI_MEMORY_DESCRIPTOR_VERSION: u32 = 1;
pub const EFI_BOOT_SERVICES_DATA: u32 = 4;
