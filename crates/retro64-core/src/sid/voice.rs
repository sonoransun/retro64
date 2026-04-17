//! SID voice oscillator (phase accumulator + waveform select).

/// One SID voice.
#[derive(Copy, Clone)]
pub struct Voice {
    /// 24-bit phase accumulator.
    pub phase: u32,
    /// 16-bit frequency step per CPU cycle.
    pub freq: u16,
    /// 12-bit pulse width threshold.
    pub pw: u16,
    /// Voice control register bits (GATE, SYNC, RING, TEST, W_TRI, W_SAW, W_PUL, W_NOI).
    pub control: u8,
    /// Noise LFSR (23-bit).
    pub lfsr: u32,
}

impl Voice {
    /// Create an idle voice.
    pub const fn new() -> Self {
        Voice { phase: 0, freq: 0, pw: 0, control: 0, lfsr: 0x7FFFFF }
    }

    /// Advance phase by one CPU cycle and clock noise if pulse bit 19 toggled.
    pub fn step(&mut self) {
        if self.control & 0x08 != 0 {
            // TEST bit: phase locked at 0
            self.phase = 0;
            return;
        }
        let prev = self.phase;
        self.phase = (self.phase + self.freq as u32) & 0xFFFFFF;
        // Noise LFSR clocks on rising edge of bit 19
        if (prev & 0x080000) == 0 && (self.phase & 0x080000) != 0 {
            let feed = ((self.lfsr >> 22) ^ (self.lfsr >> 17)) & 1;
            self.lfsr = ((self.lfsr << 1) & 0x7FFFFF) | feed;
        }
    }

    /// Current waveform output (12-bit unsigned, centered at 0x800).
    pub fn output(&self) -> u16 {
        let mut out: u16 = 0xFFF;
        let waveforms = self.control >> 4;
        if waveforms & 0x01 != 0 {
            // Triangle: fold top half of phase
            let p = if self.phase & 0x800000 != 0 { !self.phase } else { self.phase };
            out &= ((p >> 11) & 0xFFF) as u16;
        }
        if waveforms & 0x02 != 0 {
            // Sawtooth: high 12 bits of phase
            out &= (self.phase >> 12) as u16 & 0xFFF;
        }
        if waveforms & 0x04 != 0 {
            // Pulse
            let p = (self.phase >> 12) as u16;
            let on = p >= self.pw;
            out &= if on { 0xFFF } else { 0x000 };
        }
        if waveforms & 0x08 != 0 {
            // Noise: pick 12 bits out of LFSR
            let n = self.lfsr;
            let nb = ((n & 0x400000) >> 11) | ((n & 0x100000) >> 10)
                | ((n & 0x010000) >> 7)  | ((n & 0x002000) >> 5)
                | ((n & 0x000800) >> 4)  | ((n & 0x000080) >> 1)
                | ((n & 0x000010) << 1)  | ((n & 0x000004) << 2);
            out &= nb as u16 & 0xFFF;
        }
        if waveforms == 0 { 0x800 } else { out }
    }
}
