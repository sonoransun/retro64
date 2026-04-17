//! T64 tape-archive parser.

use super::StorageError;

/// Parsed T64 entry.
#[derive(Debug, Clone)]
pub struct T64Entry {
    /// PETSCII filename (16 bytes).
    pub name: [u8; 16],
    /// Load address.
    pub load_addr: u16,
    /// End address (exclusive).
    pub end_addr: u16,
    /// Offset within the container.
    pub offset: u32,
}

/// A T64 archive.
pub struct T64<'a> {
    data: &'a [u8],
    /// Parsed directory entries.
    pub entries: Vec<T64Entry>,
}

impl<'a> T64<'a> {
    /// Parse a T64 container.
    pub fn new(data: &'a [u8]) -> Result<Self, StorageError> {
        if data.len() < 64 { return Err(StorageError::TooShort); }
        if &data[0..3] != b"C64" { return Err(StorageError::BadMagic); }
        let num_used = u16::from_le_bytes([data[0x24], data[0x25]]) as usize;
        let mut entries = Vec::new();
        for i in 0..num_used {
            let off = 0x40 + i * 32;
            if off + 32 > data.len() { break; }
            let kind = data[off];
            if kind == 0 { continue; }
            let load_addr = u16::from_le_bytes([data[off+2], data[off+3]]);
            let end_addr = u16::from_le_bytes([data[off+4], data[off+5]]);
            let offset = u32::from_le_bytes([data[off+8], data[off+9], data[off+10], data[off+11]]);
            let mut name = [0u8; 16];
            name.copy_from_slice(&data[off+16..off+32]);
            entries.push(T64Entry { name, load_addr, end_addr, offset });
        }
        Ok(T64 { data, entries })
    }

    /// Extract entry `idx` as a PRG blob (2-byte header + body).
    pub fn extract(&self, idx: usize) -> Option<Vec<u8>> {
        let e = self.entries.get(idx)?;
        let n = (e.end_addr.wrapping_sub(e.load_addr)) as usize;
        let start = e.offset as usize;
        if start + n > self.data.len() { return None; }
        let mut out = Vec::with_capacity(n + 2);
        out.push(e.load_addr as u8);
        out.push((e.load_addr >> 8) as u8);
        out.extend_from_slice(&self.data[start..start + n]);
        Some(out)
    }
}
