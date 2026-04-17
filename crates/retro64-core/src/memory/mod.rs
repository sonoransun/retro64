//! 64 KB RAM, 1 KB color RAM, ROM slots and PLA-driven bank switching.

mod pla;
mod rom;

pub use pla::{PlaConfig, Region};
pub use rom::{RomSet, BUILTIN_BASIC, BUILTIN_CHARGEN, BUILTIN_KERNAL};

/// Primary memory object: owns 64 KB RAM, 1 KB color RAM and the
/// three banked ROMs. The CPU and VIC-II both read through it.
pub struct Memory {
    /// 64 KB of DRAM underneath all ROM/IO banks.
    pub ram: Box<[u8; 0x1_0000]>,
    /// 1 KB × 4-bit nibbles at $D800-$DBFF (stored as bytes, masked on read/write).
    pub color_ram: Box<[u8; 0x0400]>,
    /// BASIC ROM ($A000-$BFFF).
    pub basic: Box<[u8; 0x2000]>,
    /// KERNAL ROM ($E000-$FFFF).
    pub kernal: Box<[u8; 0x2000]>,
    /// Character ROM ($D000-$DFFF when CHAREN=0, and for VIC at $1000/$9000).
    pub chargen: Box<[u8; 0x1000]>,

    /// Cached PLA state derived from LORAM/HIRAM/CHAREN bits and cart lines.
    pub pla: PlaConfig,

    /// CIA2 Port A inverted low 2 bits set the VIC bank base.
    pub vic_bank_base: u16,

    /// EXROM line state (true = high). Drives PLA.
    pub exrom: bool,
    /// GAME line state (true = high). Drives PLA.
    pub game: bool,
}

impl Memory {
    /// Create a new Memory with built-in ROM stubs.
    pub fn new() -> Self {
        let mut mem = Memory {
            ram: Box::new([0u8; 0x1_0000]),
            color_ram: Box::new([0u8; 0x0400]),
            basic: Box::new(*BUILTIN_BASIC),
            kernal: Box::new(*BUILTIN_KERNAL),
            chargen: Box::new(*BUILTIN_CHARGEN),
            pla: PlaConfig::default(),
            vic_bank_base: 0x0000,
            exrom: true,
            game: true,
        };
        mem.ram[0x0000] = 0x2F; // DDR direction bits (bits 0-5 output)
        mem.ram[0x0001] = 0x37; // LORAM | HIRAM | CHAREN
        mem.recompute_pla();
        mem
    }

    /// Install real Commodore ROMs from a set.
    pub fn install_roms(&mut self, roms: &RomSet) {
        if let Some(b) = &roms.basic {
            self.basic.copy_from_slice(&b[..]);
        }
        if let Some(k) = &roms.kernal {
            self.kernal.copy_from_slice(&k[..]);
        }
        if let Some(c) = &roms.chargen {
            self.chargen.copy_from_slice(&c[..]);
        }
    }

    /// Re-derive the PLA configuration from the current CPU I/O port bits.
    pub fn recompute_pla(&mut self) {
        // Port $0001 bits output through the DDR $0000 mask. Bits not
        // configured as outputs read as 1 (pulled up).
        let ddr = self.ram[0x0000];
        let port = self.ram[0x0001];
        let out = (port & ddr) | (!ddr & 0x17);
        self.pla = PlaConfig::from_bits(out & 0x07, self.exrom, self.game);
    }

    /// CPU read with full PLA/IO routing (the caller dispatches IO to chips).
    pub fn cpu_read(&self, addr: u16) -> u8 {
        match self.pla.region(addr) {
            Region::Ram => self.ram[addr as usize],
            Region::BasicRom => self.basic[(addr - 0xA000) as usize],
            Region::KernalRom => self.kernal[(addr - 0xE000) as usize],
            Region::CharRom => self.chargen[(addr - 0xD000) as usize],
            // IO is dispatched by the system module, but if the caller reaches
            // here we return RAM under the IO region (open bus fallback).
            Region::Io => self.ram[addr as usize],
        }
    }

    /// CPU write. Writes to ROM regions land in the underlying RAM (C64 PLA
    /// always writes RAM regardless of bank).
    pub fn cpu_write(&mut self, addr: u16, val: u8) {
        self.ram[addr as usize] = val;
        if addr == 0x0000 || addr == 0x0001 {
            self.recompute_pla();
        }
    }

    /// Read a color-RAM nibble (upper 4 bits are open-bus on real hardware,
    /// we return 0).
    pub fn color_read(&self, addr: u16) -> u8 {
        self.color_ram[(addr & 0x03FF) as usize] & 0x0F
    }

    /// Write a color-RAM nibble.
    pub fn color_write(&mut self, addr: u16, val: u8) {
        self.color_ram[(addr & 0x03FF) as usize] = val & 0x0F;
    }

    /// Read from the VIC-II's memory view (flat 16 KB window anchored at
    /// `vic_bank_base`). The Character ROM is overlaid at $1000-$1FFF of
    /// bank 0 and bank 2 (absolute $1000 and $9000).
    pub fn vic_read(&self, addr: u16) -> u8 {
        let a = (self.vic_bank_base + (addr & 0x3FFF)) & 0xFFFF;
        if (a & 0x7000) == 0x1000 && (a & 0xC000) != 0x4000 && (a & 0xC000) != 0xC000 {
            self.chargen[(a & 0x0FFF) as usize]
        } else {
            self.ram[a as usize]
        }
    }

    /// True when the current PLA configuration maps I/O into $D000-$DFFF.
    pub fn io_visible(&self) -> bool {
        matches!(self.pla.region(0xD000), Region::Io)
    }

    /// True when the current PLA configuration maps Character ROM into
    /// $D000-$DFFF (CHAREN low, but BASIC or KERNAL ROM also enabled).
    pub fn char_rom_visible_at_d000(&self) -> bool {
        matches!(self.pla.region(0xD000), Region::CharRom)
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
