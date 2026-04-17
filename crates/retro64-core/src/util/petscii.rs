//! ASCII ↔ PETSCII conversion (unshifted / upper-case mode).

/// Convert an ASCII byte to screen-compatible PETSCII.
pub fn ascii_to_petscii(c: u8) -> u8 {
    match c {
        b'a'..=b'z' => c - b'a' + b'A',  // lowercase → uppercase PETSCII
        b'A'..=b'Z' => c | 0x80,          // uppercase → graphics (shift)
        b'\n' => 0x0D,
        other => other,
    }
}

/// Convert PETSCII to ASCII.
pub fn petscii_to_ascii(c: u8) -> u8 {
    match c {
        0x0D => b'\n',
        0x41..=0x5A => c + 0x20, // uppercase PETSCII letters → lowercase ASCII
        0xC1..=0xDA => c - 0x80, // shifted letters → uppercase ASCII
        other => other,
    }
}
