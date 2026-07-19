/*
 * LumieOS SDK - Graphics Functions
 * 
 * Framebuffer and GPU rendering for LumieC programs.
 */

#ifndef LUMIE_OS_GFX_H
#define LUMIE_OS_GFX_H

#include "lumie_os.h"

/* Draw a pixel at (x, y) with color */
void draw_pixel(i64 x, i64 y, u32 color) {
    syscall3(SYS_DRAW_PIXEL, x, y, (i64)color);
}

/* Draw a filled rectangle */
void draw_rect(i64 x, i64 y, i64 w, i64 h, u32 color) {
    i64 dx, dy;
    for (dy = 0; dy < h; dy++) {
        for (dx = 0; dx < w; dx++) {
            draw_pixel(x + dx, y + dy, color);
        }
    }
}

/* Draw a horizontal line */
void draw_hline(i64 x, i64 y, i64 w, u32 color) {
    draw_rect(x, y, w, 1, color);
}

/* Draw a vertical line */
void draw_vline(i64 x, i64 y, i64 h, u32 color) {
    draw_rect(x, y, 1, h, color);
}

/* Draw a rectangle outline */
void draw_rect_outline(i64 x, i64 y, i64 w, i64 h, u32 color) {
    draw_hline(x, y, w, color);
    draw_hline(x, y + h - 1, w, color);
    draw_vline(x, y, h, color);
    draw_vline(x + w - 1, y, h, color);
}

/* Draw a circle (approximate with pixels) */
void draw_circle(i64 cx, i64 cy, i64 r, u32 color) {
    i64 x, y;
    for (y = -r; y <= r; y++) {
        for (x = -r; x <= r; x++) {
            if (x * x + y * y <= r * r) {
                draw_pixel(cx + x, cy + y, color);
            }
        }
    }
}

/* Create RGB color value */
u32 make_color(u8 r, u8 g, u8 b) {
    return ((u32)r << 16) | ((u32)g << 8) | (u32)b;
}

/* Common colors */
#define CLR_WHITE   make_color(255, 255, 255)
#define CLR_BLACK   make_color(0, 0, 0)
#define CLR_RED     make_color(255, 0, 0)
#define CLR_GREEN   make_color(0, 255, 0)
#define CLR_BLUE    make_color(0, 0, 255)
#define CLR_YELLOW  make_color(255, 255, 0)
#define CLR_CYAN    make_color(0, 255, 255)
#define CLR_MAGENTA make_color(255, 0, 255)
#define CLR_ORANGE  make_color(255, 165, 0)
#define CLR_GRAY    make_color(128, 128, 128)
#define CLR_DKGRAY  make_color(64, 64, 64)

#endif /* LUMIE_OS_GFX_H */
