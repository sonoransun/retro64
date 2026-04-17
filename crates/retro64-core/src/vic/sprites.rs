//! Per-scanline sprite pipeline with pixel-accurate collisions.

use super::{VicII, palette::VIC_PALETTE};
use crate::memory::Memory;

/// Render all enabled sprites on the current raster line on top of an
/// already-drawn background row. `fg_mask[x]` is true where the background
/// laid a foreground pixel (used for sprite-background collision and priority).
pub fn render_sprites(
    vic: &mut VicII,
    mem: &Memory,
    row_off: usize,
    x0: u32,
    fg_mask: &[bool; 320],
) {
    let screen_base = ((vic.memptr & 0xF0) as u16) << 6;
    let sprite_ptr_base = screen_base + 0x3F8;

    // Per-pixel owner bitmask for this row (which sprite index laid a pixel).
    let mut sprite_owner = [0u8; 320];

    for s in 0..8u8 {
        let bit = 1 << s;
        if vic.sprite_enable & bit == 0 { continue; }

        let ypos = vic.sprite_y[s as usize] as u32;
        let yexp = vic.sprite_yexp & bit != 0;
        let sprite_height = if yexp { 42 } else { 21 };
        let yrel = vic.raster_y.wrapping_sub(ypos);
        if yrel >= sprite_height { continue; }
        let row = if yexp { yrel / 2 } else { yrel };

        let ptr = mem.vic_read(sprite_ptr_base + s as u16);
        let data_base = (ptr as u16) * 64;
        let b0 = mem.vic_read(data_base + row as u16 * 3);
        let b1 = mem.vic_read(data_base + row as u16 * 3 + 1);
        let b2 = mem.vic_read(data_base + row as u16 * 3 + 2);

        let xbase = vic.sprite_x[s as usize] as u32 + if vic.sx_msb & bit != 0 { 0x100 } else { 0 };
        let xexp = vic.sprite_xexp & bit != 0;
        let mc = vic.sprite_mc & bit != 0;
        let priority_behind = vic.sprite_prio & bit != 0;
        let color = VIC_PALETTE[vic.sprite_color[s as usize] as usize & 15];
        let mm0 = VIC_PALETTE[vic.sprite_mm[0] as usize & 15];
        let mm1 = VIC_PALETTE[vic.sprite_mm[1] as usize & 15];

        let pattern: u32 = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        let width = if xexp { 48 } else { 24 };

        for i in 0..width {
            let bit_i = if xexp { i / 2 } else { i };
            let (pixel_on, mix) = if mc {
                let pair = (pattern >> (22 - (bit_i / 2) * 2)) & 0x03;
                match pair {
                    0 => (false, 0),
                    1 => (true, 1),    // MM0
                    2 => (true, 2),    // sprite color
                    3 => (true, 3),    // MM1
                    _ => (false, 0),
                }
            } else {
                let on = (pattern >> (23 - bit_i)) & 1 != 0;
                (on, 2)
            };
            if !pixel_on { continue; }

            let screen_x = xbase.wrapping_add(i);
            if screen_x >= 320 { continue; }

            // Collision with other sprites already drawn this line
            let existing = sprite_owner[screen_x as usize];
            if existing != 0 {
                vic.ss_collide |= existing | bit;
            }
            sprite_owner[screen_x as usize] |= bit;

            // Collision with background foreground
            if fg_mask[screen_x as usize] {
                vic.sb_collide |= bit;
                if priority_behind { continue; }
            }

            let pixel_color = match mix {
                1 => mm0,
                3 => mm1,
                _ => color,
            };

            let fx = (x0 + screen_x) as usize;
            if fx < vic.fb.len() - row_off {
                vic.fb[row_off + fx] = pixel_color;
            }
        }
    }

    // Raise collision IRQs
    if vic.ss_collide != 0 { vic.irq_latch |= 0x04; }
    if vic.sb_collide != 0 { vic.irq_latch |= 0x02; }
    vic.update_irq();
}
