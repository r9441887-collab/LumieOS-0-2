#![no_std]
#![no_main]

extern crate lumie_std;
extern crate lumieos_loader;

use lumieos_loader::uefi::{efi_handle, EfiSystemTable};

#[no_mangle]
pub extern "efiapi" fn efi_main(
    image_handle: efi_handle,
    system_table: *mut EfiSystemTable,
) -> usize {
    lumieos_loader::lumie_loader_start(image_handle, system_table);
    0
}
