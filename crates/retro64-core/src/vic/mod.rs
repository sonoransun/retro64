//! VIC-II video chip emulation.

pub mod export;
pub mod palette;
pub mod sprites;

use crate::config::Model;
use crate::memory::Memory;
use palette::VIC_PALETTE;

/// VIC-II state and framebuffer.
pub struct VicII {
    model: Model,

    /// Cycle/raster position within current line.
    pub raster_x: u32,
    /// Raster line (0..lines_per_frame).
    pub raster_y: u32,
    /// Raster line that fires IRQ (9-bit).
    pub raster_cmp: u32,

    /// $D011 control register 1.
    pub cr1: u8,
    /// $D016 control register 2.
    pub cr2: u8,
    /// $D018 memory pointer register.
    pub memptr: u8,
    /// $D019 interrupt latch ("any" bit plus four sources).
    pub irq_latch: u8,
    /// $D01A interrupt mask.
    pub irq_mask: u8,
    /// $D020 border color.
    pub border: u8,
    /// $D021-$D024 background colors.
    pub bg: [u8; 4],

    /// Sprite X (low 8 bits each) and X MSB packed in `sx_msb`.
    pub sprite_x: [u8; 8],
    /// Sprite Y position.
    pub sprite_y: [u8; 8],
    /// Sprite X MSB ($D010).
    pub sx_msb: u8,
    /// Sprite enable ($D015).
    pub sprite_enable: u8,
    /// Sprite multicolor enable ($D01C).
    pub sprite_mc: u8,
    /// Sprite X expand ($D01D).
    pub sprite_xexp: u8,
    /// Sprite Y expand ($D017).
    pub sprite_yexp: u8,
    /// Sprite priority behind background ($D01B).
    pub sprite_prio: u8,
    /// Sprite colors ($D027-$D02E).
    pub sprite_color: [u8; 8],
    /// Sprite multicolors MM0/MM1 ($D025/$D026).
    pub sprite_mm: [u8; 2],
    /// Sprite-sprite collision latch ($D01E).
    pub ss_collide: u8,
    /// Sprite-background collision latch ($D01F).
    pub sb_collide: u8,

    /// Computed framebuffer (ARGB8888).
    pub fb: Vec<u32>,
    /// Framebuffer dimensions.
    pub width: u32,
    /// Framebuffer dimensions.
    pub height: u32,
    /// Latched "frame complete" signal.
    pub frame_done: bool,

    /// Current IRQ line (latched from mask ∧ sources).
    pub irq_line: bool,
}

impl VicII {
    /// Create a fresh VIC-II.
    pub fn new(model: Model) -> Self {
        let w = model.screen_width();
        let h = model.screen_height();
        let mut v = VicII {
            model,
            raster_x: 0, raster_y: 0, raster_cmp: 0,
            cr1: 0x1B, cr2: 0xC8, memptr: 0x14, irq_latch: 0, irq_mask: 0,
            border: 14, bg: [6, 0, 0, 0],
            sprite_x: [0; 8], sprite_y: [0; 8],
            sx_msb: 0, sprite_enable: 0, sprite_mc: 0, sprite_xexp: 0, sprite_yexp: 0,
            sprite_prio: 0, sprite_color: [0; 8], sprite_mm: [0; 2],
            ss_collide: 0, sb_collide: 0,
            fb: vec![0; (w * h) as usize],
            width: w, height: h,
            frame_done: false,
            irq_line: false,
        };
        v.clear_framebuffer();
        v
    }

    /// Fill framebuffer with border colour.
    pub fn clear_framebuffer(&mut self) {
        let c = VIC_PALETTE[self.border as usize & 15];
        for p in &mut self.fb { *p = c; }
    }

    /// Read a VIC-II register. `addr` is any address in $D000-$D3FF.
    pub fn read(&mut self, addr: u16) -> u8 {
        let reg = (addr & 0x3F) as u8;
        match reg {
            0x00..=0x0F => if reg & 1 == 0 { self.sprite_x[(reg/2) as usize] } else { self.sprite_y[(reg/2) as usize] },
            0x10 => self.sx_msb,
            0x11 => (self.cr1 & 0x7F) | ((self.raster_y as u8 & 0x80) >> 0 & 0x80),
            0x12 => self.raster_y as u8,
            0x13 | 0x14 => 0, // light pen
            0x15 => self.sprite_enable,
            0x16 => self.cr2,
            0x17 => self.sprite_yexp,
            0x18 => self.memptr | 0x01,
            0x19 => self.irq_latch | 0x70,
            0x1A => self.irq_mask | 0xF0,
            0x1B => self.sprite_prio,
            0x1C => self.sprite_mc,
            0x1D => self.sprite_xexp,
            0x1E => { let v = self.ss_collide; self.ss_collide = 0; v }
            0x1F => { let v = self.sb_collide; self.sb_collide = 0; v }
            0x20 => self.border | 0xF0,
            0x21..=0x24 => self.bg[(reg - 0x21) as usize] | 0xF0,
            0x25 => self.sprite_mm[0] | 0xF0,
            0x26 => self.sprite_mm[1] | 0xF0,
            0x27..=0x2E => self.sprite_color[(reg - 0x27) as usize] | 0xF0,
            _ => 0xFF,
        }
    }

    /// Write a VIC-II register.
    pub fn write(&mut self, addr: u16, val: u8) {
        let reg = (addr & 0x3F) as u8;
        match reg {
            0x00..=0x0F => {
                if reg & 1 == 0 { self.sprite_x[(reg/2) as usize] = val; }
                else { self.sprite_y[(reg/2) as usize] = val; }
            }
            0x10 => self.sx_msb = val,
            0x11 => {
                self.cr1 = val;
                // raster_cmp bit 8 from cr1 bit 7
                self.raster_cmp = (self.raster_cmp & 0xFF) | (((val >> 7) as u32) << 8);
            }
            0x12 => self.raster_cmp = (self.raster_cmp & 0x100) | val as u32,
            0x15 => self.sprite_enable = val,
            0x16 => self.cr2 = val,
            0x17 => self.sprite_yexp = val,
            0x18 => self.memptr = val,
            0x19 => { self.irq_latch &= !(val & 0x0F); self.update_irq(); }
            0x1A => { self.irq_mask = val & 0x0F; self.update_irq(); }
            0x1B => self.sprite_prio = val,
            0x1C => self.sprite_mc = val,
            0x1D => self.sprite_xexp = val,
            0x20 => self.border = val & 0x0F,
            0x21..=0x24 => self.bg[(reg - 0x21) as usize] = val & 0x0F,
            0x25 => self.sprite_mm[0] = val & 0x0F,
            0x26 => self.sprite_mm[1] = val & 0x0F,
            0x27..=0x2E => self.sprite_color[(reg - 0x27) as usize] = val & 0x0F,
            _ => {}
        }
    }

    fn update_irq(&mut self) {
        let any = (self.irq_latch & self.irq_mask) & 0x0F;
        if any != 0 {
            self.irq_latch |= 0x80;
            self.irq_line = true;
        } else {
            self.irq_latch &= !0x80;
            self.irq_line = false;
        }
    }

    /// True if display is enabled (DEN bit of $D011).
    #[inline] pub fn den(&self) -> bool { self.cr1 & 0x10 != 0 }
    /// ECM graphics bit.
    #[inline] pub fn ecm(&self) -> bool { self.cr1 & 0x40 != 0 }
    /// BMM bitmap-mode bit.
    #[inline] pub fn bmm(&self) -> bool { self.cr1 & 0x20 != 0 }
    /// MCM multicolor bit.
    #[inline] pub fn mcm(&self) -> bool { self.cr2 & 0x10 != 0 }

    /// Advance the raster by one scanline, rendering if needed.
    pub fn step_line(&mut self, mem: &Memory) {
        // Raster IRQ match at the START of the line.
        if self.raster_y == self.raster_cmp {
            self.irq_latch |= 0x01;
            self.update_irq();
        }

        if self.raster_y < self.height {
            self.render_line(mem);
        }

        self.raster_y += 1;
        if self.raster_y >= self.model.lines_per_frame() {
            self.raster_y = 0;
            self.frame_done = true;
        }
    }

    fn render_line(&mut self, mem: &Memory) {
        let y = self.raster_y;
        let w = self.width as usize;
        let row_off = (y as usize) * w;

        // Border colour for left/right borders and off-display lines.
        let border = VIC_PALETTE[self.border as usize & 15];
        for x in 0..w { self.fb[row_off + x] = border; }

        if !self.den() { return; }

        // Display area is roughly 40x25 chars centered in the frame. For
        // simplicity we use a fixed inner window starting at (x0,y0).
        let x0: u32 = match self.model { Model::Pal => 42, Model::Ntsc => 46 };
        let y0: u32 = match self.model { Model::Pal => 42, Model::Ntsc => 20 };
        let yrel = y.wrapping_sub(y0);
        if yrel >= 200 { return; }
        let char_row = yrel / 8;
        let fine = (yrel & 7) as u8;

        let screen_base = ((self.memptr & 0xF0) as u16) << 6;
        let char_base = ((self.memptr & 0x0E) as u16) << 10;
        let bitmap_base = ((self.memptr & 0x08) as u16) << 10;

        let mut fg_mask = [false; 320];

        for col in 0..40u32 {
            let cell = char_row * 40 + col;
            let screen_code = mem.vic_read(screen_base + cell as u16);
            let color = mem.color_read(cell as u16 + 0x0400);

            if self.bmm() {
                // Bitmap mode
                let row_byte = mem.vic_read(bitmap_base + (cell * 8) as u16 + fine as u16);
                if self.mcm() {
                    self.render_mcm_bitmap_cell(
                        row_off, x0 + col * 8, row_byte, screen_code, color, &mut fg_mask, col);
                } else {
                    self.render_std_bitmap_cell(
                        row_off, x0 + col * 8, row_byte, screen_code, &mut fg_mask, col);
                }
            } else {
                let glyph_addr = char_base + (screen_code as u16) * 8 + fine as u16;
                let row_byte = mem.vic_read(glyph_addr);
                if self.mcm() && (color & 0x08) != 0 {
                    self.render_mcm_text_cell(row_off, x0 + col * 8, row_byte, color & 0x07, &mut fg_mask, col);
                } else if self.ecm() {
                    self.render_ecm_text_cell(row_off, x0 + col * 8, row_byte, screen_code, color & 0x0F, &mut fg_mask, col);
                } else {
                    self.render_std_text_cell(row_off, x0 + col * 8, row_byte, color & 0x0F, &mut fg_mask, col);
                }
            }
        }

        // Sprite pass on top of background
        sprites::render_sprites(self, mem, row_off, x0, &fg_mask);
    }

    fn render_std_text_cell(
        &mut self, row_off: usize, x_start: u32, row_byte: u8, fg_color: u8,
        fg_mask: &mut [bool; 320], col: u32)
    {
        let bg = VIC_PALETTE[self.bg[0] as usize & 15];
        let fg = VIC_PALETTE[fg_color as usize & 15];
        for bit in 0..8u32 {
            let px = (row_byte >> (7 - bit)) & 1 != 0;
            let x = x_start + bit;
            if (x as usize) < self.fb.len() - row_off {
                self.fb[row_off + x as usize] = if px { fg } else { bg };
            }
            let fgx = (col * 8 + bit) as usize;
            if fgx < 320 { fg_mask[fgx] = px; }
        }
    }

    fn render_mcm_text_cell(
        &mut self, row_off: usize, x_start: u32, row_byte: u8, fg_color: u8,
        fg_mask: &mut [bool; 320], col: u32)
    {
        let cols = [
            VIC_PALETTE[self.bg[0] as usize & 15],
            VIC_PALETTE[self.bg[1] as usize & 15],
            VIC_PALETTE[self.bg[2] as usize & 15],
            VIC_PALETTE[fg_color as usize & 15],
        ];
        for i in 0..4u32 {
            let bits = ((row_byte >> (6 - i * 2)) & 0x03) as usize;
            let c = cols[bits];
            let x = x_start + i * 2;
            if (x as usize + 1) < self.fb.len() - row_off {
                self.fb[row_off + x as usize] = c;
                self.fb[row_off + x as usize + 1] = c;
            }
            let fgx = (col * 8 + i * 2) as usize;
            if fgx + 1 < 320 {
                let on = bits & 0x02 != 0;
                fg_mask[fgx] = on;
                fg_mask[fgx + 1] = on;
            }
        }
    }

    fn render_ecm_text_cell(
        &mut self, row_off: usize, x_start: u32, row_byte: u8, screen_code: u8, fg_color: u8,
        fg_mask: &mut [bool; 320], col: u32)
    {
        let bg_idx = ((screen_code >> 6) & 0x03) as usize;
        let bg = VIC_PALETTE[self.bg[bg_idx] as usize & 15];
        let fg = VIC_PALETTE[fg_color as usize & 15];
        for bit in 0..8u32 {
            let px = (row_byte >> (7 - bit)) & 1 != 0;
            let x = x_start + bit;
            if (x as usize) < self.fb.len() - row_off {
                self.fb[row_off + x as usize] = if px { fg } else { bg };
            }
            let fgx = (col * 8 + bit) as usize;
            if fgx < 320 { fg_mask[fgx] = px; }
        }
    }

    fn render_std_bitmap_cell(
        &mut self, row_off: usize, x_start: u32, row_byte: u8, screen_code: u8,
        fg_mask: &mut [bool; 320], col: u32)
    {
        let bg = VIC_PALETTE[(screen_code & 0x0F) as usize];
        let fg = VIC_PALETTE[((screen_code >> 4) & 0x0F) as usize];
        for bit in 0..8u32 {
            let px = (row_byte >> (7 - bit)) & 1 != 0;
            let x = x_start + bit;
            if (x as usize) < self.fb.len() - row_off {
                self.fb[row_off + x as usize] = if px { fg } else { bg };
            }
            let fgx = (col * 8 + bit) as usize;
            if fgx < 320 { fg_mask[fgx] = px; }
        }
    }

    fn render_mcm_bitmap_cell(
        &mut self, row_off: usize, x_start: u32, row_byte: u8,
        screen_code: u8, color_ram: u8,
        fg_mask: &mut [bool; 320], col: u32)
    {
        let cols = [
            VIC_PALETTE[self.bg[0] as usize & 15],
            VIC_PALETTE[((screen_code >> 4) & 0x0F) as usize],
            VIC_PALETTE[(screen_code & 0x0F) as usize],
            VIC_PALETTE[(color_ram & 0x0F) as usize],
        ];
        for i in 0..4u32 {
            let bits = ((row_byte >> (6 - i * 2)) & 0x03) as usize;
            let c = cols[bits];
            let x = x_start + i * 2;
            if (x as usize + 1) < self.fb.len() - row_off {
                self.fb[row_off + x as usize] = c;
                self.fb[row_off + x as usize + 1] = c;
            }
            let fgx = (col * 8 + i * 2) as usize;
            if fgx + 1 < 320 {
                let on = bits & 0x02 != 0;
                fg_mask[fgx] = on;
                fg_mask[fgx + 1] = on;
            }
        }
    }
}
