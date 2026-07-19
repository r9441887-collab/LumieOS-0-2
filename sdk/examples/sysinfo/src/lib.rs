#![no_std]

extern crate lumie_sdk;

use lumie_sdk::AppContext;

lumie_sdk::lumie_app_entry!(sysinfo_main);

fn sysinfo_main(ctx: &AppContext) {
    ctx.term().clear(0x000020);
    ctx.term().set_fg(0x0F);
    ctx.term().writeln("╔══════════════════════════════════════╗");
    ctx.term().writeln("║       LumieOS System Information     ║");
    ctx.term().writeln("╚══════════════════════════════════════╝");
    ctx.term().writeln("");

    // Memory
    ctx.term().set_fg(0x0B);
    ctx.term().writeln("[Memory]");
    ctx.term().set_fg(0x07);
    ctx.term().write("  Total: ");
    ctx.term().writeln("");
    ctx.term().write("  Free:  ");
    ctx.term().writeln("");
    ctx.term().write("  Used:  ");
    ctx.term().writeln("");
    ctx.term().writeln("");

    // Disk
    ctx.term().set_fg(0x0B);
    ctx.term().writeln("[Disks]");
    ctx.term().set_fg(0x07);
    let disk_count = ctx.sys().disk_count();
    ctx.term().write("  Count: ");
    ctx.term().writeln("");
    ctx.term().writeln("");

    // Scheduler
    ctx.term().set_fg(0x0B);
    ctx.term().writeln("[Scheduler]");
    ctx.term().set_fg(0x07);
    let task_count = ctx.sched().count();
    ctx.term().write("  Tasks: ");
    ctx.term().writeln("");

    // PCI Devices
    ctx.term().set_fg(0x0B);
    ctx.term().writeln("[PCI Devices]");
    ctx.term().set_fg(0x07);
    let mut pci_idx = 0;
    loop {
        let (vendor, device, class) = ctx.sys().pci_scan(pci_idx);
        if vendor == 0 && device == 0 {
            break;
        }
        ctx.term().write("  [");
        ctx.term().write("");
        ctx.term().write("] Vendor:");
        ctx.term().write("");
        ctx.term().write(" Device:");
        ctx.term().writeln("");
        pci_idx += 1;
        if pci_idx > 20 {
            break;
        }
    }
    ctx.term().writeln("");

    ctx.term().set_fg(0x0E);
    ctx.term().writeln("Press any key to exit...");
    ctx.kbd().read_key_blocking();
    ctx.term().clear(0x000000);
}
