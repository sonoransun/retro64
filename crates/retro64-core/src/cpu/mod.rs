//! MOS 6510 CPU core.
//!
//! All 151 documented 6502 opcodes plus the widely-used undocumented ones
//! (`LAX`, `SAX`, `DCP`, `ISC`, `SLO`, `RLA`, `SRE`, `RRA`, `ANC`, `ALR`,
//! `ARR`, `SBX`, undocumented `NOP`s). BCD mode is implemented for `ADC`/`SBC`.

pub mod addressing;
pub mod bcd;
pub mod registers;

#[cfg(test)]
mod tests;

use registers::Flags;

/// Abstract memory bus the CPU talks to. Implementations dispatch I/O
/// reads/writes to the appropriate chip.
pub trait Bus {
    /// Read one byte from the bus.
    fn read(&mut self, addr: u16) -> u8;
    /// Write one byte to the bus.
    fn write(&mut self, addr: u16, val: u8);
}

/// A lightweight peek variant used for disassembly or trap checks.
pub fn peek16<B: Bus>(bus: &mut B, addr: u16) -> u16 {
    let lo = bus.read(addr) as u16;
    let hi = bus.read(addr.wrapping_add(1)) as u16;
    (hi << 8) | lo
}

/// MOS 6510 state.
pub struct Cpu {
    /// Accumulator.
    pub a: u8,
    /// X index register.
    pub x: u8,
    /// Y index register.
    pub y: u8,
    /// Stack pointer (points into $0100-$01FF).
    pub sp: u8,
    /// Program counter.
    pub pc: u16,
    /// Status flags.
    pub p: u8,
    /// Monotonic cycle count since reset.
    pub cycles: u64,

    /// IRQ line state (level-triggered while low and I flag clear).
    pub irq_line: bool,
    /// Latched NMI edge (cleared after servicing).
    pub nmi_pending: bool,
    prev_nmi: bool,

    /// If set, the next step() returns 0 cycles without executing
    /// (used by external trap handlers to single-step).
    pub jam: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu::new()
    }
}

impl Cpu {
    /// Create a CPU with zero registers and the unused/interrupt flags set.
    pub fn new() -> Self {
        Cpu {
            a: 0, x: 0, y: 0,
            sp: 0xFD, pc: 0,
            p: (Flags::U | Flags::I).bits(),
            cycles: 0,
            irq_line: false,
            nmi_pending: false,
            prev_nmi: false,
            jam: false,
        }
    }

    /// Reset the CPU: load PC from $FFFC/$FFFD, reset SP to $FD, set I flag.
    pub fn reset<B: Bus>(&mut self, bus: &mut B) {
        self.pc = peek16(bus, 0xFFFC);
        self.sp = 0xFD;
        self.p = (Flags::U | Flags::I).bits();
        self.cycles = 0;
        self.irq_line = false;
        self.nmi_pending = false;
        self.prev_nmi = false;
        self.jam = false;
    }

    /// Request an IRQ (level). Caller sets `line` true while a chip is
    /// asserting IRQ.
    pub fn set_irq(&mut self, line: bool) {
        self.irq_line = line;
    }

    /// Latch an NMI edge. Cleared once serviced.
    pub fn trigger_nmi(&mut self) {
        self.nmi_pending = true;
    }

    /// Execute one instruction (including any pending interrupt service).
    /// Returns the number of CPU cycles consumed.
    pub fn step<B: Bus>(&mut self, bus: &mut B) -> u8 {
        if self.jam {
            return 0;
        }

        // NMI is edge-triggered: fire on a rising edge of the latch.
        if self.nmi_pending && !self.prev_nmi {
            self.prev_nmi = true;
            self.service_interrupt(bus, 0xFFFA, false);
            self.nmi_pending = false;
            return 7;
        }
        if !self.nmi_pending {
            self.prev_nmi = false;
        }

        // IRQ fires when line is asserted and I flag is clear.
        if self.irq_line && !self.flag(Flags::I) {
            self.service_interrupt(bus, 0xFFFE, false);
            return 7;
        }

        let opcode = self.fetch(bus);
        self.execute(bus, opcode)
    }

    fn service_interrupt<B: Bus>(&mut self, bus: &mut B, vector: u16, brk: bool) {
        let pc = self.pc;
        self.push(bus, (pc >> 8) as u8);
        self.push(bus, pc as u8);
        let mut p = self.p | Flags::U.bits();
        if brk { p |= Flags::B.bits(); } else { p &= !Flags::B.bits(); }
        self.push(bus, p);
        self.set_flag(Flags::I, true);
        self.pc = peek16(bus, vector);
    }

    #[inline]
    fn flag(&self, f: Flags) -> bool {
        (self.p & f.bits()) != 0
    }

    #[inline]
    fn set_flag(&mut self, f: Flags, v: bool) {
        if v { self.p |= f.bits(); } else { self.p &= !f.bits(); }
    }

    #[inline]
    fn set_nz(&mut self, v: u8) {
        self.set_flag(Flags::Z, v == 0);
        self.set_flag(Flags::N, v & 0x80 != 0);
    }

    #[inline]
    fn fetch<B: Bus>(&mut self, bus: &mut B) -> u8 {
        let b = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        b
    }

    #[inline]
    fn fetch16<B: Bus>(&mut self, bus: &mut B) -> u16 {
        let lo = self.fetch(bus) as u16;
        let hi = self.fetch(bus) as u16;
        (hi << 8) | lo
    }

    #[inline]
    fn push<B: Bus>(&mut self, bus: &mut B, v: u8) {
        bus.write(0x0100 | self.sp as u16, v);
        self.sp = self.sp.wrapping_sub(1);
    }

    #[inline]
    fn pull<B: Bus>(&mut self, bus: &mut B) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        bus.read(0x0100 | self.sp as u16)
    }

    fn execute<B: Bus>(&mut self, bus: &mut B, opcode: u8) -> u8 {
        use addressing::*;
        // Returns cycles. Each branch embeds the cycle count from standard
        // 6502 cycle tables; page-crossing branches add 1, branches taken add 1.
        match opcode {
            // ------- LDA -------
            0xA9 => { let v = self.fetch(bus); self.a = v; self.set_nz(v); 2 }
            0xA5 => { let a = addr_zp(self, bus); let v = bus.read(a); self.a = v; self.set_nz(v); 3 }
            0xB5 => { let a = addr_zpx(self, bus); let v = bus.read(a); self.a = v; self.set_nz(v); 4 }
            0xAD => { let a = addr_abs(self, bus); let v = bus.read(a); self.a = v; self.set_nz(v); 4 }
            0xBD => { let (a, c) = addr_absx(self, bus); let v = bus.read(a); self.a = v; self.set_nz(v); 4 + c }
            0xB9 => { let (a, c) = addr_absy(self, bus); let v = bus.read(a); self.a = v; self.set_nz(v); 4 + c }
            0xA1 => { let a = addr_indx(self, bus); let v = bus.read(a); self.a = v; self.set_nz(v); 6 }
            0xB1 => { let (a, c) = addr_indy(self, bus); let v = bus.read(a); self.a = v; self.set_nz(v); 5 + c }

            // ------- LDX -------
            0xA2 => { let v = self.fetch(bus); self.x = v; self.set_nz(v); 2 }
            0xA6 => { let a = addr_zp(self, bus); let v = bus.read(a); self.x = v; self.set_nz(v); 3 }
            0xB6 => { let a = addr_zpy(self, bus); let v = bus.read(a); self.x = v; self.set_nz(v); 4 }
            0xAE => { let a = addr_abs(self, bus); let v = bus.read(a); self.x = v; self.set_nz(v); 4 }
            0xBE => { let (a, c) = addr_absy(self, bus); let v = bus.read(a); self.x = v; self.set_nz(v); 4 + c }

            // ------- LDY -------
            0xA0 => { let v = self.fetch(bus); self.y = v; self.set_nz(v); 2 }
            0xA4 => { let a = addr_zp(self, bus); let v = bus.read(a); self.y = v; self.set_nz(v); 3 }
            0xB4 => { let a = addr_zpx(self, bus); let v = bus.read(a); self.y = v; self.set_nz(v); 4 }
            0xAC => { let a = addr_abs(self, bus); let v = bus.read(a); self.y = v; self.set_nz(v); 4 }
            0xBC => { let (a, c) = addr_absx(self, bus); let v = bus.read(a); self.y = v; self.set_nz(v); 4 + c }

            // ------- STA -------
            0x85 => { let a = addr_zp(self, bus); bus.write(a, self.a); 3 }
            0x95 => { let a = addr_zpx(self, bus); bus.write(a, self.a); 4 }
            0x8D => { let a = addr_abs(self, bus); bus.write(a, self.a); 4 }
            0x9D => { let (a, _) = addr_absx(self, bus); bus.write(a, self.a); 5 }
            0x99 => { let (a, _) = addr_absy(self, bus); bus.write(a, self.a); 5 }
            0x81 => { let a = addr_indx(self, bus); bus.write(a, self.a); 6 }
            0x91 => { let (a, _) = addr_indy(self, bus); bus.write(a, self.a); 6 }

            // ------- STX / STY -------
            0x86 => { let a = addr_zp(self, bus); bus.write(a, self.x); 3 }
            0x96 => { let a = addr_zpy(self, bus); bus.write(a, self.x); 4 }
            0x8E => { let a = addr_abs(self, bus); bus.write(a, self.x); 4 }
            0x84 => { let a = addr_zp(self, bus); bus.write(a, self.y); 3 }
            0x94 => { let a = addr_zpx(self, bus); bus.write(a, self.y); 4 }
            0x8C => { let a = addr_abs(self, bus); bus.write(a, self.y); 4 }

            // ------- Transfers -------
            0xAA => { self.x = self.a; self.set_nz(self.x); 2 } // TAX
            0x8A => { self.a = self.x; self.set_nz(self.a); 2 } // TXA
            0xA8 => { self.y = self.a; self.set_nz(self.y); 2 } // TAY
            0x98 => { self.a = self.y; self.set_nz(self.a); 2 } // TYA
            0xBA => { self.x = self.sp; self.set_nz(self.x); 2 } // TSX
            0x9A => { self.sp = self.x; 2 } // TXS

            // ------- Stack -------
            0x48 => { self.push(bus, self.a); 3 } // PHA
            0x68 => { self.a = self.pull(bus); self.set_nz(self.a); 4 } // PLA
            0x08 => { self.push(bus, self.p | Flags::B.bits() | Flags::U.bits()); 3 } // PHP
            0x28 => { let v = self.pull(bus); self.p = (v & !Flags::B.bits()) | Flags::U.bits(); 4 } // PLP

            // ------- Flags -------
            0x18 => { self.set_flag(Flags::C, false); 2 } // CLC
            0x38 => { self.set_flag(Flags::C, true); 2 }  // SEC
            0x58 => { self.set_flag(Flags::I, false); 2 } // CLI
            0x78 => { self.set_flag(Flags::I, true); 2 }  // SEI
            0xB8 => { self.set_flag(Flags::V, false); 2 } // CLV
            0xD8 => { self.set_flag(Flags::D, false); 2 } // CLD
            0xF8 => { self.set_flag(Flags::D, true); 2 }  // SED

            // ------- ADC -------
            0x69 => { let v = self.fetch(bus); self.adc(v); 2 }
            0x65 => { let a = addr_zp(self, bus); let v = bus.read(a); self.adc(v); 3 }
            0x75 => { let a = addr_zpx(self, bus); let v = bus.read(a); self.adc(v); 4 }
            0x6D => { let a = addr_abs(self, bus); let v = bus.read(a); self.adc(v); 4 }
            0x7D => { let (a, c) = addr_absx(self, bus); let v = bus.read(a); self.adc(v); 4 + c }
            0x79 => { let (a, c) = addr_absy(self, bus); let v = bus.read(a); self.adc(v); 4 + c }
            0x61 => { let a = addr_indx(self, bus); let v = bus.read(a); self.adc(v); 6 }
            0x71 => { let (a, c) = addr_indy(self, bus); let v = bus.read(a); self.adc(v); 5 + c }

            // ------- SBC (incl. undocumented 0xEB) -------
            0xE9 | 0xEB => { let v = self.fetch(bus); self.sbc(v); 2 }
            0xE5 => { let a = addr_zp(self, bus); let v = bus.read(a); self.sbc(v); 3 }
            0xF5 => { let a = addr_zpx(self, bus); let v = bus.read(a); self.sbc(v); 4 }
            0xED => { let a = addr_abs(self, bus); let v = bus.read(a); self.sbc(v); 4 }
            0xFD => { let (a, c) = addr_absx(self, bus); let v = bus.read(a); self.sbc(v); 4 + c }
            0xF9 => { let (a, c) = addr_absy(self, bus); let v = bus.read(a); self.sbc(v); 4 + c }
            0xE1 => { let a = addr_indx(self, bus); let v = bus.read(a); self.sbc(v); 6 }
            0xF1 => { let (a, c) = addr_indy(self, bus); let v = bus.read(a); self.sbc(v); 5 + c }

            // ------- Bitwise AND -------
            0x29 => { let v = self.fetch(bus); self.a &= v; self.set_nz(self.a); 2 }
            0x25 => { let a = addr_zp(self, bus); self.a &= bus.read(a); self.set_nz(self.a); 3 }
            0x35 => { let a = addr_zpx(self, bus); self.a &= bus.read(a); self.set_nz(self.a); 4 }
            0x2D => { let a = addr_abs(self, bus); self.a &= bus.read(a); self.set_nz(self.a); 4 }
            0x3D => { let (a, c) = addr_absx(self, bus); self.a &= bus.read(a); self.set_nz(self.a); 4 + c }
            0x39 => { let (a, c) = addr_absy(self, bus); self.a &= bus.read(a); self.set_nz(self.a); 4 + c }
            0x21 => { let a = addr_indx(self, bus); self.a &= bus.read(a); self.set_nz(self.a); 6 }
            0x31 => { let (a, c) = addr_indy(self, bus); self.a &= bus.read(a); self.set_nz(self.a); 5 + c }

            // ------- Bitwise OR -------
            0x09 => { let v = self.fetch(bus); self.a |= v; self.set_nz(self.a); 2 }
            0x05 => { let a = addr_zp(self, bus); self.a |= bus.read(a); self.set_nz(self.a); 3 }
            0x15 => { let a = addr_zpx(self, bus); self.a |= bus.read(a); self.set_nz(self.a); 4 }
            0x0D => { let a = addr_abs(self, bus); self.a |= bus.read(a); self.set_nz(self.a); 4 }
            0x1D => { let (a, c) = addr_absx(self, bus); self.a |= bus.read(a); self.set_nz(self.a); 4 + c }
            0x19 => { let (a, c) = addr_absy(self, bus); self.a |= bus.read(a); self.set_nz(self.a); 4 + c }
            0x01 => { let a = addr_indx(self, bus); self.a |= bus.read(a); self.set_nz(self.a); 6 }
            0x11 => { let (a, c) = addr_indy(self, bus); self.a |= bus.read(a); self.set_nz(self.a); 5 + c }

            // ------- Bitwise EOR -------
            0x49 => { let v = self.fetch(bus); self.a ^= v; self.set_nz(self.a); 2 }
            0x45 => { let a = addr_zp(self, bus); self.a ^= bus.read(a); self.set_nz(self.a); 3 }
            0x55 => { let a = addr_zpx(self, bus); self.a ^= bus.read(a); self.set_nz(self.a); 4 }
            0x4D => { let a = addr_abs(self, bus); self.a ^= bus.read(a); self.set_nz(self.a); 4 }
            0x5D => { let (a, c) = addr_absx(self, bus); self.a ^= bus.read(a); self.set_nz(self.a); 4 + c }
            0x59 => { let (a, c) = addr_absy(self, bus); self.a ^= bus.read(a); self.set_nz(self.a); 4 + c }
            0x41 => { let a = addr_indx(self, bus); self.a ^= bus.read(a); self.set_nz(self.a); 6 }
            0x51 => { let (a, c) = addr_indy(self, bus); self.a ^= bus.read(a); self.set_nz(self.a); 5 + c }

            // ------- CMP / CPX / CPY -------
            0xC9 => { let v = self.fetch(bus); self.compare(self.a, v); 2 }
            0xC5 => { let a = addr_zp(self, bus); let v = bus.read(a); self.compare(self.a, v); 3 }
            0xD5 => { let a = addr_zpx(self, bus); let v = bus.read(a); self.compare(self.a, v); 4 }
            0xCD => { let a = addr_abs(self, bus); let v = bus.read(a); self.compare(self.a, v); 4 }
            0xDD => { let (a, c) = addr_absx(self, bus); let v = bus.read(a); self.compare(self.a, v); 4 + c }
            0xD9 => { let (a, c) = addr_absy(self, bus); let v = bus.read(a); self.compare(self.a, v); 4 + c }
            0xC1 => { let a = addr_indx(self, bus); let v = bus.read(a); self.compare(self.a, v); 6 }
            0xD1 => { let (a, c) = addr_indy(self, bus); let v = bus.read(a); self.compare(self.a, v); 5 + c }
            0xE0 => { let v = self.fetch(bus); self.compare(self.x, v); 2 }
            0xE4 => { let a = addr_zp(self, bus); let v = bus.read(a); self.compare(self.x, v); 3 }
            0xEC => { let a = addr_abs(self, bus); let v = bus.read(a); self.compare(self.x, v); 4 }
            0xC0 => { let v = self.fetch(bus); self.compare(self.y, v); 2 }
            0xC4 => { let a = addr_zp(self, bus); let v = bus.read(a); self.compare(self.y, v); 3 }
            0xCC => { let a = addr_abs(self, bus); let v = bus.read(a); self.compare(self.y, v); 4 }

            // ------- INC / DEC memory -------
            0xE6 => { let a = addr_zp(self, bus); let v = bus.read(a).wrapping_add(1); bus.write(a, v); self.set_nz(v); 5 }
            0xF6 => { let a = addr_zpx(self, bus); let v = bus.read(a).wrapping_add(1); bus.write(a, v); self.set_nz(v); 6 }
            0xEE => { let a = addr_abs(self, bus); let v = bus.read(a).wrapping_add(1); bus.write(a, v); self.set_nz(v); 6 }
            0xFE => { let (a, _) = addr_absx(self, bus); let v = bus.read(a).wrapping_add(1); bus.write(a, v); self.set_nz(v); 7 }
            0xC6 => { let a = addr_zp(self, bus); let v = bus.read(a).wrapping_sub(1); bus.write(a, v); self.set_nz(v); 5 }
            0xD6 => { let a = addr_zpx(self, bus); let v = bus.read(a).wrapping_sub(1); bus.write(a, v); self.set_nz(v); 6 }
            0xCE => { let a = addr_abs(self, bus); let v = bus.read(a).wrapping_sub(1); bus.write(a, v); self.set_nz(v); 6 }
            0xDE => { let (a, _) = addr_absx(self, bus); let v = bus.read(a).wrapping_sub(1); bus.write(a, v); self.set_nz(v); 7 }

            // ------- INX / DEX / INY / DEY -------
            0xE8 => { self.x = self.x.wrapping_add(1); self.set_nz(self.x); 2 }
            0xCA => { self.x = self.x.wrapping_sub(1); self.set_nz(self.x); 2 }
            0xC8 => { self.y = self.y.wrapping_add(1); self.set_nz(self.y); 2 }
            0x88 => { self.y = self.y.wrapping_sub(1); self.set_nz(self.y); 2 }

            // ------- Shifts on A -------
            0x0A => { let (v,c) = asl(self.a); self.a = v; self.set_flag(Flags::C, c); self.set_nz(v); 2 }
            0x4A => { let (v,c) = lsr(self.a); self.a = v; self.set_flag(Flags::C, c); self.set_nz(v); 2 }
            0x2A => { let cin = self.flag(Flags::C); let (v,c) = rol(self.a, cin); self.a = v; self.set_flag(Flags::C, c); self.set_nz(v); 2 }
            0x6A => { let cin = self.flag(Flags::C); let (v,c) = ror(self.a, cin); self.a = v; self.set_flag(Flags::C, c); self.set_nz(v); 2 }

            // ------- Shifts on memory -------
            0x06 => { let a = addr_zp(self, bus); let v = bus.read(a); let (r,c) = asl(v); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 5 }
            0x16 => { let a = addr_zpx(self, bus); let v = bus.read(a); let (r,c) = asl(v); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 6 }
            0x0E => { let a = addr_abs(self, bus); let v = bus.read(a); let (r,c) = asl(v); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 6 }
            0x1E => { let (a,_) = addr_absx(self, bus); let v = bus.read(a); let (r,c) = asl(v); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 7 }
            0x46 => { let a = addr_zp(self, bus); let v = bus.read(a); let (r,c) = lsr(v); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 5 }
            0x56 => { let a = addr_zpx(self, bus); let v = bus.read(a); let (r,c) = lsr(v); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 6 }
            0x4E => { let a = addr_abs(self, bus); let v = bus.read(a); let (r,c) = lsr(v); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 6 }
            0x5E => { let (a,_) = addr_absx(self, bus); let v = bus.read(a); let (r,c) = lsr(v); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 7 }
            0x26 => { let a = addr_zp(self, bus); let v = bus.read(a); let cin = self.flag(Flags::C); let (r,c) = rol(v, cin); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 5 }
            0x36 => { let a = addr_zpx(self, bus); let v = bus.read(a); let cin = self.flag(Flags::C); let (r,c) = rol(v, cin); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 6 }
            0x2E => { let a = addr_abs(self, bus); let v = bus.read(a); let cin = self.flag(Flags::C); let (r,c) = rol(v, cin); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 6 }
            0x3E => { let (a,_) = addr_absx(self, bus); let v = bus.read(a); let cin = self.flag(Flags::C); let (r,c) = rol(v, cin); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 7 }
            0x66 => { let a = addr_zp(self, bus); let v = bus.read(a); let cin = self.flag(Flags::C); let (r,c) = ror(v, cin); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 5 }
            0x76 => { let a = addr_zpx(self, bus); let v = bus.read(a); let cin = self.flag(Flags::C); let (r,c) = ror(v, cin); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 6 }
            0x6E => { let a = addr_abs(self, bus); let v = bus.read(a); let cin = self.flag(Flags::C); let (r,c) = ror(v, cin); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 6 }
            0x7E => { let (a,_) = addr_absx(self, bus); let v = bus.read(a); let cin = self.flag(Flags::C); let (r,c) = ror(v, cin); bus.write(a, r); self.set_flag(Flags::C, c); self.set_nz(r); 7 }

            // ------- BIT -------
            0x24 => { let a = addr_zp(self, bus); let v = bus.read(a); self.bit(v); 3 }
            0x2C => { let a = addr_abs(self, bus); let v = bus.read(a); self.bit(v); 4 }

            // ------- Branches -------
            0x10 => self.branch(bus, !self.flag(Flags::N)), // BPL
            0x30 => self.branch(bus, self.flag(Flags::N)),  // BMI
            0x50 => self.branch(bus, !self.flag(Flags::V)), // BVC
            0x70 => self.branch(bus, self.flag(Flags::V)),  // BVS
            0x90 => self.branch(bus, !self.flag(Flags::C)), // BCC
            0xB0 => self.branch(bus, self.flag(Flags::C)),  // BCS
            0xD0 => self.branch(bus, !self.flag(Flags::Z)), // BNE
            0xF0 => self.branch(bus, self.flag(Flags::Z)),  // BEQ

            // ------- Jumps / subroutines -------
            0x4C => { self.pc = self.fetch16(bus); 3 } // JMP abs
            0x6C => {
                // JMP (ind) with 6502 page-wrap bug: low byte wraps within page
                let ptr = self.fetch16(bus);
                let lo = bus.read(ptr) as u16;
                let hi_addr = (ptr & 0xFF00) | ((ptr + 1) & 0x00FF);
                let hi = bus.read(hi_addr) as u16;
                self.pc = (hi << 8) | lo;
                5
            }
            0x20 => { // JSR
                let target = self.fetch16(bus);
                let ret = self.pc.wrapping_sub(1);
                self.push(bus, (ret >> 8) as u8);
                self.push(bus, ret as u8);
                self.pc = target;
                6
            }
            0x60 => { // RTS
                let lo = self.pull(bus) as u16;
                let hi = self.pull(bus) as u16;
                self.pc = ((hi << 8) | lo).wrapping_add(1);
                6
            }
            0x40 => { // RTI
                let p = self.pull(bus);
                self.p = (p & !Flags::B.bits()) | Flags::U.bits();
                let lo = self.pull(bus) as u16;
                let hi = self.pull(bus) as u16;
                self.pc = (hi << 8) | lo;
                6
            }
            0x00 => { // BRK
                self.pc = self.pc.wrapping_add(1);
                let pc = self.pc;
                self.push(bus, (pc >> 8) as u8);
                self.push(bus, pc as u8);
                self.push(bus, self.p | Flags::B.bits() | Flags::U.bits());
                self.set_flag(Flags::I, true);
                self.pc = peek16(bus, 0xFFFE);
                7
            }

            // ------- NOPs -------
            0xEA => 2,
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => 2,
            0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => { self.pc = self.pc.wrapping_add(1); 2 }
            0x04 | 0x44 | 0x64 => { self.pc = self.pc.wrapping_add(1); 3 }
            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => { self.pc = self.pc.wrapping_add(1); 4 }
            0x0C => { self.pc = self.pc.wrapping_add(2); 4 }
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                let (_a, c) = addr_absx(self, bus); 4 + c
            }

            // ------- Undocumented: LAX (load A and X) -------
            0xA7 => { let a = addr_zp(self, bus); let v = bus.read(a); self.a = v; self.x = v; self.set_nz(v); 3 }
            0xB7 => { let a = addr_zpy(self, bus); let v = bus.read(a); self.a = v; self.x = v; self.set_nz(v); 4 }
            0xAF => { let a = addr_abs(self, bus); let v = bus.read(a); self.a = v; self.x = v; self.set_nz(v); 4 }
            0xBF => { let (a, c) = addr_absy(self, bus); let v = bus.read(a); self.a = v; self.x = v; self.set_nz(v); 4 + c }
            0xA3 => { let a = addr_indx(self, bus); let v = bus.read(a); self.a = v; self.x = v; self.set_nz(v); 6 }
            0xB3 => { let (a, c) = addr_indy(self, bus); let v = bus.read(a); self.a = v; self.x = v; self.set_nz(v); 5 + c }

            // ------- Undocumented: SAX (store A & X) -------
            0x87 => { let a = addr_zp(self, bus); bus.write(a, self.a & self.x); 3 }
            0x97 => { let a = addr_zpy(self, bus); bus.write(a, self.a & self.x); 4 }
            0x8F => { let a = addr_abs(self, bus); bus.write(a, self.a & self.x); 4 }
            0x83 => { let a = addr_indx(self, bus); bus.write(a, self.a & self.x); 6 }

            // ------- Undocumented: DCP / ISC / SLO / RLA / SRE / RRA (rmw combos) -------
            0xC7 => { let a = addr_zp(self, bus); self.dcp(bus, a); 5 }
            0xD7 => { let a = addr_zpx(self, bus); self.dcp(bus, a); 6 }
            0xCF => { let a = addr_abs(self, bus); self.dcp(bus, a); 6 }
            0xDF => { let (a,_) = addr_absx(self, bus); self.dcp(bus, a); 7 }
            0xDB => { let (a,_) = addr_absy(self, bus); self.dcp(bus, a); 7 }
            0xC3 => { let a = addr_indx(self, bus); self.dcp(bus, a); 8 }
            0xD3 => { let (a,_) = addr_indy(self, bus); self.dcp(bus, a); 8 }

            0xE7 => { let a = addr_zp(self, bus); self.isc(bus, a); 5 }
            0xF7 => { let a = addr_zpx(self, bus); self.isc(bus, a); 6 }
            0xEF => { let a = addr_abs(self, bus); self.isc(bus, a); 6 }
            0xFF => { let (a,_) = addr_absx(self, bus); self.isc(bus, a); 7 }
            0xFB => { let (a,_) = addr_absy(self, bus); self.isc(bus, a); 7 }
            0xE3 => { let a = addr_indx(self, bus); self.isc(bus, a); 8 }
            0xF3 => { let (a,_) = addr_indy(self, bus); self.isc(bus, a); 8 }

            0x07 => { let a = addr_zp(self, bus); self.slo(bus, a); 5 }
            0x17 => { let a = addr_zpx(self, bus); self.slo(bus, a); 6 }
            0x0F => { let a = addr_abs(self, bus); self.slo(bus, a); 6 }
            0x1F => { let (a,_) = addr_absx(self, bus); self.slo(bus, a); 7 }
            0x1B => { let (a,_) = addr_absy(self, bus); self.slo(bus, a); 7 }
            0x03 => { let a = addr_indx(self, bus); self.slo(bus, a); 8 }
            0x13 => { let (a,_) = addr_indy(self, bus); self.slo(bus, a); 8 }

            0x27 => { let a = addr_zp(self, bus); self.rla(bus, a); 5 }
            0x37 => { let a = addr_zpx(self, bus); self.rla(bus, a); 6 }
            0x2F => { let a = addr_abs(self, bus); self.rla(bus, a); 6 }
            0x3F => { let (a,_) = addr_absx(self, bus); self.rla(bus, a); 7 }
            0x3B => { let (a,_) = addr_absy(self, bus); self.rla(bus, a); 7 }
            0x23 => { let a = addr_indx(self, bus); self.rla(bus, a); 8 }
            0x33 => { let (a,_) = addr_indy(self, bus); self.rla(bus, a); 8 }

            0x47 => { let a = addr_zp(self, bus); self.sre(bus, a); 5 }
            0x57 => { let a = addr_zpx(self, bus); self.sre(bus, a); 6 }
            0x4F => { let a = addr_abs(self, bus); self.sre(bus, a); 6 }
            0x5F => { let (a,_) = addr_absx(self, bus); self.sre(bus, a); 7 }
            0x5B => { let (a,_) = addr_absy(self, bus); self.sre(bus, a); 7 }
            0x43 => { let a = addr_indx(self, bus); self.sre(bus, a); 8 }
            0x53 => { let (a,_) = addr_indy(self, bus); self.sre(bus, a); 8 }

            0x67 => { let a = addr_zp(self, bus); self.rra(bus, a); 5 }
            0x77 => { let a = addr_zpx(self, bus); self.rra(bus, a); 6 }
            0x6F => { let a = addr_abs(self, bus); self.rra(bus, a); 6 }
            0x7F => { let (a,_) = addr_absx(self, bus); self.rra(bus, a); 7 }
            0x7B => { let (a,_) = addr_absy(self, bus); self.rra(bus, a); 7 }
            0x63 => { let a = addr_indx(self, bus); self.rra(bus, a); 8 }
            0x73 => { let (a,_) = addr_indy(self, bus); self.rra(bus, a); 8 }

            // Other immediate undocumented
            0x0B | 0x2B => { // ANC
                let v = self.fetch(bus);
                self.a &= v;
                self.set_nz(self.a);
                self.set_flag(Flags::C, self.a & 0x80 != 0);
                2
            }
            0x4B => { // ALR = AND + LSR A
                let v = self.fetch(bus);
                self.a &= v;
                let (r, c) = lsr(self.a);
                self.a = r;
                self.set_flag(Flags::C, c);
                self.set_nz(r);
                2
            }
            0x6B => { // ARR = AND + ROR A (flags are special; approximate)
                let v = self.fetch(bus);
                self.a &= v;
                let cin = self.flag(Flags::C);
                let (r, _) = ror(self.a, cin);
                self.a = r;
                self.set_flag(Flags::C, r & 0x40 != 0);
                self.set_flag(Flags::V, ((r & 0x40) ^ ((r & 0x20) << 1)) != 0);
                self.set_nz(r);
                2
            }
            0xCB => { // SBX (AXS)
                let v = self.fetch(bus);
                let t = (self.a & self.x) as u16;
                let r = t.wrapping_sub(v as u16);
                self.set_flag(Flags::C, r < 0x100);
                self.x = r as u8;
                self.set_nz(self.x);
                2
            }

            // JAM / KIL opcodes
            0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
                self.jam = true; 2
            }

            // Remaining rare undocumented: treat as 2-cycle NOP-immediate
            _ => { log::warn!("unimplemented opcode ${:02X} at PC=${:04X}", opcode, self.pc.wrapping_sub(1)); 2 }
        }
    }

    // ---------------- ALU helpers ----------------

    fn adc(&mut self, v: u8) {
        if self.flag(Flags::D) {
            let (res, c, vflag) = bcd::adc_decimal(self.a, v, self.flag(Flags::C));
            self.a = res;
            self.set_flag(Flags::C, c);
            self.set_flag(Flags::V, vflag);
            self.set_nz(res);
        } else {
            let cin = if self.flag(Flags::C) { 1u16 } else { 0u16 };
            let sum = self.a as u16 + v as u16 + cin;
            let res = sum as u8;
            self.set_flag(Flags::C, sum > 0xFF);
            self.set_flag(Flags::V, ((!(self.a ^ v)) & (self.a ^ res) & 0x80) != 0);
            self.a = res;
            self.set_nz(res);
        }
    }

    fn sbc(&mut self, v: u8) {
        if self.flag(Flags::D) {
            let (res, c, vflag) = bcd::sbc_decimal(self.a, v, self.flag(Flags::C));
            self.a = res;
            self.set_flag(Flags::C, c);
            self.set_flag(Flags::V, vflag);
            self.set_nz(res);
        } else {
            // SBC in binary = ADC with one's complement.
            self.adc(!v);
        }
    }

    fn compare(&mut self, reg: u8, v: u8) {
        let r = reg.wrapping_sub(v);
        self.set_flag(Flags::C, reg >= v);
        self.set_nz(r);
    }

    fn bit(&mut self, v: u8) {
        self.set_flag(Flags::Z, (self.a & v) == 0);
        self.set_flag(Flags::N, v & 0x80 != 0);
        self.set_flag(Flags::V, v & 0x40 != 0);
    }

    fn branch<B: Bus>(&mut self, bus: &mut B, cond: bool) -> u8 {
        let off = self.fetch(bus) as i8;
        if !cond { return 2; }
        let old = self.pc;
        let new = (old as i32).wrapping_add(off as i32) as u16;
        self.pc = new;
        let page_cross = (old & 0xFF00) != (new & 0xFF00);
        3 + if page_cross { 1 } else { 0 }
    }

    // ---------------- Undocumented RMW helpers ----------------

    fn dcp<B: Bus>(&mut self, bus: &mut B, a: u16) {
        let v = bus.read(a).wrapping_sub(1);
        bus.write(a, v);
        self.compare(self.a, v);
    }
    fn isc<B: Bus>(&mut self, bus: &mut B, a: u16) {
        let v = bus.read(a).wrapping_add(1);
        bus.write(a, v);
        self.sbc(v);
    }
    fn slo<B: Bus>(&mut self, bus: &mut B, a: u16) {
        let v = bus.read(a);
        let (r, c) = asl(v);
        bus.write(a, r);
        self.set_flag(Flags::C, c);
        self.a |= r;
        self.set_nz(self.a);
    }
    fn rla<B: Bus>(&mut self, bus: &mut B, a: u16) {
        let v = bus.read(a);
        let cin = self.flag(Flags::C);
        let (r, c) = rol(v, cin);
        bus.write(a, r);
        self.set_flag(Flags::C, c);
        self.a &= r;
        self.set_nz(self.a);
    }
    fn sre<B: Bus>(&mut self, bus: &mut B, a: u16) {
        let v = bus.read(a);
        let (r, c) = lsr(v);
        bus.write(a, r);
        self.set_flag(Flags::C, c);
        self.a ^= r;
        self.set_nz(self.a);
    }
    fn rra<B: Bus>(&mut self, bus: &mut B, a: u16) {
        let v = bus.read(a);
        let cin = self.flag(Flags::C);
        let (r, c) = ror(v, cin);
        bus.write(a, r);
        self.set_flag(Flags::C, c);
        self.adc(r);
    }
}

#[inline]
fn asl(v: u8) -> (u8, bool) { (v << 1, v & 0x80 != 0) }
#[inline]
fn lsr(v: u8) -> (u8, bool) { (v >> 1, v & 0x01 != 0) }
#[inline]
fn rol(v: u8, cin: bool) -> (u8, bool) {
    let r = (v << 1) | if cin { 1 } else { 0 };
    (r, v & 0x80 != 0)
}
#[inline]
fn ror(v: u8, cin: bool) -> (u8, bool) {
    let r = (v >> 1) | if cin { 0x80 } else { 0 };
    (r, v & 0x01 != 0)
}
