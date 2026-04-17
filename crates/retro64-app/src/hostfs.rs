//! Filesystem-backed HostFs for the desktop frontend.

use std::path::PathBuf;

use retro64_core::extensions::HostFs;

pub struct FsDirHostFs {
    root: PathBuf,
}

impl FsDirHostFs {
    pub fn new(root: PathBuf) -> Self { FsDirHostFs { root } }
}

impl HostFs for FsDirHostFs {
    fn list(&self) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(&self.root) else { return Vec::new(); };
        rd.filter_map(|e| e.ok())
          .filter_map(|e| e.file_name().into_string().ok())
          .collect()
    }
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.root.join(name)).ok()
    }
    fn write(&mut self, name: &str, data: &[u8]) -> std::io::Result<()> {
        std::fs::write(self.root.join(name), data)
    }
}
