//! 1541 D64 disk image parser.
//!
//! A standard 35-track image is 174848 bytes. This module knows enough to
//! walk the BAM, list directory entries and follow sector chains — which is
//! used by KERNAL-trap shims to implement LOAD/SAVE without emulating the
//! 1541's 6502 CPU.

use super::StorageError;

const SECTORS_PER_TRACK: [u8; 36] = [
    0, // tracks are 1-indexed
    21,21,21,21,21,21,21,21,21,21,21,21,21,21,21,21,21,  // 1..17
    19,19,19,19,19,19,19,                                 // 18..24
    18,18,18,18,18,18,                                    // 25..30
    17,17,17,17,17,                                       // 31..35
];

/// Byte offset of (track, sector) in the image.
pub fn ts_offset(track: u8, sector: u8) -> usize {
    let mut off = 0usize;
    for t in 1..track {
        off += SECTORS_PER_TRACK[t as usize] as usize * 256;
    }
    off += sector as usize * 256;
    off
}

/// One directory entry.
#[derive(Clone, Debug)]
pub struct DirEntry {
    /// File type nibble (0x82=PRG, 0x81=SEQ, etc.).
    pub file_type: u8,
    /// File name padded with $A0.
    pub name: [u8; 16],
    /// First (track, sector) of the file's data chain.
    pub first_ts: (u8, u8),
    /// File size in 254-byte blocks.
    pub blocks: u16,
}

/// Parsed D64 image (borrows the bytes).
pub struct D64<'a> {
    data: &'a [u8],
}

impl<'a> D64<'a> {
    /// Wrap a D64 image.
    pub fn new(data: &'a [u8]) -> Result<Self, StorageError> {
        if data.len() < ts_offset(18, 0) + 256 {
            return Err(StorageError::TooShort);
        }
        Ok(D64 { data })
    }

    /// Read one sector.
    pub fn sector(&self, track: u8, sector: u8) -> Option<&'a [u8]> {
        let off = ts_offset(track, sector);
        if off + 256 > self.data.len() { return None; }
        Some(&self.data[off..off + 256])
    }

    /// Iterate directory entries.
    pub fn directory(&self) -> Vec<DirEntry> {
        let mut out = Vec::new();
        let (mut t, mut s) = (18u8, 1u8);
        for _step in 0..32 {
            let Some(blk) = self.sector(t, s) else { break; };
            for e in 0..8 {
                let entry = &blk[e * 32..e * 32 + 32];
                let ftype = entry[2];
                if ftype == 0 { continue; }
                let mut name = [0u8; 16];
                name.copy_from_slice(&entry[5..21]);
                let first_ts = (entry[3], entry[4]);
                let blocks = u16::from_le_bytes([entry[30], entry[31]]);
                out.push(DirEntry { file_type: ftype, name, first_ts, blocks });
            }
            t = blk[0];
            s = blk[1];
            if t == 0 { break; }
        }
        out
    }

    /// Read the bytes of a file by following its sector chain.
    pub fn read_file(&self, entry: &DirEntry) -> Vec<u8> {
        let mut out = Vec::new();
        let (mut t, mut s) = entry.first_ts;
        while t != 0 {
            let Some(blk) = self.sector(t, s) else { break; };
            let next_t = blk[0];
            let next_s = blk[1];
            if next_t == 0 {
                // Last sector: bytes 2..=next_s contain real data.
                out.extend_from_slice(&blk[2..=next_s as usize]);
            } else {
                out.extend_from_slice(&blk[2..256]);
            }
            t = next_t; s = next_s;
        }
        out
    }

    /// Locate the first PRG named `name` (PETSCII, no padding) in the directory.
    pub fn find(&self, name: &[u8]) -> Option<DirEntry> {
        for e in self.directory() {
            let n = &e.name;
            let trimmed_len = n.iter().position(|b| *b == 0xA0).unwrap_or(16);
            if &n[..trimmed_len] == name { return Some(e); }
        }
        None
    }
}
