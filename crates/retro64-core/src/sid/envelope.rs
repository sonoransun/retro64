//! SID ADSR envelope.

/// ADSR state.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum State { Attack, Decay, Sustain, Release, Idle }

/// SID ADSR envelope generator.
#[derive(Copy, Clone)]
pub struct Envelope {
    state: State,
    value: u8,
    rate: u8,
    sustain: u8,
    attack: u8,
    decay: u8,
    release: u8,
    gate: bool,
    accum: f64,
}

// Attack rate -> seconds from 0 to peak (from SID datasheet).
const ATTACK_SECS: [f64; 16] = [
    0.002, 0.008, 0.016, 0.024, 0.038, 0.056, 0.068, 0.080,
    0.100, 0.250, 0.500, 0.800, 1.000, 3.000, 5.000, 8.000,
];
// Decay/Release 36-step rates: 3x attack rate.

impl Envelope {
    /// Create an idle envelope.
    pub const fn new() -> Self {
        Envelope {
            state: State::Idle,
            value: 0,
            rate: 0,
            sustain: 0,
            attack: 0,
            decay: 0,
            release: 0,
            gate: false,
            accum: 0.0,
        }
    }

    /// Set attack (high nibble) and decay (low nibble) from $Dx05.
    pub fn set_ad(&mut self, v: u8) {
        self.attack = v >> 4;
        self.decay = v & 0x0F;
    }

    /// Set sustain (high nibble) and release (low nibble) from $Dx06.
    pub fn set_sr(&mut self, v: u8) {
        self.sustain = v >> 4;
        self.release = v & 0x0F;
    }

    /// Gate bit transition.
    pub fn gate(&mut self, on: bool) {
        if on && !self.gate {
            self.state = State::Attack;
        } else if !on && self.gate {
            self.state = State::Release;
        }
        self.gate = on;
    }

    /// Step one CPU cycle. `cpu_hz` is the clock rate in Hz.
    pub fn step(&mut self, cpu_hz: f64) {
        let _ = self.rate;
        let secs_per_cycle = 1.0 / cpu_hz;
        match self.state {
            State::Attack => {
                self.accum += secs_per_cycle / ATTACK_SECS[self.attack as usize].max(0.0001);
                if self.accum >= 1.0 / 255.0 {
                    self.accum -= 1.0 / 255.0;
                    if self.value < 255 { self.value += 1; }
                    if self.value == 255 { self.state = State::Decay; self.accum = 0.0; }
                }
            }
            State::Decay => {
                self.accum += secs_per_cycle / (3.0 * ATTACK_SECS[self.decay as usize].max(0.0001));
                let sus_val = self.sustain * 0x11;
                if self.accum >= 1.0 / 255.0 {
                    self.accum -= 1.0 / 255.0;
                    if self.value > sus_val { self.value -= 1; }
                    if self.value <= sus_val { self.state = State::Sustain; }
                }
            }
            State::Sustain => {
                let sus_val = self.sustain * 0x11;
                self.value = sus_val;
            }
            State::Release => {
                self.accum += secs_per_cycle / (3.0 * ATTACK_SECS[self.release as usize].max(0.0001));
                if self.accum >= 1.0 / 255.0 {
                    self.accum -= 1.0 / 255.0;
                    if self.value > 0 { self.value -= 1; }
                    if self.value == 0 { self.state = State::Idle; }
                }
            }
            State::Idle => {}
        }
    }

    /// Current envelope level 0..255.
    pub fn value(&self) -> u8 { self.value }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gate_on_starts_attack() {
        let mut e = Envelope::new();
        e.set_ad(0x00);
        e.gate(true);
        assert_eq!(e.state, State::Attack);
    }
    #[test]
    fn gate_off_starts_release() {
        let mut e = Envelope::new();
        e.gate(true);
        e.gate(false);
        assert_eq!(e.state, State::Release);
    }
}
