//! Compute-offload registers at $DE00-$DE2F.

/// Compute-offload state.
#[derive(Default, Clone)]
pub struct Compute {
    a: u32,
    b: u32,
    result: u32,
    status: u8,
    fill_addr: u16,
    fill_val: u8,
    fill_len: u16,
    copy_src: u16,
    copy_dst: u16,
    copy_len: u16,
    rng_state: u32,
}

/// Op codes accepted at $DE08.
#[derive(Copy, Clone, Debug)]
pub enum Op {
    /// result = a * b.
    Mul = 0,
    /// result = a / b (division by zero → 0).
    Div = 1,
    /// result = a mod b.
    Mod = 2,
    /// result = sqrt(a).
    Sqrt = 3,
    /// result = sin(a/256 radians) * 256.
    Sin = 4,
    /// result = cos(a/256 radians) * 256.
    Cos = 5,
    /// result = random u32 (LCG-stepped).
    Rng = 6,
    /// result = a + b.
    Add = 7,
}

impl Compute {
    /// Create a new Compute with a seeded RNG.
    pub fn new() -> Self {
        Compute { rng_state: 0x12345678, ..Default::default() }
    }

    /// Read register at absolute address $DE00-$DEFF.
    pub fn read(&self, addr: u16) -> u8 {
        let reg = (addr & 0xFF) as u8;
        match reg {
            0x00..=0x03 => byte(self.a, reg & 3),
            0x04..=0x07 => byte(self.b, reg & 3),
            0x09 => self.status,
            0x0A..=0x0D => byte(self.result, reg & 3),
            _ => 0,
        }
    }

    /// Write register at absolute address.
    pub fn write(&mut self, addr: u16, v: u8, ram: &mut [u8; 0x1_0000]) {
        let reg = (addr & 0xFF) as u8;
        match reg {
            0x00..=0x03 => self.a = set_byte(self.a, reg & 3, v),
            0x04..=0x07 => self.b = set_byte(self.b, reg & 3, v),
            0x08 => { self.execute_op(v); }
            0x10 => self.fill_addr = (self.fill_addr & 0xFF00) | v as u16,
            0x11 => self.fill_addr = (self.fill_addr & 0x00FF) | ((v as u16) << 8),
            0x12 => self.fill_val = v,
            0x13 => self.fill_len = (self.fill_len & 0xFF00) | v as u16,
            0x14 => self.fill_len = (self.fill_len & 0x00FF) | ((v as u16) << 8),
            0x15 => {
                // Trigger fill
                for i in 0..self.fill_len {
                    let a = self.fill_addr.wrapping_add(i) as usize;
                    ram[a] = self.fill_val;
                }
                self.status = 1;
            }
            0x20 => self.copy_src = (self.copy_src & 0xFF00) | v as u16,
            0x21 => self.copy_src = (self.copy_src & 0x00FF) | ((v as u16) << 8),
            0x22 => self.copy_dst = (self.copy_dst & 0xFF00) | v as u16,
            0x23 => self.copy_dst = (self.copy_dst & 0x00FF) | ((v as u16) << 8),
            0x24 => self.copy_len = (self.copy_len & 0xFF00) | v as u16,
            0x25 => self.copy_len = (self.copy_len & 0x00FF) | ((v as u16) << 8),
            0x26 => {
                for i in 0..self.copy_len {
                    let s = self.copy_src.wrapping_add(i) as usize;
                    let d = self.copy_dst.wrapping_add(i) as usize;
                    ram[d] = ram[s];
                }
                self.status = 1;
            }
            _ => {}
        }
    }

    fn execute_op(&mut self, code: u8) {
        match code {
            0 => self.result = self.a.wrapping_mul(self.b),
            1 => self.result = if self.b == 0 { 0 } else { self.a / self.b },
            2 => self.result = if self.b == 0 { 0 } else { self.a % self.b },
            3 => self.result = (self.a as f64).sqrt() as u32,
            4 => {
                let rad = (self.a as f64) / 256.0;
                self.result = (rad.sin() * 256.0) as i32 as u32;
            }
            5 => {
                let rad = (self.a as f64) / 256.0;
                self.result = (rad.cos() * 256.0) as i32 as u32;
            }
            6 => {
                self.rng_state = self.rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                self.result = self.rng_state;
            }
            7 => self.result = self.a.wrapping_add(self.b),
            _ => {}
        }
        self.status = 1;
    }
}

#[inline]
fn byte(v: u32, i: u8) -> u8 { (v >> ((i & 3) * 8)) as u8 }
#[inline]
fn set_byte(v: u32, i: u8, b: u8) -> u32 {
    let shift = (i & 3) * 8;
    (v & !(0xFF << shift)) | ((b as u32) << shift)
}
