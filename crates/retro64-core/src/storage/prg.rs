//! PRG format: 2-byte little-endian load address followed by raw bytes.

use super::StorageError;

/// Parsed PRG: load address and body.
pub struct Prg<'a> {
    /// Target load address in RAM.
    pub load_addr: u16,
    /// Program body bytes.
    pub body: &'a [u8],
}

/// Parse a PRG blob.
pub fn parse(bytes: &[u8]) -> Result<Prg<'_>, StorageError> {
    if bytes.len() < 2 {
        return Err(StorageError::TooShort);
    }
    let load_addr = u16::from_le_bytes([bytes[0], bytes[1]]);
    Ok(Prg { load_addr, body: &bytes[2..] })
}

/// Inject a parsed PRG into RAM. If `load_addr == 0x0801` (the BASIC start),
/// also update the BASIC pointers at $2B/$2C, $2D/$2E and $AE/$AF so that
/// `RUN` sees the program.
pub fn inject(ram: &mut [u8; 0x1_0000], prg: &Prg) {
    let end = prg.load_addr as usize + prg.body.len();
    let end = end.min(ram.len());
    let start = prg.load_addr as usize;
    if start >= ram.len() { return; }
    let n = end - start;
    ram[start..end].copy_from_slice(&prg.body[..n]);

    if prg.load_addr == 0x0801 {
        let eop = end as u16; // end of program
        // TXTTAB = $2B/$2C = start of BASIC text
        ram[0x002B] = 0x01; ram[0x002C] = 0x08;
        // VARTAB / ARYTAB / STREND all point to end of program
        ram[0x002D] = eop as u8; ram[0x002E] = (eop >> 8) as u8;
        ram[0x002F] = eop as u8; ram[0x0030] = (eop >> 8) as u8;
        ram[0x0031] = eop as u8; ram[0x0032] = (eop >> 8) as u8;
    }
}

/// Queue `RUN\r` into the KERNAL keyboard buffer at $0277-$027D so that BASIC
/// autostarts after the READY prompt arrives.
pub fn autostart(ram: &mut [u8; 0x1_0000]) {
    let cmd = b"RUN\r";
    for (i, b) in cmd.iter().enumerate() {
        ram[0x0277 + i] = *b;
    }
    ram[0x00C6] = cmd.len() as u8; // NDX
}
