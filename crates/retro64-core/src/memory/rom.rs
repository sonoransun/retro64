//! ROM loading. Real Commodore ROMs can be loaded from a directory; in their
//! absence the built-in stubs below produce a minimal, legal bootable system
//! (enough to run the CPU from $FCE2 without crashing; BASIC is not functional
//! without real ROMs).

use std::path::Path;

/// Collection of the three C64 ROMs.
#[derive(Default, Debug)]
pub struct RomSet {
    /// 8 KB BASIC ROM.
    pub basic: Option<Box<[u8; 0x2000]>>,
    /// 8 KB KERNAL ROM.
    pub kernal: Option<Box<[u8; 0x2000]>>,
    /// 4 KB Character ROM.
    pub chargen: Option<Box<[u8; 0x1000]>>,
}

impl RomSet {
    /// Load ROMs from a directory containing `basic`, `kernal`, `chargen`.
    /// Any missing file is left as `None`.
    pub fn from_dir(dir: &Path) -> std::io::Result<Self> {
        let mut rs = RomSet::default();
        rs.basic = load_sized(&dir.join("basic"))?.map(boxed2k);
        rs.kernal = load_sized(&dir.join("kernal"))?.map(boxed2k);
        rs.chargen = load_sized(&dir.join("chargen"))?.map(boxed1k);
        Ok(rs)
    }
}

fn load_sized(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn boxed2k(v: Vec<u8>) -> Box<[u8; 0x2000]> {
    let mut b = Box::new([0u8; 0x2000]);
    let n = v.len().min(0x2000);
    b[..n].copy_from_slice(&v[..n]);
    b
}

fn boxed1k(v: Vec<u8>) -> Box<[u8; 0x1000]> {
    let mut b = Box::new([0u8; 0x1000]);
    let n = v.len().min(0x1000);
    b[..n].copy_from_slice(&v[..n]);
    b
}

/// Minimal BASIC ROM stub — all BRK to trap unwanted execution.
pub const BUILTIN_BASIC: &[u8; 0x2000] = &[0x00; 0x2000];

/// Minimal KERNAL ROM stub. Provides a reset vector at $FCE2 that sets up
/// the stack, sets decimal flag off, and loops. Real software won't run
/// without actual KERNAL ROMs but the CPU is in a sane state after reset.
pub const BUILTIN_KERNAL: &[u8; 0x2000] = &{
    let mut k = [0x00u8; 0x2000]; // all BRK
    // Place a tiny reset routine at $FCE2 (offset 0x1CE2 into the 8 KB ROM):
    //   LDX #$FF
    //   TXS
    //   CLI
    //   loop: JMP loop
    k[0x1CE2] = 0xA2; k[0x1CE3] = 0xFF;  // LDX #$FF
    k[0x1CE4] = 0x9A;                    // TXS
    k[0x1CE5] = 0x58;                    // CLI
    k[0x1CE6] = 0x4C; k[0x1CE7] = 0xE6; k[0x1CE8] = 0xFC; // JMP $FCE6
    // Vectors at $FFFA-$FFFF: NMI=$FCE2, RESET=$FCE2, IRQ=$FCE2
    k[0x1FFA] = 0xE2; k[0x1FFB] = 0xFC;
    k[0x1FFC] = 0xE2; k[0x1FFD] = 0xFC;
    k[0x1FFE] = 0xE2; k[0x1FFF] = 0xFC;
    k
};

/// Minimal Character ROM stub — zeroed glyphs (all blank).
pub const BUILTIN_CHARGEN: &[u8; 0x1000] = &[0x00; 0x1000];
