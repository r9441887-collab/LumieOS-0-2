#![no_std]

pub type u8 = core::primitive::u8;
pub type u16 = core::primitive::u16;
pub type u32 = core::primitive::u32;
pub type u64 = core::primitive::u64;
pub type i8 = core::primitive::i8;
pub type i16 = core::primitive::i16;
pub type i64 = core::primitive::i64;
pub type usize = core::primitive::u64;
pub type uintn = core::primitive::u64;
pub type efi_status = core::primitive::u64;
pub type efi_handle = *mut core::ffi::c_void;
pub type efi_event = *mut core::ffi::c_void;
pub type char16 = core::primitive::u16;
pub type boolean = core::primitive::u8;

pub const EFI_SUCCESS: efi_status = 0;

#[inline]
pub const fn efi_error(s: efi_status) -> bool {
    (s as i64) < 0
}

#[inline]
pub const fn efi_err(x: efi_status) -> efi_status {
    x | (1u64 << 63)
}

pub const EFI_INVALID_PARAMETER: efi_status = efi_err(2);
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
    pub runtime_services: *mut core::ffi::c_void,
    pub boot_services: *mut EfiBootServices,
    pub number_of_table_entries: u64,
    pub configuration_table: *mut core::ffi::c_void,
}

pub struct EfiLoadedImageProtocol;

pub const EFI_LOADED_IMAGE_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x5B1B4A42,
    data2: 0x46AE,
    data3: 0x4D47,
    data4: [0xA4, 0xCD, 0xC4, 0x0E, 0x86, 0xD7, 0xFB, 0x87],
};

pub const EFI_DEVICE_PATH_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x09576e91,
    data2: 0x6d3f,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

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

pub const EFI_GLOBAL_VARIABLE_GUID: EfiGuid = EfiGuid {
    data1: 0x8BE4DF61,
    data2: 0x93CA,
    data3: 0x11d2,
    data4: [0xAA, 0x0D, 0x00, 0xE0, 0x98, 0x03, 0x2B, 0x8C],
};

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
