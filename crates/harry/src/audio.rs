//! Sound output: a small mixer for the sounds a frame asks for
//! (`harry_core::sound`), stereo at the Mac's 22254 Hz. The original plays
//! through SAT's sound channels; the port mixes up to [`VOICES`] sounds,
//! dropping the oldest effect when they are all busy. A voice, the chatter,
//! a remark and the charge sound each replace the one before.

use std::rc::Rc;

use anyhow::Result;
use harry_core::sound::{Channel, RATE, SoundEvent};
use harry_data::{GameData, Sound};
use sdl3::audio::{AudioFormat, AudioSpec, AudioStreamOwner};

const OUTPUT_RATE: u32 = 22254;
/// Sounds at once.
const VOICES: usize = 8;
/// Mixed ahead of the device: enough for a late frame, little enough that
/// sounds start with the picture.
const AHEAD: f64 = 0.05;

struct Voice {
    id: i16,
    samples: Rc<Vec<u8>>,
    pos: f64,
    step: f64,
    left: f32,
    right: f32,
    channel: Channel,
}

pub struct Mixer {
    stream: Option<AudioStreamOwner>,
    voices: Vec<Voice>,
    sounds: Vec<(i16, Rc<Vec<u8>>, u32)>,
    world: Vec<(i16, Rc<Vec<u8>>, u32)>,
}

fn table(set: &std::collections::BTreeMap<i16, Sound>) -> Vec<(i16, Rc<Vec<u8>>, u32)> {
    set.iter().map(|(&id, s)| (id, Rc::new(s.samples.clone()), s.rate)).collect()
}

impl Mixer {
    /// Open the default output; without one the game is silent.
    pub fn open(sdl: &sdl3::Sdl, data: &GameData) -> Mixer {
        sdl3::hint::set("SDL_AUDIO_DEVICE_SAMPLE_FRAMES", "256");
        let stream = (|| -> Result<AudioStreamOwner> {
            let spec = AudioSpec { freq: Some(OUTPUT_RATE as i32), channels: Some(2), format: Some(AudioFormat::s16_sys()) };
            let stream = sdl.audio()?.default_playback_device().open_device_stream(Some(&spec))?;
            stream.resume()?;
            Ok(stream)
        })();
        let stream = stream.map_err(|e| eprintln!("no sound: {e}")).ok();
        Mixer { stream, voices: Vec::new(), sounds: table(&data.sounds), world: table(&data.world_sounds) }
    }

    fn find(&self, id: i16, own: bool) -> Option<(Rc<Vec<u8>>, u32)> {
        let look = |t: &Vec<(i16, Rc<Vec<u8>>, u32)>| t.iter().find(|x| x.0 == id).map(|x| (x.1.clone(), x.2));
        if own { look(&self.world).or_else(|| look(&self.sounds)) } else { look(&self.sounds) }
    }

    /// The sounds of a frame; `own` = the level brings its own sounds.
    pub fn events(&mut self, events: &[SoundEvent], own: bool) {
        for e in events {
            match *e {
                SoundEvent::Play { id, left, right, rate, channel } => {
                    let Some((samples, sample_rate)) = self.find(id, own) else { continue };
                    if channel != Channel::Effect {
                        self.voices.retain(|v| v.channel != channel);
                    }
                    if self.voices.len() >= VOICES {
                        if let Some(p) = self.voices.iter().position(|v| v.channel == Channel::Effect) {
                            self.voices.remove(p);
                        } else {
                            continue;
                        }
                    }
                    let step = f64::from(sample_rate) / f64::from(OUTPUT_RATE) * f64::from(rate) / f64::from(RATE);
                    self.voices.push(Voice {
                        id,
                        samples,
                        pos: 0.0,
                        step,
                        left: f32::from(left) / 256.0,
                        right: f32::from(right) / 256.0,
                        channel,
                    });
                }
                SoundEvent::HaltVoices => self.voices.retain(|v| !matches!(v.channel, Channel::Voice | Channel::Chatter)),
                SoundEvent::StopCharge => self.voices.retain(|v| v.channel != Channel::Charge),
                SoundEvent::Stop(id) => self.voices.retain(|v| v.id != id),
            }
        }
    }

    /// Mix until [`AHEAD`] seconds are queued.
    pub fn fill(&mut self) {
        let Some(stream) = &self.stream else { return };
        let ahead = (f64::from(OUTPUT_RATE) * AHEAD) as i32 * 4;
        let queued = stream.queued_bytes().unwrap_or(ahead);
        if queued >= ahead {
            return;
        }
        let frames = ((ahead - queued) / 4) as usize;
        let mut out = vec![0i16; frames * 2];
        for f in 0..frames {
            let (mut l, mut r) = (0.0f32, 0.0f32);
            for v in &mut self.voices {
                let i = v.pos as usize;
                if let Some(&s) = v.samples.get(i) {
                    let x = (f32::from(s) - 128.0) * 256.0;
                    l += x * v.left;
                    r += x * v.right;
                }
                v.pos += v.step;
            }
            out[2 * f] = l.clamp(-32768.0, 32767.0) as i16;
            out[2 * f + 1] = r.clamp(-32768.0, 32767.0) as i16;
            if f % 64 == 0 {
                self.voices.retain(|v| (v.pos as usize) < v.samples.len());
            }
        }
        self.voices.retain(|v| (v.pos as usize) < v.samples.len());
        let _ = stream.put_data_i16(&out);
    }
}
