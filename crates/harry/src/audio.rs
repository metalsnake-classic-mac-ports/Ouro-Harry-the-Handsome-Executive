//! Sound output: a small mixer for the sounds a frame asks for
//! (`harry_core::sound`), stereo at the Mac's 22254 Hz. The original plays
//! through SAT's sound channels; the port mixes up to [`VOICES`] sounds,
//! dropping the oldest effect when they are all busy. A voice, the chatter,
//! a remark and the charge sound each replace the one before. What still
//! plays goes back to the game before each frame ([`Mixer::playing`]):
//! the original's routines ask their mixer.
//!
//! The mixing runs in SDL's audio callback, so a sound starts within one
//! buffer of the device (256 frames). The music (`crate::music`) is mixed
//! in the same callback.

use std::sync::Arc;

use harry_core::sound::{Channel, Playing, RATE, SoundEvent};
use harry_data::{GameData, Sound};
use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream, AudioStreamWithCallback};

const OUTPUT_RATE: u32 = 22254;
/// Sounds at once.
const VOICES: usize = 8;

struct Voice {
    id: i16,
    /// The game's handle (0: a menu sound).
    handle: u32,
    samples: Arc<Vec<u8>>,
    pos: f64,
    step: f64,
    left: f32,
    right: f32,
    channel: Channel,
}

/// The voices and the music, mixed by the audio thread.
struct Voices(Vec<Voice>, crate::music::Player);

impl AudioCallback<i16> for Voices {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        let frames = requested.max(0) as usize / 2;
        let mut music = vec![0.0f32; frames * 2];
        self.1.mix(&mut music);
        let mut out = vec![0i16; frames * 2];
        for f in 0..frames {
            let (mut l, mut r) = (music[2 * f] * 32768.0, music[2 * f + 1] * 32768.0);
            for v in &mut self.0 {
                if let Some(&s) = v.samples.get(v.pos as usize) {
                    let x = (f32::from(s) - 128.0) * 256.0;
                    l += x * v.left;
                    r += x * v.right;
                }
                v.pos += v.step;
            }
            out[2 * f] = l.clamp(-32768.0, 32767.0) as i16;
            out[2 * f + 1] = r.clamp(-32768.0, 32767.0) as i16;
        }
        self.0.retain(|v| (v.pos as usize) < v.samples.len());
        let _ = stream.put_data_i16(&out);
    }
}

type Table = Vec<(i16, Arc<Vec<u8>>, u32)>;

pub struct Mixer {
    stream: Option<AudioStreamWithCallback<Voices>>,
    sounds: Table,
    world: Table,
    /// The sound volume setting (0 – 1).
    volume: f32,
}

fn table(set: &std::collections::BTreeMap<i16, Sound>) -> Table {
    set.iter().map(|(&id, s)| (id, Arc::new(s.samples.clone()), s.rate)).collect()
}

impl Mixer {
    /// Open the default output; without one the game is silent.
    pub fn open(sdl: &sdl3::Sdl, data: &GameData) -> Mixer {
        sdl3::hint::set("SDL_AUDIO_DEVICE_SAMPLE_FRAMES", "256");
        let spec = AudioSpec { freq: Some(OUTPUT_RATE as i32), channels: Some(2), format: Some(AudioFormat::s16_sys()) };
        let stream = sdl
            .audio()
            .map_err(|e| e.to_string())
            .and_then(|a| {
                let player = crate::music::Player::new(crate::music::Library::new(&data.music), OUTPUT_RATE);
                a.open_playback_stream(&spec, Voices(Vec::new(), player)).map_err(|e| e.to_string())
            })
            .and_then(|s| s.resume().map(|_| s).map_err(|e| e.to_string()));
        let stream = stream.map_err(|e| eprintln!("no sound: {e}")).ok();
        Mixer { stream, sounds: table(&data.sounds), world: table(&data.world_sounds), volume: 1.0 }
    }

    /// A mixer without an output (for tests): nothing plays.
    #[cfg(test)]
    pub fn silent(data: &GameData) -> Mixer {
        Mixer { stream: None, sounds: table(&data.sounds), world: table(&data.world_sounds), volume: 1.0 }
    }

    /// `SetMusic(id)`: the tune from its start (silent while faded out).
    pub fn music(&mut self, id: i16) {
        if let Some(mut v) = self.stream.as_mut().and_then(|s| s.lock()) {
            v.1.play(id);
        }
    }

    /// `FadeMusic`: out (or in) in 24 steps of `ticks`.
    pub fn fade_music(&mut self, out: bool, ticks: u16) {
        if let Some(mut v) = self.stream.as_mut().and_then(|s| s.lock()) {
            v.1.fade(out, ticks);
        }
    }

    /// The music volume setting, 0 (off) – 9.
    pub fn set_music_volume(&mut self, vol: u8) {
        if let Some(mut v) = self.stream.as_mut().and_then(|s| s.lock()) {
            v.1.set_volume(vol);
        }
    }

    /// The volume setting, 0 (off) – 9 (full), for the sounds started from
    /// now on.
    pub fn set_volume(&mut self, v: u8) {
        self.volume = f32::from(v.min(9)) / 9.0;
    }

    /// The sounds that still play, for the game's routines that ask.
    pub fn playing(&mut self) -> Playing {
        let Some(mut all) = self.stream.as_mut().and_then(|s| s.lock()) else { return Playing::default() };
        all.0.retain(|v| (v.pos as usize) < v.samples.len());
        Playing(all.0.iter().map(|v| harry_core::sound::Voice { handle: v.handle, id: v.id, channel: v.channel }).collect())
    }

    fn find(&self, id: i16, own: bool) -> Option<(Arc<Vec<u8>>, u32)> {
        let look = |t: &Table| t.iter().find(|x| x.0 == id).map(|x| (x.1.clone(), x.2));
        if own { look(&self.world).or_else(|| look(&self.sounds)) } else { look(&self.sounds) }
    }

    /// The sounds of a frame; `own` = the level brings its own sounds.
    pub fn events(&mut self, events: &[SoundEvent], own: bool) {
        if events.is_empty() {
            return;
        }
        let new: Vec<Option<(Arc<Vec<u8>>, u32)>> = events
            .iter()
            .map(|e| match *e {
                SoundEvent::Play { id, .. } => self.find(id, own),
                _ => None,
            })
            .collect();
        let volume = self.volume;
        let Some(stream) = &mut self.stream else { return };
        let Some(mut all) = stream.lock() else { return };
        let mut fade = None;
        let voices = &mut all.0;
        for (e, found) in events.iter().zip(new) {
            match *e {
                SoundEvent::Play { id, left, right, rate, channel, handle } => {
                    let Some((samples, sample_rate)) = found else { continue };
                    if channel != Channel::Effect {
                        voices.retain(|v| v.channel != channel);
                    }
                    if voices.len() >= VOICES {
                        match voices.iter().position(|v| v.channel == Channel::Effect) {
                            Some(p) => {
                                voices.remove(p);
                            }
                            None => continue,
                        }
                    }
                    let step = f64::from(sample_rate) / f64::from(OUTPUT_RATE) * f64::from(rate) / f64::from(RATE);
                    voices.push(Voice {
                        id,
                        handle,
                        samples,
                        pos: 0.0,
                        step,
                        left: f32::from(left) / 256.0 * volume,
                        right: f32::from(right) / 256.0 * volume,
                        channel,
                    });
                }
                SoundEvent::HaltVoices => voices.retain(|v| !matches!(v.channel, Channel::Voice | Channel::Chatter)),
                SoundEvent::StopCharge => voices.retain(|v| v.channel != Channel::Charge),
                SoundEvent::Stop(id) => voices.retain(|v| v.id != id),
                SoundEvent::StopHandle(h) => voices.retain(|v| h != 0 && v.handle != h),
                SoundEvent::Pan { handle, left, right } => {
                    for v in voices.iter_mut().filter(|v| v.handle == handle) {
                        v.left = f32::from(left) / 256.0 * volume;
                        v.right = f32::from(right) / 256.0 * volume;
                    }
                }
                // the music fades without stopping the game (the original
                // waits for it; NOTES.md, "Port improvements")
                SoundEvent::FadeMusicOut(ticks) => fade = Some(ticks),
            }
        }
        if let Some(ticks) = fade {
            all.1.fade(true, ticks);
        }
    }
}
