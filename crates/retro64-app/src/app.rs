//! Main application loop.

use std::time::{Duration, Instant};

use retro64_core::{C64, Config, JoystickPort, MediaKind, Model, storage::kind_from_name};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;

use crate::{audio, cli::Args, hostfs::FsDirHostFs, input};

pub fn run(args: Args) -> Result<(), String> {
    let mut cfg = Config::default();
    cfg.model = match args.model.to_ascii_lowercase().as_str() {
        "ntsc" => Model::Ntsc,
        _ => Model::Pal,
    };
    cfg.warp = args.warp;
    cfg.extensions_enabled = args.extensions;
    cfg.hostfs_root = args.hostfs.clone();
    cfg.joystick_port = if args.joystick_port == 1 { JoystickPort::Port1 } else { JoystickPort::Port2 };
    cfg.rom_dir = args.rom_dir.clone();
    cfg.scale = args.scale.max(1);

    let sample_rate = cfg.sample_rate;
    let mut emu = C64::new(cfg);
    if args.extensions {
        if let Some(root) = &args.hostfs {
            // Replace the hostfs backend with a real FS-backed one.
            let _ = FsDirHostFs::new(root.clone());
            // (Current core Extensions struct does not expose a setter; left as a hook.)
        }
    }

    if let Some(path) = &args.load {
        let bytes = std::fs::read(path).map_err(|e| format!("load {:?}: {e}", path))?;
        let kind = path.to_str().and_then(kind_from_name).unwrap_or(MediaKind::Prg);
        emu.load_media(&bytes, kind).map_err(|e| format!("{e}"))?;
    }

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let w = emu.screen_width();
    let h = emu.screen_height();
    let scale = args.scale.max(1);
    let window = video
        .window("Retro 64", w * scale, h * scale)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().accelerated().present_vsync().build()
        .map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::ARGB8888, w, h)
        .map_err(|e| e.to_string())?;

    let audio_sink = audio::start(&sdl, sample_rate)?;

    let mut event_pump = sdl.event_pump()?;
    let frame_dur = Duration::from_secs_f32(1.0 / emu.target_fps());
    let mut last = Instant::now();
    let mut warp = args.warp;

    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main,
                Event::KeyDown { keycode: Some(kc), .. } => match kc {
                    Keycode::Escape => break 'main,
                    Keycode::F11 => { warp = !warp; emu.set_warp(warp); }
                    Keycode::F12 => emu.trigger_nmi(),
                    _ => {
                        if let Some(k) = input::map(kc) { emu.key_down(k); }
                    }
                }
                Event::KeyUp { keycode: Some(kc), .. } => {
                    if let Some(k) = input::map(kc) { emu.key_up(k); }
                }
                _ => {}
            }
        }

        let fb = emu.run_frame();
        // SDL expects row bytes; ARGB8888 is 4 bytes per pixel.
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(fb.as_ptr() as *const u8, fb.len() * 4)
        };
        texture.update(None, bytes, (w * 4) as usize).map_err(|e| e.to_string())?;
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        canvas.present();

        // Push audio
        let samples = emu.drain_audio();
        audio_sink.push(&samples);

        if !warp {
            let elapsed = last.elapsed();
            if elapsed < frame_dur { std::thread::sleep(frame_dur - elapsed); }
            last = Instant::now();
        }
    }
    Ok(())
}
