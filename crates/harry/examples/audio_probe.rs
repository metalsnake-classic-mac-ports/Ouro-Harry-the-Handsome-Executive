//! How SDL drives the audio callback on this machine: the device's
//! buffer and the callback's requests (outputs silence).

use std::sync::{Arc, Mutex};
use std::time::Instant;

use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream};

struct Probe(Arc<Mutex<Vec<(f64, i32, i32)>>>, Instant);

impl AudioCallback<i16> for Probe {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        let queued = stream.queued_bytes().unwrap_or(-1);
        self.0.lock().unwrap().push((self.1.elapsed().as_secs_f64() * 1000.0, requested, queued));
        let _ = stream.put_data_i16(&vec![0i16; requested.max(0) as usize]);
    }
}

fn main() {
    sdl3::hint::set("SDL_AUDIO_DEVICE_SAMPLE_FRAMES", "256");
    let sdl = sdl3::init().unwrap();
    let audio = sdl.audio().unwrap();
    let mut spec = sdl3::sys::audio::SDL_AudioSpec { format: sdl3::sys::audio::SDL_AUDIO_S16, channels: 0, freq: 0 };
    let mut frames = 0;
    unsafe {
        sdl3::sys::audio::SDL_GetAudioDeviceFormat(sdl3::sys::audio::SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &mut spec, &mut frames);
    }
    println!("device: {} Hz, {} channels, {} sample frames", spec.freq, spec.channels, frames);
    let log = Arc::new(Mutex::new(Vec::new()));
    let want = AudioSpec { freq: Some(22254), channels: Some(2), format: Some(AudioFormat::s16_sys()) };
    let s = audio.open_playback_stream(&want, Probe(log.clone(), Instant::now())).unwrap();
    s.resume().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    let l = log.lock().unwrap();
    println!("{} callbacks in 500 ms", l.len());
    for e in l.iter().take(12) {
        println!("  t={:7.1} ms requested={} queued={}", e.0, e.1, e.2);
    }
}
