//! Integration smoke tests: boot the emulator and verify basic behaviour
//! without real Commodore ROMs.

use retro64_core::{C64, Config, Model};

#[test]
fn c64_boots_with_stub_roms() {
    let mut cfg = Config::default();
    cfg.model = Model::Pal;
    let mut emu = C64::new(cfg);
    // Run ten frames; the stub KERNAL does an infinite JMP loop, which is
    // fine — we just need the system to not panic.
    for _ in 0..10 {
        let _ = emu.run_frame();
    }
    assert_eq!(emu.screen_width(), 403);
    assert_eq!(emu.screen_height(), 284);
}

#[test]
fn ntsc_dimensions() {
    let mut cfg = Config::default();
    cfg.model = Model::Ntsc;
    let emu = C64::new(cfg);
    assert_eq!(emu.screen_width(), 411);
    assert_eq!(emu.screen_height(), 234);
    assert!((emu.target_fps() - 59.826).abs() < 0.1);
}

#[test]
fn framebuffer_is_populated() {
    let mut emu = C64::new(Config::default());
    let fb = emu.run_frame();
    assert_eq!(fb.len(), (403 * 284) as usize);
    // At least the border colour (non-zero).
    assert!(fb.iter().any(|p| *p != 0));
}

#[test]
fn load_prg_accepts_basic_program() {
    // PRG: load address $0801, body = one BASIC byte
    let prg = [0x01u8, 0x08, 0x00, 0x00];
    let mut emu = C64::new(Config::default());
    assert!(emu.load_prg(&prg).is_ok());
}

#[test]
fn audio_drain_returns_samples_per_frame() {
    let mut emu = C64::new(Config::default());
    emu.run_frame();
    let s = emu.drain_audio();
    // PAL: 985248/50 ≈ 19704 CPU cycles/frame. At 48k samples → ~960/frame.
    assert!(s.len() > 500 && s.len() < 1100,
            "got {} samples (expected ~960)", s.len());
}
