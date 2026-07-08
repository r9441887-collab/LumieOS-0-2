use crate::mm::paging;

pub const HEAP_POOL_SIZE: u64 = 64 * 1024 * 1024;

#[repr(C)]
struct HeapHeader {
    next: *mut HeapHeader,
    size: u64,
    free: bool,
}

impl HeapHeader {
    #[allow(dead_code)]
    const fn new() -> Self {
        Self {
            next: core::ptr::null_mut(),
            size: 0,
            free: false,
        }
    }
}

struct HeapAllocator {
    heap_start: *mut u8,
    heap_first: *mut HeapHeader,
}

impl HeapAllocator {
    const fn new() -> Self {
        Self {
            heap_start: core::ptr::null_mut(),
            heap_first: core::ptr::null_mut(),
        }
    }
}

static mut ALLOCATOR: HeapAllocator = HeapAllocator::new();

pub unsafe fn init(heap_start: *mut u8) {
    ALLOCATOR.heap_start = heap_start;
    let hdr_size = core::mem::size_of::<HeapHeader>() as u64;
    let first = heap_start as *mut HeapHeader;
    *first = HeapHeader {
        next: core::ptr::null_mut(),
        size: HEAP_POOL_SIZE - hdr_size,
        free: true,
    };
    ALLOCATOR.heap_first = first;
}

unsafe fn find_free_block(last: *mut *mut HeapHeader, size: u64) -> *mut HeapHeader {
    let mut cur = ALLOCATOR.heap_first;
    while !cur.is_null() && !((*cur).free && (*cur).size >= size) {
        *last = cur;
        cur = (*cur).next;
    }
    cur
}

unsafe fn split_block(h: *mut HeapHeader, size: u64) -> *mut HeapHeader {
    let hdr_size = core::mem::size_of::<HeapHeader>() as u64;
    if (*h).size < size + hdr_size + 32 {
        return h;
    }
    let new_hdr = (h as u64 + hdr_size + size) as *mut HeapHeader;
    *new_hdr = HeapHeader {
        next: (*h).next,
        size: (*h).size - size - hdr_size,
        free: true,
    };
    (*h).size = size;
    (*h).next = new_hdr;
    h
}

pub unsafe fn kmalloc(size: u64) -> *mut u8 {
    let mut size = size;
    if size == 0 {
        size = 1;
    }
    size = (size + 7) & !7;

    let mut last: *mut HeapHeader = core::ptr::null_mut();
    let h = find_free_block(&mut last, size);
    if h.is_null() {
        return core::ptr::null_mut();
    }
    let h = split_block(h, size);
    (*h).free = false;
    (h as u64 + core::mem::size_of::<HeapHeader>() as u64) as *mut u8
}

pub unsafe fn kfree(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    let h = (ptr as u64 - core::mem::size_of::<HeapHeader>() as u64) as *mut HeapHeader;
    (*h).free = true;

    let mut cur = ALLOCATOR.heap_first;
    while !cur.is_null() {
        if (*cur).free && !(*cur).next.is_null() && (*(*cur).next).free {
            let next_size = (*(*cur).next).size;
            (*cur).size += core::mem::size_of::<HeapHeader>() as u64 + next_size;
            (*cur).next = (*(*cur).next).next;
        } else {
            cur = (*cur).next;
        }
    }
}

pub unsafe fn alloc(size: u64) -> *mut u8 {
    kmalloc(size)
}

pub unsafe fn free(ptr: *mut u8) {
    kfree(ptr)
}

pub unsafe fn get_free_mem() -> u64 {
    let mut total = 0u64;
    let mut cur = ALLOCATOR.heap_first;
    while !cur.is_null() {
        if (*cur).free {
            total += (*cur).size;
        }
        cur = (*cur).next;
    }
    total += paging::get_page_stack_top() * paging::PAGE_SIZE;
    total
}
