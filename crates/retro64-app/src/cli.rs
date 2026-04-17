//! CLI argument parsing.

use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "retro64", version, about = "Retro 64 — Commodore 64 emulator")]
pub struct Args {
    /// PRG/D64/T64 file to load at startup.
    #[arg(long)]
    pub load: Option<PathBuf>,

    /// Directory containing original Commodore ROMs (basic, kernal, chargen).
    #[arg(long)]
    pub rom_dir: Option<PathBuf>,

    /// Model: pal or ntsc.
    #[arg(long, default_value = "pal")]
    pub model: String,

    /// Run at maximum speed (frame limiter disabled).
    #[arg(long, default_value_t = false)]
    pub warp: bool,

    /// Enable host filesystem and compute offload extensions.
    #[arg(long, default_value_t = false)]
    pub extensions: bool,

    /// Directory exposed via virtual IEC device #10.
    #[arg(long)]
    pub hostfs: Option<PathBuf>,

    /// Joystick port (1 or 2).
    #[arg(long, default_value_t = 2)]
    pub joystick_port: u8,

    /// Window scale factor.
    #[arg(long, default_value_t = 2)]
    pub scale: u32,
}

use clap::Parser;
