//! SID 6581 audio emulation.
//!
//! Line-accurate: oscillators are clocked once per CPU cycle; samples are
//! decimated to 48 kHz (or the configured rate) via a fractional accumulator.

pub mod envelope;
pub mod filter;
pub mod voice;

use crate::config::Model;
use envelope::Envelope;
use voice::Voice;

/// SID chip state.
pub struct Sid {
    voices: [Voice; 3],
    envelopes: [Envelope; 3],
    /// $D418: low 4 bits = master volume, high 4 bits = filter mode + FIL3OFF.
    pub mode_vol: u8,
    /// $D415 / $D416: filter cutoff low (3 bits) / high (8 bits).
    pub fcut_lo: u8,
    /// $D415 / $D416: filter cutoff high.
    pub fcut_hi: u8,
    /// $D417: filter resonance (high nibble) + filter-enable (low nibble).
    pub res_filt: u8,
    /// Audio sample accumulator (fractional).
    accum: f64,
    /// Cycles per one output sample.
    cycles_per_sample: f64,
    /// Output ring buffer (drained by the host).
    samples: Vec<i16>,
    /// Filter state.
    filter: filter::Biquad,
    cpu_hz: f64,
    sample_rate: u32,
}

impl Sid {
    /// Create a SID for the given model at a given output sample rate.
    pub fn new(model: Model, sample_rate: u32) -> Self {
        let hz = model.cpu_hz() as f64;
        let sr = sample_rate.max(8000) as f64;
        Sid {
            voices: [Voice::new(); 3],
            envelopes: [Envelope::new(); 3],
            mode_vol: 0,
            fcut_lo: 0, fcut_hi: 0, res_filt: 0,
            accum: 0.0,
            cycles_per_sample: hz / sr,
            samples: Vec::with_capacity(4096),
            filter: filter::Biquad::new(),
            cpu_hz: hz,
            sample_rate,
        }
    }

    /// Write to a SID register (address masked to $D400-$D41F range).
    pub fn write(&mut self, addr: u16, val: u8) {
        let reg = (addr & 0x1F) as u8;
        match reg {
            0x00..=0x06 | 0x07..=0x0D | 0x0E..=0x14 => {
                let v = (reg / 7) as usize;
                let r = reg % 7;
                match r {
                    0 => self.voices[v].freq = (self.voices[v].freq & 0xFF00) | val as u16,
                    1 => self.voices[v].freq = (self.voices[v].freq & 0x00FF) | ((val as u16) << 8),
                    2 => self.voices[v].pw = (self.voices[v].pw & 0x0F00) | val as u16,
                    3 => self.voices[v].pw = (self.voices[v].pw & 0x00FF) | (((val & 0x0F) as u16) << 8),
                    4 => {
                        self.voices[v].control = val;
                        self.envelopes[v].gate(val & 0x01 != 0);
                    }
                    5 => self.envelopes[v].set_ad(val),
                    6 => self.envelopes[v].set_sr(val),
                    _ => {}
                }
            }
            0x15 => { self.fcut_lo = val & 0x07; self.update_filter(); }
            0x16 => { self.fcut_hi = val; self.update_filter(); }
            0x17 => { self.res_filt = val; self.update_filter(); }
            0x18 => self.mode_vol = val,
            _ => {}
        }
    }

    /// Read a SID register. Most registers are write-only; reads return 0.
    pub fn read(&self, addr: u16) -> u8 {
        let reg = (addr & 0x1F) as u8;
        match reg {
            0x19 | 0x1A => 0xFF, // POTX / POTY (paddles not wired)
            _ => 0x00,
        }
    }

    fn update_filter(&mut self) {
        let cut_raw = ((self.fcut_hi as u32) << 3) | (self.fcut_lo & 0x07) as u32;
        let cutoff = (cut_raw as f64 / 2047.0) * 12000.0 + 30.0;
        let q = 0.7 + (self.res_filt >> 4) as f64 / 15.0 * 3.3;
        let mode = (self.mode_vol >> 4) & 0x07;
        self.filter.setup(cutoff, q, mode, self.sample_rate as f64);
    }

    /// Advance the SID by `cycles` CPU cycles, emitting samples to the ring.
    pub fn clock(&mut self, cycles: u32) {
        for _ in 0..cycles {
            for v in 0..3 {
                self.voices[v].step();
                self.envelopes[v].step(self.cpu_hz);
            }
            self.accum += 1.0;
            if self.accum >= self.cycles_per_sample {
                self.accum -= self.cycles_per_sample;
                let s = self.produce_sample();
                self.samples.push(s);
            }
        }
    }

    fn produce_sample(&mut self) -> i16 {
        let filt_enable = self.res_filt & 0x0F;
        let mut filt_in: i32 = 0;
        let mut dry: i32 = 0;

        for v in 0..3 {
            let osc = self.voices[v].output() as i32 - 0x800; // signed 12-bit
            let env = self.envelopes[v].value() as i32;
            let s = osc * env / 255;
            if (filt_enable >> v) & 1 != 0 {
                filt_in += s;
            } else {
                dry += s;
            }
        }

        let fmode = (self.mode_vol >> 4) & 0x07;
        let mixed: i32 = if fmode != 0 {
            dry + self.filter.process(filt_in)
        } else {
            dry + filt_in
        };

        let vol = (self.mode_vol & 0x0F) as i32;
        let out = mixed * vol / 15;
        out.clamp(-32768, 32767) as i16
    }

    /// Drain accumulated samples.
    pub fn drain(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.samples)
    }
}
