#![allow(non_camel_case_types)]
#[allow(dead_code)]
pub type uintn = usize;
pub type efi_status = core::primitive::u64;
pub type efi_handle = *mut core::ffi::c_void;
pub type efi_event = *mut core::ffi::c_void;
pub type char16 = core::primitive::u16;
pub type boolean = core::primitive::u8;

#[allow(dead_code)]
pub const EFI_SUCCESS: efi_status = 0;

#[inline]
#[allow(dead_code)]
pub const fn efi_error(s: efi_status) -> bool {
    (s as i64) < 0
}

#[inline]
#[allow(dead_code)]
pub const fn efi_err(x: efi_status) -> efi_status {
    x | (1u64 << 63)
}

#[allow(dead_code)]
pub const EFI_INVALID_PARAMETER: efi_status = efi_err(2);
#[allow(dead_code)]
pub const EFI_NOT_FOUND: efi_status = efi_err(14);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiGuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiTableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

pub type EfiOutputString = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *mut char16) -> efi_status>;
pub type EfiClearScreen = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void) -> efi_status>;
pub type EfiEnableCursor = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, boolean) -> efi_status>;

#[repr(C)]
pub struct EfiSimpleTextOutputProtocol {
    pub reset: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, boolean)>,
    pub output_string: EfiOutputString,
    pub test_string: *mut core::ffi::c_void,
    pub set_attribute: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u64) -> efi_status>,
    pub clear_screen: EfiClearScreen,
    pub set_cursor_position: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u64, u64) -> efi_status>,
    pub enable_cursor: EfiEnableCursor,
    pub mode: *mut core::ffi::c_void,
}

pub type EfiBsHandleProtocol = Option<unsafe extern "efiapi" fn(efi_handle, *const EfiGuid, *mut *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsAllocatePool = Option<unsafe extern "efiapi" fn(u32, u64, *mut *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsFreePool = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void) -> efi_status>;
pub type EfiBsLocateProtocol = Option<unsafe extern "efiapi" fn(*const EfiGuid, *mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsLocateHandleBuffer = Option<unsafe extern "efiapi" fn(u32, *const EfiGuid, *mut core::ffi::c_void, *mut u64, *mut *mut efi_handle) -> efi_status>;
pub type EfiBsExiBootServices = Option<unsafe extern "efiapi" fn(efi_handle, u64) -> efi_status>;

#[repr(C)]
pub struct EfiBootServices {
    pub hdr: EfiTableHeader,
    pub raise_tpl: *mut core::ffi::c_void,
    pub restore_tpl: *mut core::ffi::c_void,
    pub allocate_pages: *mut core::ffi::c_void,
    pub free_pages: *mut core::ffi::c_void,
    pub get_memory_map: *mut core::ffi::c_void,
    pub allocate_pool: EfiBsAllocatePool,
    pub free_pool: EfiBsFreePool,
    pub create_event: *mut core::ffi::c_void,
    pub set_timer: *mut core::ffi::c_void,
    pub wait_for_event: *mut core::ffi::c_void,
    pub signal_event: *mut core::ffi::c_void,
    pub close_event: *mut core::ffi::c_void,
    pub check_event: *mut core::ffi::c_void,
    pub install_protocol_interface: *mut core::ffi::c_void,
    pub reinstall_protocol_interface: *mut core::ffi::c_void,
    pub uninstall_protocol_interface: *mut core::ffi::c_void,
    pub handle_protocol: EfiBsHandleProtocol,
    pub reserved: *mut core::ffi::c_void,
    pub register_protocol_notify: *mut core::ffi::c_void,
    pub locate_handle: *mut core::ffi::c_void,
    pub locate_device_path: *mut core::ffi::c_void,
    pub install_configuration_table: *mut core::ffi::c_void,
    pub load_image: *mut core::ffi::c_void,
    pub start_image: *mut core::ffi::c_void,
    pub exit: *mut core::ffi::c_void,
    pub unload_image: *mut core::ffi::c_void,
    pub exit_boot_services: EfiBsExiBootServices,
    pub get_next_monotonic_count: *mut core::ffi::c_void,
    pub stall: *mut core::ffi::c_void,
    pub set_watchdog_timer: *mut core::ffi::c_void,
    pub connect_controller: *mut core::ffi::c_void,
    pub disconnect_controller: *mut core::ffi::c_void,
    pub open_protocol: *mut core::ffi::c_void,
    pub close_protocol: *mut core::ffi::c_void,
    pub open_protocol_information: *mut core::ffi::c_void,
    pub protocols_per_handle: *mut core::ffi::c_void,
    pub locate_handle_buffer: EfiBsLocateHandleBuffer,
    pub locate_protocol: EfiBsLocateProtocol,
    pub install_multiple_protocol_interfaces: *mut core::ffi::c_void,
    pub uninstall_multiple_protocol_interfaces: *mut core::ffi::c_void,
    pub calculate_crc32: *mut core::ffi::c_void,
    pub copy_mem: *mut core::ffi::c_void,
    pub set_mem: *mut core::ffi::c_void,
    pub create_event_ex: *mut core::ffi::c_void,
}

#[repr(C)]
pub struct EfiSystemTable {
    pub hdr: EfiTableHeader,
    pub firmware_vendor: *mut char16,
    pub firmware_revision: u32,
    pub console_in_handle: efi_handle,
    pub con_in: *mut core::ffi::c_void,
    pub console_out_handle: efi_handle,
    pub con_out: *mut EfiSimpleTextOutputProtocol,
    pub standard_error_handle: efi_handle,
    pub std_err: *mut core::ffi::c_void,
    pub runtime_services: *mut EfiRuntimeServices,
    pub boot_services: *mut EfiBootServices,
    pub number_of_table_entries: u64,
    pub configuration_table: *mut core::ffi::c_void,
}

#[allow(dead_code)]
pub struct EfiLoadedImageProtocol;

pub const EFI_LOADED_IMAGE_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x5B1B4A42,
    data2: 0x46AE,
    data3: 0x4D47,
    data4: [0xA4, 0xCD, 0xC4, 0x0E, 0x86, 0xD7, 0xFB, 0x87],
};

#[allow(dead_code)]
pub const EFI_DEVICE_PATH_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x09576e91,
    data2: 0x6d3f,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

#[allow(dead_code)]
pub const EFI_SIMPLE_FILE_SYSTEM_GUID: EfiGuid = EfiGuid {
    data1: 0x964e5b22,
    data2: 0x6459,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

pub const EFI_BLOCK_IO_GUID: EfiGuid = EfiGuid {
    data1: 0x964e5b21,
    data2: 0x6459,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

#[allow(dead_code)]
pub const EFI_GLOBAL_VARIABLE_GUID: EfiGuid = EfiGuid {
    data1: 0x8BE4DF61,
    data2: 0x93CA,
    data3: 0x11d2,
    data4: [0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C],
};

#[allow(dead_code)]
pub const EFI_SIMPLE_TEXT_INPUT_GUID: EfiGuid = EfiGuid {
    data1: 0x387477c1,
    data2: 0x69c7,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

pub const EFI_SIMPLE_POINTER_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x31878c87,
    data2: 0x0b75,
    data3: 0x11d5,
    data4: [0x9a, 0x4f, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
};

pub const EFI_ABSOLUTE_POINTER_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x8D59D32B,
    data2: 0xC655,
    data3: 0x4AE9,
    data4: [0x9B, 0x15, 0xF2, 0x59, 0x04, 0x99, 0x2A, 0x43],
};

pub const EFI_BOOT_SERVICES_DATA: u32 = 4;

/* Block I/O Protocol */
#[repr(C)]
pub struct EfiBlockIoProtocol {
    pub revision: u64,
    pub media: *mut EfiBlockIoMedia,
    pub reset: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, boolean) -> efi_status>,
    pub read_blocks: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u32, u64, u64, *mut core::ffi::c_void) -> efi_status>,
    pub write_blocks: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u32, u64, u64, *mut core::ffi::c_void) -> efi_status>,
    pub flush_blocks: Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void) -> efi_status>,
}

#[repr(C, packed)]
pub struct EfiBlockIoMedia {
    pub media_id: u32,
    pub removable_media: boolean,
    pub media_present: boolean,
    pub logical_partition: boolean,
    pub read_only: boolean,
    pub write_caching: boolean,
    pub pad: [u8; 3],
    pub block_size: u32,
    pub io_align: u32,
    pub last_block: u64,
    pub lowest_aligned_lba: u64,
    pub logical_blocks_per_physical_block: u32,
    pub optimal_transfer_length_granularity: u32,
}

/* GOP Protocol */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiPixelBitmask {
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub reserved_mask: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiGopModeInfo {
    pub version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: u32,
    pub pixel_information: EfiPixelBitmask,
    pub pixels_per_scan_line: u32,
}

#[repr(C)]
pub struct EfiGopMode {
    pub max_mode: u32,
    pub mode: u32,
    pub info: *mut EfiGopModeInfo,
    pub size_of_info: u64,
    pub frame_buffer_base: u64,
    pub frame_buffer_size: u64,
}

pub type EfiGopQueryMode = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u32, *mut u64, *mut *mut EfiGopModeInfo) -> efi_status>;
pub type EfiGopSetMode = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u32) -> efi_status>;
pub type EfiGopBlt = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, u32, u32, u32, u32, u32, u32, u32, u32) -> efi_status>;

#[repr(C)]
pub struct EfiGopProtocol {
    pub query_mode: EfiGopQueryMode,
    pub set_mode: EfiGopSetMode,
    pub blt: EfiGopBlt,
    pub mode: *mut EfiGopMode,
}

pub const EFI_GOP_GUID: EfiGuid = EfiGuid {
    data1: 0x9042a9de,
    data2: 0x23dc,
    data3: 0x4a38,
    data4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

/* Runtime Services */
pub type EfiRtGetVariable = Option<unsafe extern "efiapi" fn(*mut char16, *const EfiGuid, *mut u32, *mut u64, *mut core::ffi::c_void) -> efi_status>;
pub type EfiRtSetVariable = Option<unsafe extern "efiapi" fn(*mut char16, *const EfiGuid, u32, u64, *mut core::ffi::c_void) -> efi_status>;
pub type EfiRtResetSystem = Option<unsafe extern "efiapi" fn(u32, efi_status, u64, *mut core::ffi::c_void)>;

#[repr(C)]
pub struct EfiRuntimeServices {
    pub hdr: EfiTableHeader,
    pub get_time: *mut core::ffi::c_void,
    pub set_time: *mut core::ffi::c_void,
    pub get_wakeup_time: *mut core::ffi::c_void,
    pub set_wakeup_time: *mut core::ffi::c_void,
    pub set_virtual_address_map: *mut core::ffi::c_void,
    pub convert_pointer: *mut core::ffi::c_void,
    pub get_variable: EfiRtGetVariable,
    pub get_next_variable_name: *mut core::ffi::c_void,
    pub set_variable: EfiRtSetVariable,
    pub get_next_high_monotonic_count: *mut core::ffi::c_void,
    pub reset_system: EfiRtResetSystem,
    pub update_capsule: *mut core::ffi::c_void,
    pub query_capsule_capabilities: *mut core::ffi::c_void,
    pub query_variable_info: *mut core::ffi::c_void,
}
