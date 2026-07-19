# LumieOS SDK

SDK for developing applications, drivers, and modules for LumieOS.

## Structure

```
sdk/
  lumie_sdk/          Rust SDK crate - safe bindings for KAPI
    src/
      api/            Raw kernel API vtable types
      term/           Terminal I/O (write, colors, cursor)
      input/          Keyboard and mouse input
      mem/            Memory allocation (alloc, free, memset, memcpy)
      fs/             Filesystem operations (read, write, mkdir, list)
      gpu/            Graphics (pixels, rectangles, rendering)
      sched/          Scheduler (task info)
      sys/            System services (time, PCI, disks, modules)
      driver/         Driver framework + AppContext
  lumiec_sdk/         LumieC SDK headers (.h files for C-like language)
    lumie_os.h        Syscall definitions + constants
    lumie_term.h      Terminal functions
    lumie_mem.h       Memory management
    lumie_gfx.h       Graphics functions
    lumie_fs.h        Filesystem functions
    lumie_input.h     Input handling
    lumie_os_all.h    All-in-one include
  templates/          Starter project templates
    app/              Template for Rust SDK applications
    driver/           Template for Rust SDK drivers
    lumiec_app/       Template for LumieC programs
  examples/           Example projects
    hello/            Hello World with system info
    sysinfo/          Detailed system information
    gpu_demo/         GPU rendering demo
  sdk_build.py        Unified build tool
```

## Quick Start

### Rust SDK App

```rust
#![no_std]
extern crate lumie_sdk;
use lumie_sdk::AppContext;

lumie_sdk::lumie_app_entry!(my_app);

fn my_app(ctx: &AppContext) {
    ctx.term().clear(0x000030);
    ctx.term().writeln("Hello from LumieOS!");
    ctx.kbd().read_key_blocking();
}
```

Build: `python sdk/sdk_build.py app my_app`

### LumieC Program

```c
#include "lumie_os_all.h"

int lumiec_main(void) {
    term_clear(COLOR_BLUE);
    term_write("Hello LumieC!");
    draw_rect(100, 100, 200, 100, CLR_RED);
    input_waitkey();
    return 0;
}
```

Build: Place on disk, run `lumiec hello.lc` in the OS shell.

### Rust SDK Driver

```rust
#![no_std]
extern crate lumie_sdk;
use lumie_sdk::{DriverExport, DriverModule, KernelApiV1};

pub struct MyDriver { exports: DriverExport }

impl DriverModule for MyDriver {
    fn init(&mut self, _kapi: *const KernelApiV1) -> i32 { 0 }
    fn exports(&self) -> &DriverExport { &self.exports }
}

lumie_sdk::lumie_driver_entry!(MyDriver);
```

Build: `python sdk/sdk_build.py driver my_driver`

## API Reference

### Terminal (`ctx.term()`)
- `clear(bg)` - Clear screen with background color
- `set_fg(color)` / `set_bg(color)` - Set text colors
- `set_pos(x, y)` - Set cursor position
- `write(s)` / `writeln(s)` - Write text
- `putchar(c)` - Write single character
- `width()` / `height()` - Get terminal dimensions

### Keyboard (`ctx.kbd()`)
- `getchar()` - Non-blocking key read (-1 if none)
- `kbhit()` - Check if key available
- `flush()` - Drain keyboard buffer
- `read_key_blocking()` - Wait for and return key

### Memory (`ctx.mem()`)
- `alloc(size)` / `free(ptr)` - Heap allocation
- `calloc(count, size)` - Zeroed allocation
- `memset` / `memcpy` - Memory operations
- `total()` / `free_mem()` / `used()` - Memory info

### Filesystem (`ctx.fs()`)
- `read(path, buf)` / `write(path, data)` - File I/O
- `exists(path)` - Check file existence
- `list_dir(path, entries)` / `mkdir(path)` - Directory ops
- `read_to_string(path, max)` - Read file as string

### GPU (`ctx.gpu()`)
- `put_pixel(x, y, color)` - Draw single pixel
- `fill_rect(x, y, w, h, color)` - Fill rectangle
- `draw_rect_outline(x, y, w, h, color)` - Rectangle outline
- `flip()` / `vsync()` - Buffer operations
- `is_active()` - Check GPU acceleration

### System (`ctx.sys()`)
- `stall(us)` / `sleep_ms(ms)` - Timing
- `reboot()` / `shutdown()` - System control
- `get_time(buf)` - Get current time
- `pci_scan(index)` - Enumerate PCI devices
- `disk_count()` / `disk_name(id)` - Disk info
- `mod_load(path)` - Load module

## Module Formats

| Magic | Extension | Type | Description |
|-------|-----------|------|-------------|
| `LKRN` | `.lkrn` | Kernel | Core OS kernel |
| `LSH` | `.lsh` | App | Shell applications |
| `LDRV` | `.ldrv` | Driver | Loadable drivers |
| `SYS` | `.sys` | Program | LumieC compiled programs |

## Build Commands

```bash
# Build a Rust SDK app
python sdk/sdk_build.py app my_app

# Build a Rust SDK driver
python sdk/sdk_build.py driver my_driver

# Build kernel
python sdk/sdk_build.py kernel

# Clean build artifacts
python sdk/sdk_build.py clean

# Create install package
python sdk/sdk_build.py package
```

## Disk Layout

```
/system/
  kernel.lkrn      Kernel module
  shell.lsh         Shell
  editor.lsh        Text editor
  desktop.lsh       Desktop environment
  users.cfg         User accounts
  registry.cfg      Configuration registry
/drivers/
  kbd.ldrv          Keyboard driver
  mouse.ldrv        Mouse driver
  fs.ldrv           Filesystem driver
  nv_gpu.sys        NVIDIA GPU driver
```
