//! WebAssembly bindings for the Retro 64 core.

mod input;

use retro64_core::{C64, Config, JoystickPort, MediaKind, Model, storage::kind_from_name};
use wasm_bindgen::prelude::*;

/// Browser-facing emulator handle.
#[wasm_bindgen]
pub struct WebEmulator {
    inner: C64,
    rgba_buf: Vec<u8>,
}

#[wasm_bindgen]
impl WebEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(model: &str) -> Self {
        console_error_panic_hook::set_once();
        let mut cfg = Config::default();
        cfg.model = if model.eq_ignore_ascii_case("ntsc") { Model::Ntsc } else { Model::Pal };
        let inner = C64::new(cfg);
        let n = (inner.screen_width() * inner.screen_height()) as usize;
        WebEmulator { inner, rgba_buf: vec![0; n * 4] }
    }

    pub fn run_frame(&mut self) {
        let _ = self.inner.run_frame();
        retro64_core::argb_to_rgba(self.inner.framebuffer(), &mut self.rgba_buf);
    }

    pub fn framebuffer_ptr(&self) -> *const u8 {
        self.rgba_buf.as_ptr()
    }

    pub fn framebuffer_len(&self) -> usize { self.rgba_buf.len() }

    pub fn drain_audio(&mut self) -> Vec<i16> { self.inner.drain_audio() }

    pub fn load_prg(&mut self, data: &[u8]) -> Result<(), JsValue> {
        self.inner.load_prg(data).map_err(|e| JsValue::from_str(&format!("{e}")))
    }

    pub fn load_media(&mut self, data: &[u8], filename: &str) -> Result<(), JsValue> {
        let kind = kind_from_name(filename).unwrap_or(MediaKind::Prg);
        self.inner.load_media(data, kind).map_err(|e| JsValue::from_str(&format!("{e}")))
    }

    pub fn key_down(&mut self, code: &str) {
        if let Some(k) = input::map_code(code) { self.inner.key_down(k); }
    }

    pub fn key_up(&mut self, code: &str) {
        if let Some(k) = input::map_code(code) { self.inner.key_up(k); }
    }

    pub fn joystick(&mut self, port: u8, bits: u8) {
        let p = if port == 1 { JoystickPort::Port1 } else { JoystickPort::Port2 };
        self.inner.joystick(p, bits);
    }

    pub fn trigger_nmi(&mut self) { self.inner.trigger_nmi(); }

    pub fn screen_width(&self) -> u32 { self.inner.screen_width() }
    pub fn screen_height(&self) -> u32 { self.inner.screen_height() }
    pub fn target_fps(&self) -> f32 { self.inner.target_fps() }

    pub fn reset(&mut self) { self.inner.reset(); }
}
