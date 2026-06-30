#![no_std]

use crate::uefi::types::*;
use crate::uefi::guid::EfiGuid;
use crate::uefi::memory::EfiMemoryDescriptor;
use crate::uefi::time::{EfiTime, EfiTimeCapabilities};
use crate::uefi::protocols::input::EfiSimpleTextInputProtocol;
use crate::uefi::protocols::output::EfiSimpleTextOutputProtocol;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EfiTableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct EfiSystemTable {
    pub hdr: EfiTableHeader,
    pub firmware_vendor: *mut char16,
    pub firmware_revision: u32,
    pub console_in_handle: efi_handle,
    pub con_in: *mut EfiSimpleTextInputProtocol,
    pub console_out_handle: efi_handle,
    pub con_out: *mut EfiSimpleTextOutputProtocol,
    pub standard_error_handle: efi_handle,
    pub std_err: *mut EfiSimpleTextOutputProtocol,
    pub runtime_services: *mut EfiRuntimeServices,
    pub boot_services: *mut EfiBootServices,
    pub number_of_table_entries: u64,
    pub configuration_table: *mut core::ffi::c_void,
}

pub type EfiBsRaiseTpl = Option<unsafe extern "efiapi" fn(u64) -> u64>;
pub type EfiBsRestoreTpl = Option<unsafe extern "efiapi" fn(u64)>;
pub type EfiBsAllocatePages = Option<unsafe extern "efiapi" fn(u32, u32, u64, *mut efi_handle) -> efi_status>;
pub type EfiBsFreePages = Option<unsafe extern "efiapi" fn(efi_handle, u64) -> efi_status>;
pub type EfiBsGetMemoryMap = Option<unsafe extern "efiapi" fn(*mut u64, *mut EfiMemoryDescriptor, *mut u64, *mut u64, *mut u32) -> efi_status>;
pub type EfiBsAllocatePool = Option<unsafe extern "efiapi" fn(u32, u64, *mut *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsFreePool = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void) -> efi_status>;
pub type EfiBsCreateEvent = Option<unsafe extern "efiapi" fn(u32, u64, *mut core::ffi::c_void, *mut core::ffi::c_void, *mut efi_event) -> efi_status>;
pub type EfiBsSetTimer = Option<unsafe extern "efiapi" fn(efi_event, u64, u64) -> efi_status>;
pub type EfiBsWaitForEvent = Option<unsafe extern "efiapi" fn(u64, *mut efi_event, *mut u64) -> efi_status>;
pub type EfiBsSignalEvent = Option<unsafe extern "efiapi" fn(efi_event) -> efi_status>;
pub type EfiBsCloseEvent = Option<unsafe extern "efiapi" fn(efi_event) -> efi_status>;
pub type EfiBsCheckEvent = Option<unsafe extern "efiapi" fn(efi_event) -> efi_status>;
pub type EfiBsInstallProtocolInterface = Option<unsafe extern "efiapi" fn(*mut efi_handle, *const EfiGuid, u32, *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsReinstallProtocolInterface = Option<unsafe extern "efiapi" fn(efi_handle, *const EfiGuid, *mut core::ffi::c_void, *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsUninstallProtocolInterface = Option<unsafe extern "efiapi" fn(efi_handle, *const EfiGuid, *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsHandleProtocol = Option<unsafe extern "efiapi" fn(efi_handle, *const EfiGuid, *mut *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsRegisterProtocolNotify = Option<unsafe extern "efiapi" fn(*const EfiGuid, efi_event, *mut *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsLocateHandle = Option<unsafe extern "efiapi" fn(u32, *const EfiGuid, *mut core::ffi::c_void, *mut u64, *mut efi_handle) -> efi_status>;
pub type EfiBsLocateDevicePath = Option<unsafe extern "efiapi" fn(*const EfiGuid, *mut *mut core::ffi::c_void, *mut efi_handle) -> efi_status>;
pub type EfiBsInstallConfigurationTable = Option<unsafe extern "efiapi" fn(*const EfiGuid, *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsLoadImage = Option<unsafe extern "efiapi" fn(u8, efi_handle, *mut core::ffi::c_void, *mut core::ffi::c_void, u64, *mut efi_handle) -> efi_status>;
pub type EfiBsStartImage = Option<unsafe extern "efiapi" fn(efi_handle, *mut u64, *mut *mut char16) -> efi_status>;
pub type EfiBsExit = Option<unsafe extern "efiapi" fn(efi_handle, efi_status, u64, *mut char16) -> efi_status>;
pub type EfiBsUnloadImage = Option<unsafe extern "efiapi" fn(efi_handle) -> efi_status>;
pub type EfiBsExitBootServices = Option<unsafe extern "efiapi" fn(efi_handle, u64) -> efi_status>;
pub type EfiBsGetNextMonotonicCount = Option<unsafe extern "efiapi" fn(*mut u64) -> efi_status>;
pub type EfiBsStall = Option<unsafe extern "efiapi" fn(u64) -> efi_status>;
pub type EfiBsSetWatchdogTimer = Option<unsafe extern "efiapi" fn(u64, u64, u64, *mut char16) -> efi_status>;
pub type EfiBsConnectController = Option<unsafe extern "efiapi" fn(efi_handle, *mut efi_handle, *mut core::ffi::c_void, u8) -> efi_status>;
pub type EfiBsDisconnectController = Option<unsafe extern "efiapi" fn(efi_handle, efi_handle, efi_handle) -> efi_status>;
pub type EfiBsOpenProtocol = Option<unsafe extern "efiapi" fn(efi_handle, *const EfiGuid, *mut *mut core::ffi::c_void, efi_handle, efi_handle, u32) -> efi_status>;
pub type EfiBsCloseProtocol = Option<unsafe extern "efiapi" fn(efi_handle, *const EfiGuid, efi_handle, efi_handle) -> efi_status>;
pub type EfiBsOpenProtocolInformation = Option<unsafe extern "efiapi" fn(efi_handle, *const EfiGuid, *mut *mut core::ffi::c_void, *mut u64) -> efi_status>;
pub type EfiBsProtocolsPerHandle = Option<unsafe extern "efiapi" fn(efi_handle, *mut *mut *mut EfiGuid, *mut u64) -> efi_status>;
pub type EfiBsLocateHandleBuffer = Option<unsafe extern "efiapi" fn(u32, *const EfiGuid, *mut core::ffi::c_void, *mut u64, *mut *mut efi_handle) -> efi_status>;
pub type EfiBsLocateProtocol = Option<unsafe extern "efiapi" fn(*const EfiGuid, *mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> efi_status>;
pub type EfiBsInstallMultipleProtocolInterfaces = Option<unsafe extern "efiapi" fn(*mut efi_handle, ...) -> efi_status>;
pub type EfiBsUninstallMultipleProtocolInterfaces = Option<unsafe extern "efiapi" fn(efi_handle, ...) -> efi_status>;
pub type EfiBsCalculateCrc32 = Option<unsafe extern "efiapi" fn(*const core::ffi::c_void, u64, *mut u32) -> efi_status>;
pub type EfiBsCopyMem = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, *const core::ffi::c_void, u64)>;
pub type EfiBsSetMem = Option<unsafe extern "efiapi" fn(*mut core::ffi::c_void, u64, u8)>;
pub type EfiBsCreateEventEx = Option<unsafe extern "efiapi" fn(u32, u64, *mut core::ffi::c_void, *mut core::ffi::c_void, *mut efi_event) -> efi_status>;

#[repr(C)]
pub struct EfiBootServices {
    pub hdr: EfiTableHeader,
    pub raise_tpl: EfiBsRaiseTpl,
    pub restore_tpl: EfiBsRestoreTpl,
    pub allocate_pages: EfiBsAllocatePages,
    pub free_pages: EfiBsFreePages,
    pub get_memory_map: EfiBsGetMemoryMap,
    pub allocate_pool: EfiBsAllocatePool,
    pub free_pool: EfiBsFreePool,
    pub create_event: EfiBsCreateEvent,
    pub set_timer: EfiBsSetTimer,
    pub wait_for_event: EfiBsWaitForEvent,
    pub signal_event: EfiBsSignalEvent,
    pub close_event: EfiBsCloseEvent,
    pub check_event: EfiBsCheckEvent,
    pub install_protocol_interface: EfiBsInstallProtocolInterface,
    pub reinstall_protocol_interface: EfiBsReinstallProtocolInterface,
    pub uninstall_protocol_interface: EfiBsUninstallProtocolInterface,
    pub handle_protocol: EfiBsHandleProtocol,
    pub reserved: *mut core::ffi::c_void,
    pub register_protocol_notify: EfiBsRegisterProtocolNotify,
    pub locate_handle: EfiBsLocateHandle,
    pub locate_device_path: EfiBsLocateDevicePath,
    pub install_configuration_table: EfiBsInstallConfigurationTable,
    pub load_image: EfiBsLoadImage,
    pub start_image: EfiBsStartImage,
    pub exit: EfiBsExit,
    pub unload_image: EfiBsUnloadImage,
    pub exit_boot_services: EfiBsExitBootServices,
    pub get_next_monotonic_count: EfiBsGetNextMonotonicCount,
    pub stall: EfiBsStall,
    pub set_watchdog_timer: EfiBsSetWatchdogTimer,
    pub connect_controller: EfiBsConnectController,
    pub disconnect_controller: EfiBsDisconnectController,
    pub open_protocol: EfiBsOpenProtocol,
    pub close_protocol: EfiBsCloseProtocol,
    pub open_protocol_information: EfiBsOpenProtocolInformation,
    pub protocols_per_handle: EfiBsProtocolsPerHandle,
    pub locate_handle_buffer: EfiBsLocateHandleBuffer,
    pub locate_protocol: EfiBsLocateProtocol,
    pub install_multiple_protocol_interfaces: EfiBsInstallMultipleProtocolInterfaces,
    pub uninstall_multiple_protocol_interfaces: EfiBsUninstallMultipleProtocolInterfaces,
    pub calculate_crc32: EfiBsCalculateCrc32,
    pub copy_mem: EfiBsCopyMem,
    pub set_mem: EfiBsSetMem,
    pub create_event_ex: EfiBsCreateEventEx,
}

pub type EfiRtGetTime = Option<unsafe extern "efiapi" fn(*mut EfiTime, *mut EfiTimeCapabilities) -> efi_status>;
pub type EfiRtSetTime = Option<unsafe extern "efiapi" fn(*mut EfiTime) -> efi_status>;
pub type EfiRtGetVariable = Option<unsafe extern "efiapi" fn(*mut char16, *const EfiGuid, *mut u32, *mut u64, *mut core::ffi::c_void) -> efi_status>;
pub type EfiRtSetVariable = Option<unsafe extern "efiapi" fn(*mut char16, *const EfiGuid, u32, u64, *mut core::ffi::c_void) -> efi_status>;
pub type EfiRtGetNextVariableName = Option<unsafe extern "efiapi" fn(*mut u64, *mut char16, *mut EfiGuid) -> efi_status>;
pub type EfiRtResetSystem = Option<unsafe extern "efiapi" fn(EfiResetType, efi_status, u64, *mut core::ffi::c_void)>;

#[repr(C)]
pub struct EfiRuntimeServices {
    pub hdr: EfiTableHeader,
    pub get_time: EfiRtGetTime,
    pub set_time: EfiRtSetTime,
    pub get_wakeup_time: *mut core::ffi::c_void,
    pub set_wakeup_time: *mut core::ffi::c_void,
    pub set_virtual_address_map: *mut core::ffi::c_void,
    pub convert_pointer: *mut core::ffi::c_void,
    pub get_variable: EfiRtGetVariable,
    pub get_next_variable_name: EfiRtGetNextVariableName,
    pub set_variable: EfiRtSetVariable,
    pub get_next_high_monotonic_count: *mut core::ffi::c_void,
    pub reset_system: EfiRtResetSystem,
    pub update_capsule: *mut core::ffi::c_void,
    pub query_capsule_capabilities: *mut core::ffi::c_void,
    pub query_variable_info: *mut core::ffi::c_void,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EfiResetType {
    EfiResetCold = 0,
    EfiResetWarm = 1,
    EfiResetShutdown = 2,
}

pub const EFI_LOCATE_BY_PROTOCOL: u32 = 2;
