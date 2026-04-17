//! Platform-independent Commodore 64 emulation core.
//!
//! The [`C64`] struct wires all chips together and drives frame-by-frame
//! emulation. See the crate root re-exports for the stable public API.

#![allow(clippy::too_many_arguments)]

pub mod config;
pub mod cia;
pub mod cpu;
pub mod extensions;
pub mod memory;
pub mod sid;
pub mod storage;
pub mod system;
pub mod util;
pub mod vic;

pub use cia::keyboard::{C64Key, Modifier};
pub use config::{Config, JoystickPort, Model};
pub use storage::{MediaKind, StorageError};
pub use system::C64;
pub use vic::export::{argb_to_rgba, framebuffer_to_bmp};
