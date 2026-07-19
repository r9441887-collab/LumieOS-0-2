#![no_std]

extern crate lumie_sdk;

use lumie_sdk::AppContext;

lumie_sdk::lumie_app_entry!(gpu_demo_main);

fn gpu_demo_main(ctx: &AppContext) {
    let gpu = ctx.gpu();
    let kbd = ctx.kbd();

    // Clear screen
    gpu.fill_rect(0, 0, 1024, 768, 0x000000);

    // Draw title
    ctx.term().set_fg(0x0F);
    ctx.term().set_pos(0, 0);
    ctx.term().write("GPU Demo - Press any key to exit");

    // Color gradient (red channel)
    for x in 0..256u32 {
        for y in 0..256u32 {
            let color = (x << 16) | (y << 8) | 0;
            gpu.put_pixel(50 + x, 50 + y, color);
        }
    }

    // Color gradient (green channel)
    for x in 0..256u32 {
        for y in 0..256u32 {
            let color = (x << 8) | 0x00FF00;
            gpu.put_pixel(320 + x, 50 + y, color);
        }
    }

    // Color gradient (blue channel)
    for x in 0..256u32 {
        for y in 0..256u32 {
            let color = (y << 16) | (x << 8) | 0x0000FF;
            gpu.put_pixel(590 + x, 50 + y, color);
        }
    }

    // Draw some rectangles
    gpu.fill_rect(50, 330, 200, 100, 0xFF0000); // Red
    gpu.fill_rect(270, 330, 200, 100, 0x00FF00); // Green
    gpu.fill_rect(490, 330, 200, 100, 0x0000FF); // Blue
    gpu.fill_rect(710, 330, 200, 100, 0xFFFF00); // Yellow

    // Draw outlined rectangles
    gpu.draw_rect_outline(50, 460, 200, 100, 0xFF00FF); // Magenta
    gpu.draw_rect_outline(270, 460, 200, 100, 0x00FFFF); // Cyan
    gpu.draw_rect_outline(490, 460, 200, 100, 0xFFFFFF); // White
    gpu.draw_rect_outline(710, 460, 200, 100, 0xFF8800); // Orange

    // Draw a gradient circle (approximate)
    let cx = 150u32;
    let cy = 620u32;
    let r = 60u32;
    for y in 0..(r * 2) {
        for x in 0..(r * 2) {
            let dx = if x > r { x - r } else { r - x };
            let dy = if y > r { y - r } else { r - y };
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= r * r {
                let intensity = (255 - (dist_sq * 255 / (r * r))) as u32;
                let color = (intensity << 16) | (0 << 8) | (255 - intensity);
                gpu.put_pixel(cx - r + x, cy - r + y, color);
            }
        }
    }

    // Wait for key
    kbd.read_key_blocking();

    gpu.fill_rect(0, 0, 1024, 768, 0x000000);
}
