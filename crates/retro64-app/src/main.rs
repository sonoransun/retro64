//! Retro 64 SDL2 desktop binary.

use clap::Parser;

mod app;
mod audio;
mod cli;
mod hostfs;
mod input;
mod video;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = cli::Args::parse();
    if let Err(e) = app::run(args) {
        eprintln!("retro64: {e}");
        std::process::exit(1);
    }
}
