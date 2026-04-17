//! Host filesystem trait for virtual IEC device #10.

use std::collections::BTreeMap;

/// Abstract host filesystem.
pub trait HostFs {
    /// List file names in the root.
    fn list(&self) -> Vec<String>;
    /// Read a file. Returns None if it doesn't exist.
    fn read(&self, name: &str) -> Option<Vec<u8>>;
    /// Write a file, creating or overwriting. Returns Err on failure.
    fn write(&mut self, name: &str, data: &[u8]) -> std::io::Result<()>;
}

/// No-op backend.
pub struct NullHostFs;
impl HostFs for NullHostFs {
    fn list(&self) -> Vec<String> { Vec::new() }
    fn read(&self, _name: &str) -> Option<Vec<u8>> { None }
    fn write(&mut self, _name: &str, _data: &[u8]) -> std::io::Result<()> { Ok(()) }
}

/// In-memory backend (used by the web frontend).
#[derive(Default)]
pub struct InMemoryHostFs {
    files: BTreeMap<String, Vec<u8>>,
}

impl HostFs for InMemoryHostFs {
    fn list(&self) -> Vec<String> {
        self.files.keys().cloned().collect()
    }
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.files.get(name).cloned()
    }
    fn write(&mut self, name: &str, data: &[u8]) -> std::io::Result<()> {
        self.files.insert(name.to_string(), data.to_vec());
        Ok(())
    }
}
