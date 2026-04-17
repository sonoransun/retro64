//! Simplified PLA (Programmable Logic Array) decoding.
//!
//! The real PLA considers ~8 input signals and produces ~16 outputs. For
//! a functional emulator we collapse the cases into a per-region table
//! driven by LORAM/HIRAM/CHAREN and the cartridge lines EXROM/GAME.

/// A memory region as seen by the CPU.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Region {
    /// Plain RAM.
    Ram,
    /// BASIC ROM at $A000-$BFFF.
    BasicRom,
    /// KERNAL ROM at $E000-$FFFF.
    KernalRom,
    /// Character ROM at $D000-$DFFF.
    CharRom,
    /// I/O region at $D000-$DFFF (chips live here).
    Io,
}

/// Cached PLA configuration: the region each address belongs to.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PlaConfig {
    /// Mode index 0-7 taken from LORAM/HIRAM/CHAREN (we ignore cart for MVP).
    pub mode: u8,
    basic_rom: bool,
    kernal_rom: bool,
    char_rom: bool,
    io: bool,
}

impl PlaConfig {
    /// Decode the 3-bit mode + cart lines into a region table.
    pub fn from_bits(mode: u8, _exrom: bool, _game: bool) -> Self {
        let loram = mode & 0x01 != 0;
        let hiram = mode & 0x02 != 0;
        let charen = mode & 0x04 != 0;

        // Mirrors the "Zimmers" decoding table for the common (no-cart) case.
        let basic_rom = loram && hiram;
        let kernal_rom = hiram;
        let char_rom = !charen && (loram || hiram);
        let io = charen && (loram || hiram);

        PlaConfig { mode, basic_rom, kernal_rom, char_rom, io }
    }

    /// Classify an address under the current configuration.
    pub fn region(&self, addr: u16) -> Region {
        match addr {
            0xA000..=0xBFFF if self.basic_rom => Region::BasicRom,
            0xD000..=0xDFFF if self.io => Region::Io,
            0xD000..=0xDFFF if self.char_rom => Region::CharRom,
            0xE000..=0xFFFF if self.kernal_rom => Region::KernalRom,
            _ => Region::Ram,
        }
    }
}

impl Default for PlaConfig {
    fn default() -> Self {
        // Power-on: LORAM | HIRAM | CHAREN = 0b111 -> all three ROMs enabled
        // and I/O mapped at $D000.
        Self::from_bits(0x07, true, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_maps_io_and_roms() {
        let p = PlaConfig::default();
        assert_eq!(p.region(0x0001), Region::Ram);
        assert_eq!(p.region(0xA000), Region::BasicRom);
        assert_eq!(p.region(0xD000), Region::Io);
        assert_eq!(p.region(0xE000), Region::KernalRom);
    }

    #[test]
    fn all_ram_when_loram_hiram_zero() {
        let p = PlaConfig::from_bits(0b000, true, true);
        assert_eq!(p.region(0xA000), Region::Ram);
        assert_eq!(p.region(0xD000), Region::Ram);
        assert_eq!(p.region(0xE000), Region::Ram);
    }

    #[test]
    fn char_rom_when_charen_low() {
        let p = PlaConfig::from_bits(0b011, true, true); // LORAM|HIRAM, CHAREN=0
        assert_eq!(p.region(0xD000), Region::CharRom);
        assert_eq!(p.region(0xA000), Region::BasicRom);
        assert_eq!(p.region(0xE000), Region::KernalRom);
    }
}
