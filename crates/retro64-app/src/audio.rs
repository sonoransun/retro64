//! SDL2 audio output. Pulls samples from a shared ring filled by the main loop.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};

type Ring = Arc<Mutex<VecDeque<i16>>>;

struct Sink { ring: Ring }

impl AudioCallback for Sink {
    type Channel = i16;
    fn callback(&mut self, out: &mut [i16]) {
        let mut r = self.ring.lock().unwrap();
        for s in out.iter_mut() {
            *s = r.pop_front().unwrap_or(0);
        }
    }
}

/// Handle returned to the app: exposes `push` for the main loop.
pub struct AudioSink {
    _device: AudioDevice<Sink>,
    ring: Ring,
    cap: usize,
}

impl AudioSink {
    pub fn push(&self, samples: &[i16]) {
        let mut r = self.ring.lock().unwrap();
        for s in samples {
            if r.len() >= self.cap { r.pop_front(); }
            r.push_back(*s);
        }
    }
}

/// Initialise SDL audio at `sample_rate` Hz mono.
pub fn start(sdl: &sdl2::Sdl, sample_rate: u32) -> Result<AudioSink, String> {
    let audio = sdl.audio()?;
    let spec = AudioSpecDesired {
        freq: Some(sample_rate as i32),
        channels: Some(1),
        samples: Some(1024),
    };
    let ring: Ring = Arc::new(Mutex::new(VecDeque::with_capacity(sample_rate as usize)));
    let ring_cb = ring.clone();
    let device = audio.open_playback(None, &spec, |_| Sink { ring: ring_cb })?;
    device.resume();
    Ok(AudioSink { _device: device, ring, cap: sample_rate as usize })
}
