#![no_std]

extern crate lumie_std;

pub mod api;
pub mod term;
pub mod input;
pub mod mem;
pub mod fs;
pub mod gpu;
pub mod sched;
pub mod sys;
pub mod driver;

pub use api::*;
pub use driver::{AppContext, DriverExport, DriverModule};
pub use term::{Terminal, Kbd};
pub use input::Mouse;
pub use mem::Memory;
pub use fs::FileSystem;
pub use gpu::Gpu;
pub use sched::Scheduler;
pub use sys::System;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
