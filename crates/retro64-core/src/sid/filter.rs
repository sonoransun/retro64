//! RBJ biquad filter used as a simplified stand-in for the real SID filter.

/// A direct-form biquad with three output modes.
pub struct Biquad {
    b0: f64, b1: f64, b2: f64,
    a1: f64, a2: f64,
    x1: f64, x2: f64,
    y1: f64, y2: f64,
    mode: u8,
}

impl Biquad {
    /// New pass-through filter.
    pub fn new() -> Self {
        Biquad { b0: 1.0, b1: 0.0, b2: 0.0, a1: 0.0, a2: 0.0,
                 x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0, mode: 0 }
    }

    /// Configure cutoff/resonance/mode (LP=1, BP=2, HP=4 from $D418 high bits).
    pub fn setup(&mut self, freq: f64, q: f64, mode: u8, sr: f64) {
        let w0 = 2.0 * std::f64::consts::PI * freq.min(sr * 0.49) / sr;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q.max(0.5));
        let a0 = 1.0 + alpha;
        self.mode = mode;
        match mode & 0x07 {
            0x01 => { // LP
                self.b0 = (1.0 - cos_w0) * 0.5 / a0;
                self.b1 = (1.0 - cos_w0) / a0;
                self.b2 = (1.0 - cos_w0) * 0.5 / a0;
                self.a1 = -2.0 * cos_w0 / a0;
                self.a2 = (1.0 - alpha) / a0;
            }
            0x02 => { // BP
                self.b0 = alpha / a0;
                self.b1 = 0.0;
                self.b2 = -alpha / a0;
                self.a1 = -2.0 * cos_w0 / a0;
                self.a2 = (1.0 - alpha) / a0;
            }
            0x04 => { // HP
                self.b0 = (1.0 + cos_w0) * 0.5 / a0;
                self.b1 = -(1.0 + cos_w0) / a0;
                self.b2 = (1.0 + cos_w0) * 0.5 / a0;
                self.a1 = -2.0 * cos_w0 / a0;
                self.a2 = (1.0 - alpha) / a0;
            }
            _ => { // pass-through
                self.b0 = 1.0; self.b1 = 0.0; self.b2 = 0.0;
                self.a1 = 0.0; self.a2 = 0.0;
            }
        }
    }

    /// Process one sample.
    pub fn process(&mut self, x: i32) -> i32 {
        let xf = x as f64;
        let y = self.b0 * xf + self.b1 * self.x1 + self.b2 * self.x2
              - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1; self.x1 = xf;
        self.y2 = self.y1; self.y1 = y;
        y as i32
    }
}
