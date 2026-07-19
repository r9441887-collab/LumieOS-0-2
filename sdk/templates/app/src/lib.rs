#![no_std]

extern crate lumie_sdk;

use lumie_sdk::AppContext;

lumie_sdk::lumie_app_entry!(my_app_main);

fn my_app_main(ctx: &AppContext) {
    ctx.term().clear(0x000030);
    ctx.term().set_fg(0x0F);
    ctx.term().writeln("=== My LumieOS App ===");
    ctx.term().writeln("");
    ctx.term().writeln("Press any key to exit...");

    loop {
        let key = ctx.kbd().getchar();
        if key != -1 {
            break;
        }
    }

    ctx.term().clear(0x000000);
}
