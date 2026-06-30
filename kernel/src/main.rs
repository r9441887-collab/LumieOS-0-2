#![no_std]
#![feature(abi_efiapi)]
#![feature(c_variadic)]
#![feature(naked_functions)]

extern crate lumie_std;

pub mod uefi;
pub mod api;
pub mod globals;
pub mod console;
pub mod drivers;
pub mod fs;
pub mod mm;
pub mod arch;
pub mod sched;
pub mod system;

use core::ffi::c_void;
use core::ptr;
use lumie_std::types::*;
use lumie_std::LumieColor;
use crate::uefi::types::*;
use crate::uefi::tables::*;
use crate::uefi::time::*;
use crate::uefi::guid::*;
use crate::uefi::memory::*;
use crate::uefi::protocols::loaded_image::*;
use crate::uefi::protocols::device_path::*;
use crate::uefi::protocols::file_system::*;
use crate::uefi::protocols::variable::*;
use crate::uefi::protocols::block_io::*;
use crate::uefi::protocols::input::EfiInputKey;
use crate::console::FbInfo;

const BOOT_ORDER_NAME: [u16; 10] = [0x0042, 0x006F, 0x006F, 0x0074, 0x004F, 0x0072, 0x0064, 0x0065, 0x0072, 0x0000];

#[repr(C)]
struct ModuleHeader {
    _opaque: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct module_t {
    hdr: *const ModuleHeader,
    base: *mut c_void,
    size: u32,
    module_api: *mut c_void,
    loaded: i32,
}

const KAPI_VERSION: u32 = 2;

unsafe fn cstr<'a>(s: *const u8) -> &'a str {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    core::str::from_utf8_unchecked(core::slice::from_raw_parts(s, len))
}

fn uefi_getchar() -> i32 {
    let st = match globals::get_st() {
        Some(s) => s,
        None => return 0,
    };
    let con_in = st.con_in;
    if con_in.is_null() {
        return 0;
    }
    unsafe {
        loop {
            let mut key: EfiInputKey = core::mem::zeroed();
            if let Some(rks) = (*con_in).read_key_stroke {
                if rks(con_in as *mut c_void, &mut key) == EFI_SUCCESS {
                    if key.unicode_char == 0x0D {
                        return b'\n' as i32;
                    }
                    if key.unicode_char >= 0x01 && key.unicode_char <= 0x7E {
                        return key.unicode_char as i32;
                    }
                    if key.scan_code >= 0x01 && key.scan_code <= 0x0B {
                        return 0xE0 + key.scan_code as i32;
                    }
                }
            }
            if let Some(bs) = globals::get_bs() {
                if let Some(stall) = bs.stall {
                    stall(1000);
                }
            }
        }
    }
}

fn uefi_kbhit() -> i32 {
    let st = match globals::get_st() {
        Some(s) => s,
        None => return 0,
    };
    let con_in = st.con_in;
    if con_in.is_null() {
        return 0;
    }
    unsafe {
        let mut key: EfiInputKey = core::mem::zeroed();
        if let Some(rks) = (*con_in).read_key_stroke {
            if rks(con_in as *mut c_void, &mut key) == EFI_SUCCESS {
                return 1;
            }
        }
    }
    0
}

fn lumie_stall(us: u64) {
    unsafe {
        if let Some(bs_ptr) = globals::get_bs() {
            if let Some(stall_fn) = (*bs_ptr).stall {
                stall_fn(us);
                return;
            }
        }
        let mut remaining = us;
        while remaining >= 1000 {
            drivers::pit::stall(1000);
            remaining -= 1000;
        }
        if remaining > 0 {
            drivers::pit::stall(remaining as u32);
        }
    }
}

fn lumie_clear(bg: u32) {
    unsafe {
        console::gop::fill_rect(0, 0, console::gop::get_width(), console::gop::get_height(), bg);
        console::term_set_pos(0, 0);
    }
}

fn lumie_set_fg(c: u32) {
    unsafe {
        console::term_set_fg(match c {
            0x000000 => LumieColor::Black,
            0x0000AA => LumieColor::Blue,
            0x00AA00 => LumieColor::Green,
            0x00AAAA => LumieColor::Cyan,
            0xAA0000 => LumieColor::Red,
            0xAA00AA => LumieColor::Magenta,
            0xAA5500 => LumieColor::Brown,
            0xAAAAAA => LumieColor::LightGray,
            0x555555 => LumieColor::DarkGray,
            0x5555FF => LumieColor::LightBlue,
            0x55FF55 => LumieColor::LightGreen,
            0x55FFFF => LumieColor::LightCyan,
            0xFF5555 => LumieColor::LightRed,
            0xFF55FF => LumieColor::LightMagenta,
            0xFFFF55 => LumieColor::Yellow,
            0xFFFFFF => LumieColor::White,
            _ => LumieColor::White,
        })
    }
}

fn lumie_set_bg(c: u32) {
    unsafe {
        console::term_set_bg(match c {
            0x000000 => LumieColor::Black,
            0x0000AA => LumieColor::Blue,
            0x555555 => LumieColor::DarkGray,
            0xAAAAAA => LumieColor::LightGray,
            0xFFFFFF => LumieColor::White,
            _ => LumieColor::Black,
        })
    }
}

fn lumie_set_pos(x: i32, y: i32) {
    unsafe { console::term_set_pos(x, y); }
}

fn lumie_putchar(c: u8) {
    unsafe { console::term_putchar(c); }
}

fn lumie_write(s: *const u8) {
    unsafe { console::term_write(cstr(s)); }
}

fn lumie_writeln(s: *const u8) {
    unsafe { console::term_writeln(cstr(s)); }
}

fn lumie_getchar() -> i32 {
    uefi_getchar()
}

fn lumie_kbhit() -> i32 {
    uefi_kbhit()
}

fn lumie_get_width() -> i32 {
    console::term_get_width()
}

fn lumie_get_height() -> i32 {
    console::term_get_height()
}

fn lumie_fs_init() -> i32 {
    unsafe { fs::fat32::init() }
}

fn lumie_fs_read(path: *const u8, buffer: *mut c_void, max_size: u32) -> i32 {
    unsafe { fs::fat32::read_file(path, buffer as *mut u8, max_size) }
}

fn lumie_fs_write(path: *const u8, data: *const c_void, size: u32) -> i32 {
    unsafe { fs::fat32::write_file(path, data as *const u8, size) }
}

fn lumie_fs_list(path: *const u8, entries: *mut c_void, max_entries: i32) -> i32 {
    unsafe { fs::fat32::list_dir(path, entries as *mut crate::fs::types::LumieDirEnt, max_entries) }
}

fn lumie_fs_exists(path: *const u8) -> i32 {
    if unsafe { fs::fat32::exists(path) } { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn lumie_load_shell_module() -> i32 {
    if crate::globals::get_shell_mod_loaded() {
        return 0;
    }
    let kapi = api::KernelApi {
        version: KAPI_VERSION,
        term_clear: None, term_set_fg: None, term_set_bg: None,
        term_set_pos: None, term_write: None, term_writeln: None,
        term_putchar: None, term_get_width: None, term_get_height: None,
        kbd_getchar: None, kbd_kbhit: None,
        kmalloc: None, kfree: None, kcalloc: None, kmemset: None, kmemcpy: None,
        fs_read: None, fs_write: None, fs_exists: None, fs_list: None, fs_mkdir: None,
        printf: None, stall: None, reboot: None, shutdown: None,
        gpu_fill_rect: None, gpu_put_pixel: None, gpu_get_pixel: None,
        gpu_is_active: None, gpu_flip: None, gpu_vsync: None,
        desktop_ctx: ptr::null_mut(),
        mod_load: None, mod_unload: None,
        mem_total: None, mem_free: None, mem_used: None,
        disk_read: None, disk_write: None, disk_count: None,
        disk_name: None, disk_sectors: None,
        pci_scan: None, pci_vendor_str: None, pci_device_str: None,
        get_time: None,
        sched_count: None, sched_name: None, sched_state: None,
        reserved: [0u64; 8],
    };

    let paths: &[*const u8] = &[
        b"\\system\\shell.lsh\0" as *const u8 as *const u8,
        b"\\drivers\\shell.lsh\0" as *const u8 as *const u8,
    ];

    for &path in paths {
        let mut mod_: module_t = unsafe { core::mem::zeroed() };
        let rc = unsafe {
            system::module::module_load(
                path,
                &kapi as *const api::KernelApi as *const c_void,
                &mut mod_,
            )
        };
        if rc == 0 {
            crate::globals::set_shell_mod_loaded(true);
            crate::globals::set_shell_module(mod_);
            return 0;
        }
    }
    -1
}

fn lumie_shell_run() {
    if crate::globals::get_shell_mod_loaded() {
        if let Some(mod_) = crate::globals::get_shell_module() {
            if !mod_.module_api.is_null() {
                let shell_entry: extern "C" fn() =
                    unsafe { core::mem::transmute(mod_.module_api) };
                shell_entry();
                return;
            }
        }
    }
}

#[no_mangle]
pub extern "efiapi" fn efi_main(
    image_handle: efi_handle,
    system_table: *mut EfiSystemTable,
) -> efi_status {
    unsafe {
        let st = &*system_table;
        globals::set_st(st);
        globals::set_image_handle(image_handle);
        if !st.boot_services.is_null() {
            globals::set_bs(&*st.boot_services);
        }
        if !st.runtime_services.is_null() {
            globals::set_rt(&*st.runtime_services);
        }
    }

    unsafe {
        let st = &*system_table;

        /* Clear screen, disable cursor */
        let con_out = st.con_out;
        if !con_out.is_null() {
            if let Some(cs) = (*con_out).clear_screen {
                cs(con_out as *mut c_void);
            }
            if let Some(ec) = (*con_out).enable_cursor {
                ec(con_out as *mut c_void, 0);
            }
        }

        /* Initialize GOP */
        console::gop::init(image_handle, &*system_table);

        /* Init subsystems */
        mm::init(st.boot_services, image_handle);
        drivers::ahci::init();
        drivers::pit::init(1000);
        drivers::pcspkr::init();

        /* Init keyboard, mouse, terminal, filesystem */
        drivers::keyboard::init(system_table);
        drivers::mouse::init(system_table);
        console::term_init();
        fs::fat32::init();
    }

    EFI_SUCCESS
}

fn lumie_panic(msg: *const u8, file: *const u8, line: i32) {
    unsafe {
        console::term_clear(LumieColor::Blue);
        console::term_set_fg(LumieColor::White);
        console::term_set_bg(LumieColor::Blue);

        let rows = console::term_get_height();

        console::term_set_pos(2, 2);
        console::term_set_fg(LumieColor::White);
        console::term_write("LumieOS");
        console::term_set_fg(LumieColor::LightCyan);
        console::term_writeln(" (Windows Edition)");

        console::term_set_fg(LumieColor::White);
        console::term_set_pos(2, 4);
        console::term_write("A fatal error has occurred.");
        console::term_set_pos(2, 5);
        console::term_write("The operating system needs to restart.");

        console::term_set_pos(2, 7);
        console::term_write("Error: ");
        console::term_writeln(cstr(msg));

        if !file.is_null() {
            console::term_set_pos(2, 9);
            console::term_write("At: ");
            console::term_write(cstr(file));
            if line > 0 {
                let mut lbuf: [u8; 32] = [0u8; 32];
                lumie_std::format::lumie_itoa(line as i64, lbuf.as_mut_ptr(), 10);
                console::term_write(":");
                console::term_write(cstr(lbuf.as_ptr()));
            }
        }

        console::term_set_pos(2, rows - 3);
        console::term_write("Press any key to restart...");

        uefi_getchar();
        console::term_clear(LumieColor::Blue);
    }

    if let Some(rt) = crate::globals::get_rt() {
        unsafe {
            if let Some(reset) = rt.reset_system {
                reset(EfiResetType::EfiResetWarm, EFI_SUCCESS, 0, ptr::null_mut());
            }
        }
    }
}

pub fn lumie_reboot() {
    unsafe { console::term_writeln("Rebooting..."); }
    lumie_stall(500000);
    if let Some(rt) = crate::globals::get_rt() {
        unsafe {
            if let Some(reset) = rt.reset_system {
                reset(EfiResetType::EfiResetWarm, EFI_SUCCESS, 0, ptr::null_mut());
            }
        }
    }
}

fn lumie_shutdown() {
    unsafe { console::term_writeln("Shutting down..."); }
    lumie_stall(500000);
    if let Some(rt) = crate::globals::get_rt() {
        unsafe {
            if let Some(reset) = rt.reset_system {
                reset(EfiResetType::EfiResetShutdown, EFI_SUCCESS, 0, ptr::null_mut());
            }
        }
    }
}

fn pad2(buf: &mut [u8], pos: &mut usize, max: usize, val: i32) {
    if *pos >= max - 4 {
        return;
    }
    let mut tmp: [u8; 8] = [0u8; 8];
    let mut ti: usize = 0;
    let v = if val < 0 { 0 } else { val };
    let mut x = v;
    loop {
        tmp[ti] = b'0' + (x % 10) as u8;
        ti += 1;
        x /= 10;
        if x == 0 || ti >= 7 {
            break;
        }
    }
    if val < 10 && ti == 1 && *pos < max - 1 {
        buf[*pos] = b'0';
        *pos += 1;
    }
    while ti > 0 && *pos < max - 1 {
        ti -= 1;
        buf[*pos] = tmp[ti];
        *pos += 1;
    }
}

fn lumie_get_time(buf: &mut [u8], max_len: i32) -> i32 {
    let rt = match crate::globals::get_rt() {
        Some(r) => r,
        None => return -1,
    };
    let mut et: EfiTime = unsafe { core::mem::zeroed() };
    let mut cap: EfiTimeCapabilities = unsafe { core::mem::zeroed() };

    let st = unsafe {
        if let Some(get_time) = rt.get_time {
            get_time(&mut et, &mut cap)
        } else {
            return -1;
        }
    };
    if st != EFI_SUCCESS {
        return -1;
    }

    let max = max_len as usize;
    let mut p: usize = 0;

    let mut yb: [u8; 16] = [0u8; 16];
    unsafe {
        lumie_std::format::lumie_itoa(et.year as i64, yb.as_mut_ptr(), 10);
    }
    for &c in yb.iter() {
        if c == 0 || p >= max - 1 {
            break;
        }
        buf[p] = c;
        p += 1;
    }
    if p < max - 1 {
        buf[p] = b'-';
        p += 1;
    }
    pad2(buf, &mut p, max, et.month as i32);
    if p < max - 1 {
        buf[p] = b'-';
        p += 1;
    }
    pad2(buf, &mut p, max, et.day as i32);
    if p < max - 1 {
        buf[p] = b' ';
        p += 1;
    }
    pad2(buf, &mut p, max, et.hour as i32);
    if p < max - 1 {
        buf[p] = b':';
        p += 1;
    }
    pad2(buf, &mut p, max, et.minute as i32);
    if p < max - 1 {
        buf[p] = b':';
        p += 1;
    }
    pad2(buf, &mut p, max, et.second as i32);

    if et.time_zone != 2047 && p < max - 10 {
        let mut tz_total = et.time_zone as i32;
        let tz_sign: u8 = if tz_total < 0 {
            tz_total = -tz_total;
            b'-'
        } else {
            b'+'
        };
        let tz_h = tz_total / 60;
        let tz_m = tz_total % 60;
        buf[p] = b' ';
        p += 1;
        buf[p] = b'U';
        p += 1;
        buf[p] = b'T';
        p += 1;
        buf[p] = b'C';
        p += 1;
        buf[p] = tz_sign;
        p += 1;
        let mut tb: [u8; 8] = [0u8; 8];
        unsafe {
            lumie_std::format::lumie_itoa(tz_h as i64, tb.as_mut_ptr(), 10);
        }
        for &c in tb.iter() {
            if c == 0 || p >= max - 1 {
                break;
            }
            buf[p] = c;
            p += 1;
        }
        if p < max - 1 {
            buf[p] = b':';
            p += 1;
        }
        pad2(buf, &mut p, max, tz_m);
    } else if p < max - 5 {
        buf[p] = b' ';
        p += 1;
        buf[p] = b'U';
        p += 1;
        buf[p] = b'T';
        p += 1;
        buf[p] = b'C';
        p += 1;
    }

    if p < max {
        buf[p] = 0;
    }
    p as i32
}

fn try_register_boot_entry(base_num: u16, attrs: u32) -> i32 {
    let bs = match crate::globals::get_bs() {
        Some(b) => b,
        None => return -1,
    };
    let rt = match crate::globals::get_rt() {
        Some(r) => r,
        None => return -1,
    };
    let image_handle = match crate::globals::get_image_handle() {
        Some(h) => h,
        None => return -1,
    };

    let loaded_image_guid = &EFI_LOADED_IMAGE_PROTOCOL_GUID as *const EfiGuid;
    let mut loaded_image: *mut EfiLoadedImageProtocol = ptr::null_mut();
    let status = unsafe {
        let hp = bs.handle_protocol.ok_or(-1)?;
        hp(
            image_handle,
            loaded_image_guid,
            &mut loaded_image as *mut *mut EfiLoadedImageProtocol as *mut *mut c_void,
        )
    };
    if status != EFI_SUCCESS || loaded_image.is_null() {
        return -1;
    }
    let loaded_image_ref = unsafe { &*loaded_image };
    let device_handle = loaded_image_ref.device_handle;
    if device_handle.is_null() {
        return -1;
    }

    let dp_guid = &EFI_DEVICE_PATH_PROTOCOL_GUID as *const EfiGuid;
    let mut dev_path: *mut EfiDevicePathProtocol = ptr::null_mut();
    let status = unsafe {
        let hp = bs.handle_protocol.ok_or(-1)?;
        hp(
            device_handle,
            dp_guid,
            &mut dev_path as *mut *mut EfiDevicePathProtocol as *mut *mut c_void,
        )
    };
    if status != EFI_SUCCESS || dev_path.is_null() {
        return -1;
    }

    let file_path: [u16; 18] = [
        b'\\' as u16, b'E' as u16, b'F' as u16, b'I' as u16,
        b'\\' as u16, b'L' as u16, b'u' as u16, b'm' as u16,
        b'i' as u16, b'e' as u16, b'O' as u16, b'S' as u16,
        b'\\' as u16, b'B' as u16, b'O' as u16, b'O' as u16,
        b'T' as u16, 0,
    ];
    let desc: [u16; 9] = [
        b'L' as u16, b'u' as u16, b'm' as u16, b'i' as u16,
        b'e' as u16, b'O' as u16, b'S' as u16, 0, 0,
    ];

    let mut dp_len: u32 = 0;
    let mut dp = dev_path;
    unsafe {
        loop {
            if (*dp).type_ == DEVICE_PATH_TYPE_END && (*dp).sub_type == END_ENTIRE_DEVICE_PATH {
                break;
            }
            dp_len += (*dp).length as u32;
            dp = (dp as *mut u8).add((*dp).length as usize) as *mut EfiDevicePathProtocol;
        }
    }

    let mut fp_chars: u32 = 0;
    while file_path[fp_chars as usize] != 0 {
        fp_chars += 1;
    }
    let sizeof_dp = core::mem::size_of::<EfiDevicePathProtocol>() as u32;
    let mut filepath_node_len = sizeof_dp + (fp_chars + 1) * 2;
    if filepath_node_len & 1 != 0 {
        filepath_node_len += 1;
    }
    let full_dp_len = dp_len + filepath_node_len + sizeof_dp;

    let mut d_chars: u32 = 0;
    while desc[d_chars as usize] != 0 {
        d_chars += 1;
    }
    let desc_len = (d_chars + 1) * 2;
    let option_size = core::mem::size_of::<EfiLoadOption>() as u32 + desc_len + full_dp_len;

    let mut option_data: *mut u8 = ptr::null_mut();
    let status = unsafe {
        let ap = bs.allocate_pool.ok_or(-1)?;
        ap(
            EFI_BOOT_SERVICES_DATA,
            option_size as u64,
            &mut option_data as *mut *mut u8 as *mut *mut c_void,
        )
    };
    if status != EFI_SUCCESS || option_data.is_null() {
        return -1;
    }
    unsafe {
        ptr::write_bytes(option_data, 0, option_size as usize);
    }

    let option = option_data as *mut EfiLoadOption;
    unsafe {
        (*option).attributes = LOAD_OPTION_ACTIVE | LOAD_OPTION_CATEGORY_APP;
        (*option).file_path_list_length = full_dp_len as u16;
    }

    let mut ptr_offset = unsafe { option_data.add(core::mem::size_of::<EfiLoadOption>()) };
    unsafe {
        ptr::copy_nonoverlapping(desc.as_ptr() as *const u8, ptr_offset, desc_len as usize);
    }
    ptr_offset = unsafe { ptr_offset.add(desc_len as usize) };

    unsafe {
        ptr::copy_nonoverlapping(dev_path as *const u8, ptr_offset, dp_len as usize);
    }
    ptr_offset = unsafe { ptr_offset.add(dp_len as usize) };

    let fp_node = ptr_offset as *mut EfiDevicePathProtocol;
    unsafe {
        (*fp_node).type_ = DEVICE_PATH_TYPE_MEDIA;
        (*fp_node).sub_type = MEDIA_FILEPATH_DP;
        (*fp_node).length = filepath_node_len as u16;
        ptr::copy_nonoverlapping(
            file_path.as_ptr() as *const u8,
            ptr_offset.add(sizeof_dp as usize),
            (filepath_node_len - sizeof_dp) as usize,
        );
    }
    ptr_offset = unsafe { ptr_offset.add(filepath_node_len as usize) };

    let end_node = ptr_offset as *mut EfiDevicePathProtocol;
    unsafe {
        (*end_node).type_ = DEVICE_PATH_TYPE_END;
        (*end_node).sub_type = END_ENTIRE_DEVICE_PATH;
        (*end_node).length = sizeof_dp as u16;
    }

    let global_guid = &EFI_GLOBAL_VARIABLE_GUID as *const EfiGuid;
    let mut boot_order_buf: [u16; 128] = [0u16; 128];
    let mut boot_order_size: u64 = (boot_order_buf.len() * 2) as u64;
    let mut bo_attrs: u32 = 0;
    let mut boot_order: *const u16 = ptr::null();
    let mut existing_count: u64 = 0;

    let status = unsafe {
        let gv = rt.get_variable.ok_or(-1)?;
        gv(
            BOOT_ORDER_NAME.as_ptr() as *mut u16,
            global_guid,
            &mut bo_attrs,
            &mut boot_order_size,
            boot_order_buf.as_mut_ptr() as *mut c_void,
        )
    };
    if status == EFI_SUCCESS {
        existing_count = boot_order_size / 2;
        if existing_count > 0 {
            boot_order = boot_order_buf.as_ptr();
        }
    }

    let hex_digits = b"0123456789ABCDEF";
    let mut new_boot_num: u16 = 0;
    let mut found = false;
    for candidate in base_num..(base_num + 0xFF) {
        let mut name_buf: [u16; 9] = [0u16; 9];
        name_buf[0] = b'B' as u16;
        name_buf[1] = b'o' as u16;
        name_buf[2] = b'o' as u16;
        name_buf[3] = b't' as u16;
        name_buf[4] = hex_digits[((candidate >> 12) & 0xF) as usize] as u16;
        name_buf[5] = hex_digits[((candidate >> 8) & 0xF) as usize] as u16;
        name_buf[6] = hex_digits[((candidate >> 4) & 0xF) as usize] as u16;
        name_buf[7] = hex_digits[(candidate & 0xF) as usize] as u16;
        name_buf[8] = 0;

        let mut size: u64 = 0;
        let gs = unsafe {
            let gv = rt.get_variable.ok_or(-1)?;
            gv(
                name_buf.as_mut_ptr(),
                global_guid,
                ptr::null_mut(),
                &mut size,
                ptr::null_mut(),
            )
        };
        if gs == EFI_NOT_FOUND || ((gs as i64) < 0 && gs != 0) {
            if gs == EFI_NOT_FOUND {
                new_boot_num = candidate;
                found = true;
                break;
            }
        }
    }

    if !found {
        unsafe { let _ = bs.free_pool.map(|fp| fp(option_data as *mut c_void)); }
        return -1;
    }

    let mut bootvar_name: [u16; 9] = [0u16; 9];
    bootvar_name[0] = b'B' as u16;
    bootvar_name[1] = b'o' as u16;
    bootvar_name[2] = b'o' as u16;
    bootvar_name[3] = b't' as u16;
    bootvar_name[4] = hex_digits[((new_boot_num >> 12) & 0xF) as usize] as u16;
    bootvar_name[5] = hex_digits[((new_boot_num >> 8) & 0xF) as usize] as u16;
    bootvar_name[6] = hex_digits[((new_boot_num >> 4) & 0xF) as usize] as u16;
    bootvar_name[7] = hex_digits[(new_boot_num & 0xF) as usize] as u16;
    bootvar_name[8] = 0;

    let status = unsafe {
        let sv = rt.set_variable.ok_or(-1)?;
        sv(
            bootvar_name.as_mut_ptr(),
            global_guid,
            attrs,
            option_size as u64,
            option_data as *mut c_void,
        )
    };
    unsafe { let _ = bs.free_pool.map(|fp| fp(option_data as *mut c_void)); }
    if status != EFI_SUCCESS {
        return -1;
    }

    let mut new_boot_order_buf: [u16; 129] = [0u16; 129];
    new_boot_order_buf[0] = new_boot_num;
    let mut new_boot_order_size: u64 = 2;
    if !boot_order.is_null() && existing_count > 0 {
        let count = if existing_count > 128 { 128 } else { existing_count };
        unsafe {
            ptr::copy_nonoverlapping(boot_order, new_boot_order_buf.as_mut_ptr().add(1), count as usize);
        }
        new_boot_order_size = (existing_count + 1) * 2;
    }

    let status = unsafe {
        let sv = rt.set_variable.ok_or(-1)?;
        sv(
            BOOT_ORDER_NAME.as_ptr() as *mut u16,
            global_guid,
            attrs,
            new_boot_order_size,
            new_boot_order_buf.as_ptr() as *const c_void as *mut c_void,
        )
    };
    if status != EFI_SUCCESS { -1 } else { 0 }
}

fn try_copy_fallback_boot() -> i32 {
    let bs = match crate::globals::get_bs() {
        Some(b) => b,
        None => return -1,
    };
    let image_handle = match crate::globals::get_image_handle() {
        Some(h) => h,
        None => return -1,
    };

    let loaded_image_guid = &EFI_LOADED_IMAGE_PROTOCOL_GUID as *const EfiGuid;
    let mut loaded_image: *mut EfiLoadedImageProtocol = ptr::null_mut();
    let status = unsafe {
        let hp = bs.handle_protocol.ok_or(-1)?;
        hp(
            image_handle,
            loaded_image_guid,
            &mut loaded_image as *mut *mut EfiLoadedImageProtocol as *mut *mut c_void,
        )
    };
    if status != EFI_SUCCESS || loaded_image.is_null() {
        return -1;
    }
    let li = unsafe { &*loaded_image };
    if li.image_base.is_null() || li.image_size == 0 {
        return -1;
    }

    let sf_guid = &EFI_SIMPLE_FILE_SYSTEM_GUID as *const EfiGuid;
    let mut vol: *mut EfiSimpleFileSystemProtocol = ptr::null_mut();
    let status = unsafe {
        let hp = bs.handle_protocol.ok_or(-1)?;
        hp(
            li.device_handle,
            sf_guid,
            &mut vol as *mut *mut EfiSimpleFileSystemProtocol as *mut *mut c_void,
        )
    };
    if status != EFI_SUCCESS || vol.is_null() {
        return -1;
    }

    let mut root: *mut EfiFileProtocol = ptr::null_mut();
    let status = unsafe {
        let ov = (*vol).open_volume.ok_or(-1)?;
        ov(vol as *mut c_void, &mut root as *mut *mut EfiFileProtocol as *mut *mut c_void)
    };
    if status != EFI_SUCCESS || root.is_null() {
        return -1;
    }

    let dir_path: [u16; 10] = [
        b'\\' as u16, b'E' as u16, b'F' as u16, b'I' as u16,
        b'\\' as u16, b'B' as u16, b'O' as u16, b'O' as u16,
        b'T' as u16, 0,
    ];
    let file_name: [u16; 10] = [
        b'B' as u16, b'O' as u16, b'O' as u16, b'T' as u16,
        b'X' as u16, b'6' as u16, b'4' as u16, b'.' as u16,
        b'E' as u16, 0,
    ];

    let mut dir: *mut EfiFileProtocol = ptr::null_mut();
    let status = unsafe {
        let open = (*root).open.ok_or(-1)?;
        open(root, &mut dir, dir_path.as_ptr() as *mut char16, EFI_FILE_MODE_READ, 0)
    };
    if status != EFI_SUCCESS || dir.is_null() {
        unsafe { let _ = (*root).close.map(|c| c(root)); }
        return -1;
    }

    let mut file: *mut EfiFileProtocol = ptr::null_mut();
    let status = unsafe {
        let open = (*dir).open.ok_or(-1)?;
        open(
            dir,
            &mut file,
            file_name.as_ptr() as *mut char16,
            EFI_FILE_MODE_READ | EFI_FILE_MODE_WRITE | EFI_FILE_MODE_CREATE,
            0,
        )
    };
    if status != EFI_SUCCESS || file.is_null() {
        unsafe {
            let _ = (*dir).close.map(|c| c(dir));
            let _ = (*root).close.map(|c| c(root));
        }
        return -1;
    }

    let mut write_size = li.image_size;
    let status = unsafe {
        let write = (*file).write.ok_or(-1)?;
        write(file, &mut write_size, li.image_base)
    };
    unsafe {
        let _ = (*file).close.map(|c| c(file));
        let _ = (*dir).close.map(|c| c(dir));
        let _ = (*root).close.map(|c| c(root));
    }
    if status != EFI_SUCCESS { -1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn lumie_efi_register_boot_entry() -> i32 {
    if crate::globals::get_bs().is_none() || crate::globals::get_rt().is_none() {
        return -1;
    }

    let strategies: [(u16, u32); 5] = [
        (0x0000, EFI_VARIABLE_NON_VOLATILE | EFI_VARIABLE_BOOTSERVICE_ACCESS | EFI_VARIABLE_RUNTIME_ACCESS),
        (0x0100, EFI_VARIABLE_NON_VOLATILE | EFI_VARIABLE_BOOTSERVICE_ACCESS | EFI_VARIABLE_RUNTIME_ACCESS),
        (0x8000, EFI_VARIABLE_NON_VOLATILE | EFI_VARIABLE_BOOTSERVICE_ACCESS | EFI_VARIABLE_RUNTIME_ACCESS),
        (0x0000, EFI_VARIABLE_NON_VOLATILE | EFI_VARIABLE_BOOTSERVICE_ACCESS),
        (0x0000, 0),
    ];

    for &(base_num, attrs) in &strategies {
        if try_register_boot_entry(base_num, attrs) == 0 {
            return 0;
        }
    }
    try_copy_fallback_boot()
}

#[no_mangle]
pub extern "C" fn exit_boot_services() {
    let bs = match crate::globals::get_bs() {
        Some(b) => b,
        None => return,
    };
    let image_handle = match crate::globals::get_image_handle() {
        Some(h) => h,
        None => return,
    };

    let ebs = bs.exit_boot_services;
    let get_mmap = bs.get_memory_map;
    let allocate_pool = bs.allocate_pool;
    let free_pool = bs.free_pool;

    let mut map_key: u64 = mm::get_map_key();
    let mut desc_size: u64 = mm::get_desc_size();
    let mut desc_ver: u32 = mm::get_desc_ver();

    for _ in 0..3 {
        let status = unsafe {
            if let Some(ebs_fn) = ebs {
                ebs_fn(image_handle, map_key)
            } else {
                break;
            }
        };
        if status == EFI_SUCCESS {
            crate::globals::set_bs(unsafe { &*(ptr::null::<EfiBootServices>()) });
            return;
        }

        let mut mmap_size: u64 = 0;
        let mut new_key: u64 = 0;

        unsafe {
            if let Some(gm) = get_mmap {
                gm(&mut mmap_size, ptr::null_mut(), &mut new_key, &mut desc_size, &mut desc_ver);
            }
        }
        mmap_size += desc_size * 16;

        let mut mmap_buf: *mut u8 = ptr::null_mut();
        let st = unsafe {
            if let Some(ap) = allocate_pool {
                ap(EFI_BOOT_SERVICES_DATA, mmap_size, &mut mmap_buf as *mut *mut u8 as *mut *mut c_void)
            } else {
                break;
            }
        };
        if st != EFI_SUCCESS || mmap_buf.is_null() {
            break;
        }

        let st = unsafe {
            if let Some(gm) = get_mmap {
                gm(&mut mmap_size, mmap_buf as *mut EfiMemoryDescriptor, &mut new_key, &mut desc_size, &mut desc_ver)
            } else {
                break;
            }
        };
        if st != EFI_SUCCESS {
            unsafe { let _ = free_pool.map(|fp| fp(mmap_buf as *mut c_void)); }
            break;
        }

        unsafe { let _ = free_pool.map(|fp| fp(mmap_buf as *mut c_void)); }
        map_key = new_key;
    }

    crate::globals::set_bs(unsafe { &*(ptr::null::<EfiBootServices>()) });
}

#[no_mangle]
pub extern "C" fn lumie_ps2_available() -> i32 {
    drivers::ps2mouse::is_ready()
}

#[no_mangle]
pub extern "C" fn lumie_sched_init() {
    unsafe { sched::sched_init(); }
}

fn lumie_users_init() -> i32 {
    -1
}

fn lumie_registry_init() -> i32 {
    -1
}

#[no_mangle]
pub extern "C" fn lumie_cache_kernel_image(base: *const c_void, size: u32) {
    crate::globals::set_kernel_image(base, size);
}

fn lumie_get_kernel_image(base: *mut *const c_void, size: *mut u32) -> i32 {
    let (kb, ks) = crate::globals::get_kernel_image();
    if kb.is_null() || ks == 0 {
        return -1;
    }
    unsafe {
        if !base.is_null() {
            *base = kb;
        }
        if !size.is_null() {
            *size = ks;
        }
    }
    0
}

struct LumieFmtPrinter {
    buf: [u8; 1024],
    pos: usize,
}

impl LumieFmtPrinter {
    fn new() -> Self {
        LumieFmtPrinter { buf: [0u8; 1024], pos: 0 }
    }

    fn write_byte(&mut self, b: u8) {
        if self.pos < 1023 {
            self.buf[self.pos] = b;
            self.pos += 1;
        }
    }

    fn write_str(&mut self, s: &[u8]) {
        for &b in s {
            self.write_byte(b);
        }
    }

    fn write_i64(&mut self, v: i64) {
        let mut tmp: [u8; 32] = [0u8; 32];
        unsafe { lumie_std::format::lumie_itoa(v, tmp.as_mut_ptr(), 10); }
        for i in 0..32 {
            if tmp[i] == 0 { break; }
            self.write_byte(tmp[i]);
        }
    }

    fn write_u32_hex(&mut self, v: u32) {
        let mut tmp: [u8; 32] = [0u8; 32];
        unsafe { lumie_std::format::lumie_itoa(v as i64, tmp.as_mut_ptr(), 16); }
        for i in 0..32 {
            if tmp[i] == 0 { break; }
            self.write_byte(tmp[i]);
        }
    }

    fn finish(&mut self) {
        self.buf[self.pos] = 0;
        unsafe { console::term_write(cstr(self.buf.as_ptr())); }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lumie_printf(fmt: *const u8, mut args: ...) {
    let mut printer = LumieFmtPrinter::new();
    let mut fp = fmt;
    loop {
        let c = *fp;
        if c == 0 || printer.pos >= 1023 { break; }
        if c != b'%' {
            printer.write_byte(c);
            fp = fp.add(1);
            continue;
        }
        fp = fp.add(1);
        let spec = *fp;
        if spec == 0 { break; }
        match spec {
            b's' => {
                let s: *const u8 = args.arg();
                let mut sp = s;
                while *sp != 0 && printer.pos < 1023 {
                    printer.write_byte(*sp);
                    sp = sp.add(1);
                }
            }
            b'd' => { printer.write_i64(args.arg::<i32>() as i64); }
            b'u' => { printer.write_i64(args.arg::<u32>() as i64); }
            b'x' | b'X' => { printer.write_u32_hex(args.arg::<u32>()); }
            b'c' => { printer.write_byte(args.arg::<i32>() as u8); }
            b'%' => { printer.write_byte(b'%'); }
            _ => {}
        }
        fp = fp.add(1);
    }
    printer.finish();
}
