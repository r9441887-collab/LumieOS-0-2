use core::ffi::c_void;

use crate::uefi::memory::*;
use crate::uefi::tables::*;
use crate::uefi::types::*;

pub const PAGE_SIZE: u64 = 4096;
pub const MM_MAX_PAGES: u64 = 262144;

const EFI_CONVENTIONAL_MEMORY: u32 = 7;

struct PageAllocator {
    page_stack: *mut u64,
    page_stack_top: u64,
    page_stack_capacity: u64,
    mem_map_key: u64,
    mem_desc_size: u64,
    mem_desc_ver: u32,
}

impl PageAllocator {
    const fn new() -> Self {
        Self {
            page_stack: core::ptr::null_mut(),
            page_stack_top: 0,
            page_stack_capacity: 0,
            mem_map_key: 0,
            mem_desc_size: 0,
            mem_desc_ver: 0,
        }
    }
}

static mut ALLOCATOR: PageAllocator = PageAllocator::new();

pub unsafe fn init(bs: *mut EfiBootServices, _image_handle: efi_handle) {
    let get_mmap = (*bs).get_memory_map.unwrap();
    let alloc_pages = (*bs).allocate_pages.unwrap();
    let free_pages = (*bs).free_pages.unwrap();

    let mut mmap_size: u64 = 0;
    let mut map_key: u64 = 0;
    let mut desc_size: u64 = 0;
    let mut desc_ver: u32 = 0;

    get_mmap(&mut mmap_size, core::ptr::null_mut(), &mut map_key, &mut desc_size, &mut desc_ver);
    mmap_size += desc_size * 64;

    let pages_needed = (mmap_size + PAGE_SIZE - 1) / PAGE_SIZE;
    let mut mmap_addr: *mut c_void = core::ptr::null_mut();
    let st = alloc_pages(0, EFI_BOOT_SERVICES_DATA, pages_needed, &mut mmap_addr);
    if (st as i64) < 0 {
        return;
    }
    let mmap_buf = mmap_addr as *mut EfiMemoryDescriptor;

    let st = get_mmap(&mut mmap_size, mmap_buf, &mut map_key, &mut desc_size, &mut desc_ver);
    if (st as i64) < 0 {
        free_pages(mmap_addr, pages_needed);
        return;
    }

    let total_desc = mmap_size / desc_size;
    let mut avail_pages: u64 = 0;
    for i in 0..total_desc {
        let d = (mmap_buf as u64 + i * desc_size) as *mut EfiMemoryDescriptor;
        if (*d).type_ == EFI_CONVENTIONAL_MEMORY && (*d).physical_start >= 0x100000 {
            avail_pages += (*d).number_of_pages;
        }
    }

    free_pages(mmap_addr, pages_needed);

    if avail_pages > MM_MAX_PAGES {
        avail_pages = MM_MAX_PAGES;
    }

    let stack_pages = (avail_pages * core::mem::size_of::<u64>() as u64 + PAGE_SIZE - 1) / PAGE_SIZE;
    let heap_pages = crate::mm::heap::HEAP_POOL_SIZE / PAGE_SIZE;
    let total_needed = stack_pages + heap_pages;

    let mut alloc_addr: *mut c_void = core::ptr::null_mut();
    let st = alloc_pages(0, EFI_BOOT_SERVICES_DATA, total_needed, &mut alloc_addr);
    if (st as i64) < 0 {
        return;
    }

    let alloc_base = alloc_addr as u64;
    ALLOCATOR.page_stack = alloc_base as *mut u64;
    ALLOCATOR.page_stack_capacity = avail_pages;
    ALLOCATOR.page_stack_top = 0;

    let heap_phys = alloc_base + stack_pages * PAGE_SIZE;
    crate::mm::heap::init(heap_phys as *mut u8);

    mmap_size = 0;
    map_key = 0;
    desc_size = 0;
    desc_ver = 0;
    get_mmap(&mut mmap_size, core::ptr::null_mut(), &mut map_key, &mut desc_size, &mut desc_ver);
    mmap_size += desc_size * 16;
    let pages_needed = (mmap_size + PAGE_SIZE - 1) / PAGE_SIZE;
    let mut page_addr: *mut c_void = core::ptr::null_mut();
    let st = alloc_pages(0, EFI_BOOT_SERVICES_DATA, pages_needed, &mut page_addr);
    if (st as i64) < 0 {
        free_pages(alloc_addr, total_needed);
        return;
    }
    let mmap_buf = page_addr as *mut EfiMemoryDescriptor;

    let st = get_mmap(&mut mmap_size, mmap_buf, &mut map_key, &mut desc_size, &mut desc_ver);
    if (st as i64) < 0 {
        free_pages(page_addr, pages_needed);
        free_pages(alloc_addr, total_needed);
        return;
    }

    ALLOCATOR.mem_map_key = map_key;
    ALLOCATOR.mem_desc_size = desc_size;
    ALLOCATOR.mem_desc_ver = desc_ver;

    let total_desc = mmap_size / desc_size;
    for i in 0..total_desc {
        if ALLOCATOR.page_stack_top >= ALLOCATOR.page_stack_capacity {
            break;
        }
        let d = (mmap_buf as u64 + i * desc_size) as *mut EfiMemoryDescriptor;
        if (*d).type_ == EFI_CONVENTIONAL_MEMORY && (*d).physical_start >= 0x100000 {
            let start = (*d).physical_start;
            let count = (*d).number_of_pages;
            let end = start + count * PAGE_SIZE;
            let alloc_end = alloc_base + total_needed * PAGE_SIZE;

            if start < alloc_end {
                if end <= alloc_base {
                    for j in 0..count {
                        if ALLOCATOR.page_stack_top >= ALLOCATOR.page_stack_capacity { break; }
                        let stack = ALLOCATOR.page_stack;
                        let top = ALLOCATOR.page_stack_top as usize;
                        *stack.add(top) = start + j * PAGE_SIZE;
                        ALLOCATOR.page_stack_top += 1;
                    }
                } else {
                    let before_start = start;
                    let before_end = alloc_base;
                    if before_end > before_start {
                        let before_count = (before_end - before_start) / PAGE_SIZE;
                        for j in 0..before_count {
                            if ALLOCATOR.page_stack_top >= ALLOCATOR.page_stack_capacity { break; }
                            let stack = ALLOCATOR.page_stack;
                            let top = ALLOCATOR.page_stack_top as usize;
                            *stack.add(top) = before_start + j * PAGE_SIZE;
                            ALLOCATOR.page_stack_top += 1;
                        }
                    }
                    let after_start = alloc_end;
                    let after_end = end;
                    if after_end > after_start {
                        let after_count = (after_end - after_start) / PAGE_SIZE;
                        for j in 0..after_count {
                            if ALLOCATOR.page_stack_top >= ALLOCATOR.page_stack_capacity { break; }
                            let stack = ALLOCATOR.page_stack;
                            let top = ALLOCATOR.page_stack_top as usize;
                            *stack.add(top) = after_start + j * PAGE_SIZE;
                            ALLOCATOR.page_stack_top += 1;
                        }
                    }
                }
            } else {
                for j in 0..count {
                    if ALLOCATOR.page_stack_top >= ALLOCATOR.page_stack_capacity { break; }
                    let stack = ALLOCATOR.page_stack;
                    let top = ALLOCATOR.page_stack_top as usize;
                    *stack.add(top) = start + j * PAGE_SIZE;
                    ALLOCATOR.page_stack_top += 1;
                }
            }
        }
    }

    free_pages(page_addr, pages_needed);
}

pub unsafe fn alloc_pages(count: u32) -> Option<*mut u8> {
    if ALLOCATOR.page_stack_top < count as u64 {
        return None;
    }
    let phys: u64;
    if count == 1 {
        ALLOCATOR.page_stack_top -= 1;
        let stack = ALLOCATOR.page_stack;
        phys = *stack.add(ALLOCATOR.page_stack_top as usize);
    } else {
        let top = ALLOCATOR.page_stack_top as usize;
        let stack = ALLOCATOR.page_stack;
        let first_page = *stack.add(top - count as usize);
        let mut ok = true;
        for i in 0..count {
            let page = *stack.add(top - count as usize + i as usize);
            if page != first_page + i as u64 * PAGE_SIZE {
                ok = false;
                break;
            }
        }
        if !ok {
            return None;
        }
        ALLOCATOR.page_stack_top -= count as u64;
        phys = first_page;
    }
    Some(phys as *mut u8)
}

pub unsafe fn free_pages(addr: *mut u8, count: u32) {
    let phys = addr as u64;
    for i in 0..count {
        if ALLOCATOR.page_stack_top >= ALLOCATOR.page_stack_capacity {
            break;
        }
        let stack = ALLOCATOR.page_stack;
        let top = ALLOCATOR.page_stack_top as usize;
        *stack.add(top) = phys + i as u64 * PAGE_SIZE;
        ALLOCATOR.page_stack_top += 1;
    }
}

pub fn get_map_key() -> u64 {
    unsafe { ALLOCATOR.mem_map_key }
}

pub fn get_desc_size() -> u64 {
    unsafe { ALLOCATOR.mem_desc_size }
}

pub fn get_desc_ver() -> u32 {
    unsafe { ALLOCATOR.mem_desc_ver }
}

pub fn get_page_stack_top() -> u64 {
    unsafe { ALLOCATOR.page_stack_top }
}
