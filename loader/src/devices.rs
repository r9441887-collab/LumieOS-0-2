#![no_std]

use core::ffi::c_void;
use core::ptr;
use crate::uefi::*;

pub struct LoaderBlockDevice {
    pub handle: efi_handle,
    pub label: [u8; 64],
    pub block_count: u64,
    pub block_size: u32,
    pub is_removable: u8,
    pub is_partition: u8,
}

type EfiBlockIoReadBlocks =
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
    pub block_size: u64,
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
    pub write_blocks: *mut c_void,
    pub flush_blocks: *mut c_void,
}

pub fn loader_enum_block_devices(bs: &EfiBootServices, devices: &mut [LoaderBlockDevice]) -> i32 {
    let max = devices.len();
    let bio_guid = &EFI_BLOCK_IO_GUID as *const EfiGuid;
    let mut handle_count: u64 = 0;
    let mut handles: *mut efi_handle = ptr::null_mut();

    let st = unsafe {
        if let Some(lhb) = bs.locate_handle_buffer {
            lhb(2, bio_guid, ptr::null_mut(), &mut handle_count, &mut handles)
        } else {
            return 0;
        }
    };
    if st != 0 || handles.is_null() {
        return 0;
    }

    let mut count: i32 = 0;
    for i in 0..handle_count {
        if count >= max as i32 {
            break;
        }

        let h = unsafe { *handles.add(i as usize) };
        let mut bio: *mut EfiBlockIoProtocol = ptr::null_mut();
        let st = unsafe {
            if let Some(hp) = bs.handle_protocol {
                hp(
                    h,
                    bio_guid,
                    &mut bio as *mut *mut EfiBlockIoProtocol as *mut *mut c_void,
                )
            } else {
                break;
            }
        };
        if st != 0 || bio.is_null() {
            continue;
        }

        let media = unsafe { (*bio).media };
        if media.is_null() {
            continue;
        }

        let media_ref = unsafe { &*media };
        if media_ref.media_present == 0 && media_ref.removable_media == 0 {
            continue;
        }

        devices[count as usize].handle = h;
        devices[count as usize].block_count =
            (media_ref.last_block + 1) - media_ref.lowest_aligned_lba;
        devices[count as usize].block_size =
            if media_ref.block_size != 0 { media_ref.block_size as u32 } else { 512 };
        devices[count as usize].is_removable = media_ref.removable_media;
        devices[count as usize].is_partition = media_ref.logical_partition;

        /* Detect FAT32 from BPB */
        let mut is_fat32 = false;
        let mut sector: [u8; 512] = [0u8; 512];
        let rs = unsafe {
            if let Some(rb) = (*bio).read_blocks {
                rb(
                    bio as *mut c_void,
                    media_ref.media_id,
                    0,
                    512,
                    sector.as_mut_ptr() as *mut c_void,
                )
            } else {
                1
            }
        };
        if rs == 0 {
            let spf = u32::from_le_bytes([sector[36], sector[37], sector[38], sector[39]]);
            if spf != 0 {
                is_fat32 = true;
            }
        }

        let r = if devices[count as usize].is_removable != 0 {
            b"USB Flash"
        } else {
            b"Disk"
        };
        let mut label = [0u8; 64];
        let mut lp = 0;
        for &c in r {
            if lp < 63 {
                label[lp] = c;
                lp += 1;
            }
        }

        if devices[count as usize].is_partition != 0 {
            if is_fat32 {
                let mut vol: [u8; 12] = [0u8; 12];
                let mut vi = 0;
                for j in 43..54 {
                    if vi < 11 {
                        vol[vi] = sector[j];
                        vi += 1;
                    }
                }
                vol[11] = 0;
                let tag = b" [FAT32: ";
                for &c in tag {
                    if lp < 63 { label[lp] = c; lp += 1; }
                }
                for &c in vol.iter() {
                    if c == 0 { break; }
                    if lp < 63 { label[lp] = c; lp += 1; }
                }
                if lp < 63 { label[lp] = b']'; lp += 1; }
            } else {
                let tag = b" (Partition)";
                for &c in tag { if lp < 63 { label[lp] = c; lp += 1; } }
            }
        } else if devices[count as usize].is_removable != 0 {
            let tag = b" (Removable)";
            for &c in tag { if lp < 63 { label[lp] = c; lp += 1; } }
        } else {
            let tag = b" (Fixed)";
            for &c in tag { if lp < 63 { label[lp] = c; lp += 1; } }
        }

        devices[count as usize].label = label;
        count += 1;
    }

    unsafe {
        if let Some(fp) = bs.free_pool {
            fp(handles as *mut c_void);
        }
    }
    count
}

pub fn loader_show_device_menu(devices: &[LoaderBlockDevice]) -> i32 {
    let count = devices.len();
    if count == 0 { return -1; }
    if count == 1 { return 0; }

    let bg = crate::display::ld_make_color(0x00, 0x00, 0x80);
    let white = crate::display::ld_make_color(0xFF, 0xFF, 0xFF);
    let lcyan = crate::display::ld_make_color(0x55, 0xFF, 0xFF);
    let yellow = crate::display::ld_make_color(0xFF, 0xFF, 0x00);

    let scr_w = unsafe { crate::gop_get_width() };
    let scr_h = unsafe { crate::gop_get_height() };
    let logo_y = scr_h / 5;
    let line_h = 20u32;

    let mut sel: i32 = 0;
    let mut page_offset: i32 = 0;
    let max_visible = ((scr_h - logo_y - 60) / line_h) as i32;
    let max_visible = if max_visible < 1 { 1 } else { max_visible };

    loop {
        crate::display::loader_drv_clear(bg);

        crate::display::loader_drv_draw_str(
            scr_w / 2 - 4 * 8, logo_y, lcyan, bg, b"LumieOS",
        );
        crate::display::loader_drv_draw_str(
            scr_w / 2 - 10 * 8, logo_y + line_h, white, bg, b"Select Boot Device:",
        );

        let mut y = logo_y + 3 * line_h;

        if page_offset > 0 {
            crate::display::loader_drv_draw_str(
                scr_w / 2 - 5 * 8, y - line_h, yellow, bg, b"[more above]",
            );
        }

        let end = core::cmp::min(page_offset + max_visible, count as i32);
        for i in page_offset..end {
            let idx = i as usize;
            let mut buf: [u8; 128] = [0u8; 128];
            let mut bp = 0;
            buf[bp] = b' '; bp += 1; buf[bp] = b' '; bp += 1;
            if i == sel {
                buf[bp] = b'>'; bp += 1; buf[bp] = b' '; bp += 1;
            } else {
                buf[bp] = b' '; bp += 1; buf[bp] = b' '; bp += 1;
            }

            let mut num_buf: [u8; 8] = [0u8; 8];
            unsafe { lumie_std::format::lumie_itoa((idx + 1) as i64, num_buf.as_mut_ptr(), 10); }
            for &c in num_buf.iter() {
                if c == 0 { break; }
                if bp < 127 { buf[bp] = c; bp += 1; }
            }
            if bp < 127 { buf[bp] = b'.'; bp += 1; }
            if bp < 127 { buf[bp] = b' '; bp += 1; }

            for &c in devices[idx].label.iter() {
                if c == 0 { break; }
                if bp < 127 { buf[bp] = c; bp += 1; }
            }

            let total_mb = (devices[idx].block_count * devices[idx].block_size as u64) / (1024 * 1024);
            let mut sz_buf: [u8; 32] = [0u8; 32];
            unsafe { lumie_std::format::lumie_itoa(total_mb as i64, sz_buf.as_mut_ptr(), 10); }
            let tag = b" (";
            for &c in tag { if bp < 127 { buf[bp] = c; bp += 1; } }
            for &c in sz_buf.iter() {
                if c == 0 { break; }
                if bp < 127 { buf[bp] = c; bp += 1; }
            }
            let tag2 = b" MB)";
            for &c in tag2 { if bp < 127 { buf[bp] = c; bp += 1; } }

            let color = if i == sel { yellow } else { white };
            crate::display::loader_drv_draw_str(scr_w / 4, y, color, bg, &buf[..bp]);
            y += line_h;
        }

        if page_offset + max_visible < count as i32 {
            crate::display::loader_drv_draw_str(
                scr_w / 4, y, yellow, bg, b"  [more below]",
            );
        }

        crate::display::loader_drv_draw_str(
            scr_w / 4, scr_h - 3 * line_h, white, bg,
            b"ENTER: select   ESC: cancel   UP/DOWN: navigate",
        );

        loop {
            crate::input::loader_poll_mouse();
            let mut cx: i32 = 0;
            let mut cy: i32 = 0;
            if crate::input::loader_get_click(&mut cx, &mut cy) {
                let mut item_y = (logo_y + 3 * line_h) as i32;
                for i in page_offset..end {
                    if cy >= item_y && cy < item_y + line_h as i32
                        && cx as u32 >= scr_w / 4 && cx as u32 < scr_w / 4 + 400
                    {
                        sel = i;
                        return sel;
                    }
                    item_y += line_h as i32;
                }
            }
            if !crate::input::loader_kbhit() { continue; }
            let c = crate::input::loader_getchar();
            if c == b'\n' as i32 { return sel; }
            if c == 0x1B { return -1; }
            if c == 0xE1 { if sel > 0 { sel -= 1; if sel < page_offset { page_offset = sel; } } break; }
            if c == 0xE2 { if sel < count as i32 - 1 { sel += 1; if sel >= page_offset + max_visible { page_offset = sel - max_visible + 1; } } break; }
        }
    }
}
