//! File-format parsers for PRG, D64 and T64 images.

pub mod d64;
pub mod prg;
pub mod t64;

/// Identify a loaded media kind so callers can dispatch.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MediaKind {
    /// `.prg` — raw BASIC/ML program with 2-byte load address header.
    Prg,
    /// `.d64` — 1541 floppy image (35 tracks, 683 sectors, 174848 bytes).
    D64,
    /// `.t64` — tape archive container.
    T64,
    /// `.tap` — raw tape pulse stream (not supported).
    Tap,
    /// `.crt` — cartridge image (not supported).
    Crt,
}

/// Errors produced by the storage subsystem.
#[derive(Debug)]
pub enum StorageError {
    /// The data is smaller than any valid header.
    TooShort,
    /// The magic/signature bytes don't match.
    BadMagic,
    /// Feature is deliberately not implemented in this build.
    Unsupported(&'static str),
    /// Underlying I/O error (e.g. from the host filesystem).
    Io(std::io::Error),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::TooShort => write!(f, "input too short"),
            StorageError::BadMagic => write!(f, "bad magic/signature"),
            StorageError::Unsupported(what) => write!(f, "unsupported: {what}"),
            StorageError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}
impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self { StorageError::Io(e) }
}

/// Guess [`MediaKind`] from a filename/extension.
pub fn kind_from_name(name: &str) -> Option<MediaKind> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "prg" | "bas" => MediaKind::Prg,
        "d64" => MediaKind::D64,
        "t64" => MediaKind::T64,
        "tap" => MediaKind::Tap,
        "crt" => MediaKind::Crt,
        _ => return None,
    })
}
