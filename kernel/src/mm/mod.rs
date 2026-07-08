pub mod heap;
pub mod paging;

pub use heap::{alloc, free};
pub use paging::{get_map_key, get_desc_size, get_desc_ver, get_page_stack_top, PAGE_SIZE, MM_MAX_PAGES};
