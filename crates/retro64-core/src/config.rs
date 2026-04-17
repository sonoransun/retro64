//! Configuration types for [`crate::C64`].

use std::path::PathBuf;

/// Commodore 64 hardware model.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Model {
    /// 50 Hz European model: 985248 Hz, 312 scanlines, 63 cycles/line.
    Pal,
    /// 60 Hz North American model: 1022727 Hz, 263 scanlines, 65 cycles/line.
    Ntsc,
}

/// Which CIA1 port the host joystick is wired to.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JoystickPort {
    /// CIA1 Port A (bits 0-4).
    Port1,
    /// CIA1 Port B (bits 0-4).
    Port2,
}

impl Model {
    /// CPU clock frequency (approximate) in Hz.
    pub const fn cpu_hz(self) -> u32 {
        match self {
            Model::Pal => 985_248,
            Model::Ntsc => 1_022_727,
        }
    }
    /// Number of scanlines per frame (including vertical blank).
    pub const fn lines_per_frame(self) -> u32 {
        match self {
            Model::Pal => 312,
            Model::Ntsc => 263,
        }
    }
    /// Nominal CPU cycles per scanline.
    pub const fn cycles_per_line(self) -> u32 {
        match self {
            Model::Pal => 63,
            Model::Ntsc => 65,
        }
    }
    /// Nominal frames per second.
    pub fn fps(self) -> f32 {
        self.cpu_hz() as f32 / (self.lines_per_frame() * self.cycles_per_line()) as f32
    }
    /// Framebuffer width including horizontal borders.
    pub const fn screen_width(self) -> u32 {
        match self {
            Model::Pal => 403,
            Model::Ntsc => 411,
        }
    }
    /// Framebuffer height including vertical borders.
    pub const fn screen_height(self) -> u32 {
        match self {
            Model::Pal => 284,
            Model::Ntsc => 234,
        }
    }
}

/// Runtime configuration passed to [`crate::C64::new`].
#[derive(Clone, Debug)]
pub struct Config {
    /// Hardware model (PAL or NTSC).
    pub model: Model,
    /// If true, frame limiter is bypassed (frontend hint).
    pub warp: bool,
    /// Enable host filesystem (device #10) and compute offload ($DE00).
    pub extensions_enabled: bool,
    /// Host filesystem root directory (only honoured by [`crate::extensions`]
    /// backends that can read the host FS).
    pub hostfs_root: Option<PathBuf>,
    /// Which joystick port the host joystick maps to.
    pub joystick_port: JoystickPort,
    /// ROM directory containing `basic`, `kernal`, `chargen` files.
    pub rom_dir: Option<PathBuf>,
    /// Desktop window scale factor.
    pub scale: u32,
    /// Audio output sample rate in Hz.
    pub sample_rate: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            model: Model::Pal,
            warp: false,
            extensions_enabled: false,
            hostfs_root: None,
            joystick_port: JoystickPort::Port2,
            rom_dir: None,
            scale: 2,
            sample_rate: 48_000,
        }
    }
}
