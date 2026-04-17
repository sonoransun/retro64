//! CIA 6526 implementation.

use super::joystick::JoystickState;
use super::keyboard::KeyboardMatrix;

/// Distinguishes the two CIA instances (they differ in IRQ/NMI routing).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CiaIndex {
    /// CIA1 at $DC00 — drives IRQ, reads keyboard.
    Cia1,
    /// CIA2 at $DD00 — drives NMI, drives VIC bank bits.
    Cia2,
}

/// 16-bit timer.
#[derive(Default, Copy, Clone, Debug)]
pub struct Timer {
    /// Down-counter value.
    pub counter: u16,
    /// Reload latch.
    pub latch: u16,
    /// Control register bits.
    pub cr: u8,
    /// Running flag (derived from CR bit 0).
    pub running: bool,
}

impl Timer {
    fn tick(&mut self, cycles: u32) -> bool {
        if !self.running { return false; }
        let mut underflow = false;
        if cycles as u32 >= self.counter as u32 + 1 {
            underflow = true;
            self.counter = self.latch;
            if self.cr & 0x08 != 0 { self.running = false; }
        } else {
            self.counter -= cycles as u16;
        }
        underflow
    }
    fn write_cr(&mut self, v: u8) {
        if v & 0x10 != 0 { self.counter = self.latch; }
        self.running = v & 0x01 != 0;
        self.cr = v;
    }
}

/// CIA state.
pub struct Cia {
    /// Which CIA this is.
    pub index: CiaIndex,
    /// Data Port A register.
    pub pra: u8,
    /// Data Port B register.
    pub prb: u8,
    /// Data Direction A.
    pub ddra: u8,
    /// Data Direction B.
    pub ddrb: u8,
    /// Timer A.
    pub ta: Timer,
    /// Timer B.
    pub tb: Timer,
    /// Interrupt Control Register (source bits).
    pub icr: u8,
    /// Interrupt mask.
    pub imask: u8,
    /// Current IRQ/NMI line state.
    pub irq_line: bool,
    /// CIA2 low 2 bits of PA select VIC bank (inverted).
    pub vic_bank_sel: u8,
}

impl Cia {
    /// Create a fresh CIA.
    pub fn new(index: CiaIndex) -> Self {
        Cia {
            index, pra: 0xFF, prb: 0xFF, ddra: 0, ddrb: 0,
            ta: Timer::default(), tb: Timer::default(),
            icr: 0, imask: 0, irq_line: false, vic_bank_sel: 3,
        }
    }

    /// Read a register ($Dx00-$Dx0F).
    pub fn read(&mut self, reg: u8, kb: &KeyboardMatrix, joy: &JoystickState) -> u8 {
        match reg & 0x0F {
            0x00 => {
                if self.index == CiaIndex::Cia1 {
                    let joy_mask = joy.as_port();
                    let row_col = (self.pra | !self.ddra) & joy_mask;
                    row_col
                } else {
                    (self.pra & self.ddra) | (!self.ddra & 0xFF)
                }
            }
            0x01 => {
                if self.index == CiaIndex::Cia1 {
                    let col_select = (self.pra & self.ddra) | (!self.ddra);
                    kb.read_pb(col_select)
                } else {
                    (self.prb & self.ddrb) | (!self.ddrb & 0xFF)
                }
            }
            0x02 => self.ddra,
            0x03 => self.ddrb,
            0x04 => self.ta.counter as u8,
            0x05 => (self.ta.counter >> 8) as u8,
            0x06 => self.tb.counter as u8,
            0x07 => (self.tb.counter >> 8) as u8,
            0x08..=0x0B => 0,
            0x0C => 0,
            0x0D => {
                // Reading ICR latches-and-clears the source bits.
                let v = self.icr;
                self.icr = 0;
                self.irq_line = false;
                v | (if v & 0x1F != 0 { 0x80 } else { 0 })
            }
            0x0E => self.ta.cr,
            0x0F => self.tb.cr,
            _ => 0xFF,
        }
    }

    /// Write a register.
    pub fn write(&mut self, reg: u8, v: u8) {
        match reg & 0x0F {
            0x00 => {
                self.pra = v;
                if self.index == CiaIndex::Cia2 {
                    self.vic_bank_sel = (!v) & 0x03;
                }
            }
            0x01 => self.prb = v,
            0x02 => self.ddra = v,
            0x03 => self.ddrb = v,
            0x04 => self.ta.latch = (self.ta.latch & 0xFF00) | v as u16,
            0x05 => self.ta.latch = (self.ta.latch & 0x00FF) | ((v as u16) << 8),
            0x06 => self.tb.latch = (self.tb.latch & 0xFF00) | v as u16,
            0x07 => self.tb.latch = (self.tb.latch & 0x00FF) | ((v as u16) << 8),
            0x0D => {
                // Interrupt mask: bit 7 = set/clear, bits 0-4 = source bits.
                if v & 0x80 != 0 {
                    self.imask |= v & 0x1F;
                } else {
                    self.imask &= !(v & 0x1F);
                }
            }
            0x0E => self.ta.write_cr(v),
            0x0F => self.tb.write_cr(v),
            _ => {}
        }
    }

    /// Tick both timers by `cycles` CPU cycles.
    pub fn tick(&mut self, cycles: u32) {
        if self.ta.tick(cycles) {
            self.icr |= 0x01;
        }
        if self.tb.tick(cycles) {
            self.icr |= 0x02;
        }
        if self.icr & self.imask & 0x1F != 0 {
            self.icr |= 0x80;
            self.irq_line = true;
        }
    }
}
