#![no_std]

extern crate lumie_sdk;

use lumie_sdk::AppContext;

lumie_sdk::lumie_app_entry!(hello_main);

fn hello_main(ctx: &AppContext) {
    ctx.term().clear(0x001040);
    ctx.term().set_fg(0x0F);
    ctx.term().writeln("========================================");
    ctx.term().writeln("   LumieOS SDK - Hello World Example");
    ctx.term().writeln("========================================");
    ctx.term().writeln("");
    ctx.term().set_fg(0x0A);
    ctx.term().writeln("This app was built with lumie_sdk.");
    ctx.term().writeln("");
    ctx.term().set_fg(0x0B);
    ctx.term().write("Available modules: ");
    ctx.term().write("Terminal, Keyboard, Memory, FS, GPU, Scheduler, System");
    ctx.term().writeln("");
    ctx.term().writeln("");

    ctx.term().set_fg(0x0E);
    ctx.term().writeln("System info:");
    ctx.term().set_fg(0x07);
    ctx.term().write("  Memory total: ");
    let total = ctx.mem().total();
    ctx.term().writeln("");
    ctx.term().write("  Memory free:  ");
    let free = ctx.mem().free_mem();
    ctx.term().writeln("");
    ctx.term().write("  Memory used:  ");
    let used = ctx.mem().used();
    ctx.term().writeln("");

    ctx.term().writeln("");
    ctx.term().set_fg(0x0C);
    ctx.term().writeln("GPU active:");
    ctx.term().set_fg(0x07);
    if ctx.gpu().is_active() {
        ctx.term().set_fg(0x0A);
        ctx.term().writeln("  YES - Hardware acceleration enabled");
    } else {
        ctx.term().set_fg(0x08);
        ctx.term().writeln("  NO  - Using software rendering");
    }

    ctx.term().writeln("");
    ctx.term().set_fg(0x0D);
    ctx.term().writeln("Running tasks:");
    ctx.term().set_fg(0x07);
    let task_count = ctx.sched().count();
    ctx.term().write("  Total: ");
    ctx.term().writeln("");

    ctx.term().writeln("");
    ctx.term().set_fg(0x0E);
    ctx.term().writeln("Press any key to exit...");
    ctx.kbd().read_key_blocking();

    ctx.term().clear(0x000000);
}
