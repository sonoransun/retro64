//! Retro 64 extensions: compute offload ($DE00-$DEFF) and host filesystem
//! (virtual IEC device #10).

pub mod compute;
pub mod hostfs;

pub use compute::Compute;
pub use hostfs::{HostFs, InMemoryHostFs, NullHostFs};

/// Aggregate extension state.
pub struct Extensions {
    /// Compute-offload registers at $DE00-$DEFF.
    pub compute: Compute,
    /// Host filesystem backend. Swap with your own impl for platform access.
    pub hostfs: Box<dyn HostFs + Send>,
    /// Master enable.
    pub enabled: bool,
}

impl Extensions {
    /// Create a new Extensions struct with a [`NullHostFs`].
    pub fn new(enabled: bool) -> Self {
        Extensions {
            compute: Compute::new(),
            hostfs: Box::new(NullHostFs),
            enabled,
        }
    }

    /// Read a byte from $DE00-$DEFF.
    pub fn read(&self, addr: u16) -> u8 {
        if !self.enabled { return 0xFF; }
        self.compute.read(addr)
    }

    /// Write a byte to $DE00-$DEFF.
    pub fn write(&mut self, addr: u16, val: u8, ram: &mut [u8; 0x1_0000]) {
        if !self.enabled { return; }
        self.compute.write(addr, val, ram);
    }
}

impl Default for Extensions {
    fn default() -> Self { Self::new(false) }
}
