//! The music synthesizer: plays a tune of the data pack with its sampled
//! instruments, as QuickTime's music synthesizer did for the original
//! (`TuneQueue` with looping, 600 units a second), or a recording of the
//! tune when the pack has one. Runs in the audio callback.
//!
//! What it does of QuickTime's synthesizer: the zone by key, the pitch from
//! the zone's normal key (and pitch bend, controller 32, ±2 semitones),
//! the sample's loop while it sounds, velocity, part volume (7) and pan
//! (10), a short release. Not yet: the instruments' knobs (envelopes,
//! velocity curves, transposition) and controller 33.

use std::collections::BTreeMap;
use std::sync::Arc;

use harry_data::music::{Music, MusicEvent, Recording, Tune, Zone};

/// Time units of a tune a second.
const UNITS: f64 = 600.0;
/// Notes at once (the oldest goes).
const MAX_VOICES: usize = 48;
/// Release after a note's end, in seconds.
const RELEASE: f64 = 0.08;
/// Overall level of the music against the sound effects.
const LEVEL: f32 = 0.3;

/// An instrument's zones with their samples.
type Zones = Arc<Vec<(Zone, Arc<Vec<u8>>)>>;

/// The music as the audio thread holds it.
pub struct Library {
    tunes: BTreeMap<i16, Arc<Tune>>,
    zones: BTreeMap<String, Zones>,
    recordings: BTreeMap<i16, Arc<Recording>>,
}

impl Library {
    pub fn new(m: &Music) -> Library {
        let samples: BTreeMap<&String, Arc<Vec<u8>>> = m.samples.iter().map(|(k, s)| (k, Arc::new(s.samples.clone()))).collect();
        let zones = m
            .instruments
            .iter()
            .map(|(k, i)| {
                let z = i.zones.iter().filter_map(|z| Some((z.clone(), samples.get(&z.sample)?.clone()))).collect();
                (k.clone(), Arc::new(z))
            })
            .collect();
        Library {
            tunes: m.tunes.iter().map(|(&id, t)| (id, Arc::new(t.clone()))).collect(),
            zones,
            recordings: m.recordings.iter().map(|(&id, r)| (id, Arc::new(r.clone()))).collect(),
        }
    }
}

#[derive(Clone, Copy)]
struct PartState {
    volume: f32,
    pan: f32,
    bend: f32,
}

impl Default for PartState {
    fn default() -> Self {
        PartState { volume: 100.0 / 127.0, pan: 0.5, bend: 0.0 }
    }
}

struct Voice {
    part: u16,
    data: Arc<Vec<u8>>,
    pos: f64,
    /// Samples a frame at the zone's key, without pitch bend.
    step: f64,
    gain: f32,
    /// Frames left to hold; then the release.
    hold: u64,
    release: u64,
    released: u64,
    /// The loop (start, end) while the note sounds.
    looped: Option<(f64, f64)>,
}

/// What plays.
enum Source {
    Tune {
        tune: Arc<Tune>,
        instruments: BTreeMap<u16, Zones>,
        parts: BTreeMap<u16, PartState>,
        next: usize,
        /// Units to wait before the next event.
        wait: f64,
        voices: Vec<Voice>,
    },
    Recording { r: Arc<Recording>, pos: f64 },
}

pub struct Player {
    lib: Library,
    rate: f64,
    source: Option<Source>,
    pub playing: Option<i16>,
    /// The volume setting (0 – 1, `SetMusicVolume`'s) and the fade (0 – 1).
    volume: f32,
    fade: f32,
    fade_step: f32,
}

impl Player {
    pub fn new(lib: Library, rate: u32) -> Player {
        Player { lib, rate: f64::from(rate), source: None, playing: None, volume: 1.0, fade: 1.0, fade_step: 0.0 }
    }

    /// `SetMusic(id)`: from its start (a recording if there is one);
    /// silent when the music is faded out.
    pub fn play(&mut self, id: i16) {
        self.playing = Some(id);
        self.source = if let Some(r) = self.lib.recordings.get(&id) {
            Some(Source::Recording { r: r.clone(), pos: 0.0 })
        } else if let Some(t) = self.lib.tunes.get(&id) {
            let instruments = t.parts.iter().filter_map(|(&p, k)| Some((p, self.lib.zones.get(k)?.clone()))).collect();
            Some(Source::Tune { tune: t.clone(), instruments, parts: BTreeMap::new(), next: 0, wait: 0.0, voices: Vec::new() })
        } else {
            None
        };
    }

    /// The volume setting, 0 (off) – 9 (`TuneSetVolume(v · 0x1c71 − 3000)`).
    pub fn set_volume(&mut self, v: u8) {
        self.volume = ((i32::from(v.min(9)) * 0x1c71 - 3000).max(0) as f32) / 65536.0;
    }

    /// `FadeMusic`: to silence (`out`) or back, in 24 steps of `ticks`
    /// 1/60 s.
    pub fn fade(&mut self, out: bool, ticks: u16) {
        let frames = (f64::from(ticks.max(1)) * 24.0 / 60.0 * self.rate) as f32;
        self.fade_step = if out { -1.0 / frames } else { 1.0 / frames };
    }

    /// Mix the next frames into `out` (stereo, interleaved).
    pub fn mix(&mut self, out: &mut [f32]) {
        let rate = self.rate;
        for f in out.as_chunks_mut::<2>().0 {
            self.fade = (self.fade + self.fade_step).clamp(0.0, 1.0);
            if self.fade == 0.0 || self.fade == 1.0 {
                self.fade_step = 0.0;
            }
            let gain = self.volume * self.fade * LEVEL;
            let Some(src) = &mut self.source else { return };
            let (l, r) = match src {
                Source::Recording { r, pos } => {
                    let ch = usize::from(r.channels.max(1));
                    let frames = r.samples.len() / ch;
                    if frames == 0 {
                        return;
                    }
                    let i = (*pos as usize % frames) * ch;
                    let s = |c: usize| f32::from(r.samples[i + c.min(ch - 1)]) / 32768.0;
                    *pos += f64::from(r.rate) / rate;
                    if *pos as usize >= frames {
                        *pos -= frames as f64;
                    }
                    (s(0), s(1))
                }
                Source::Tune { tune, instruments, parts, next, wait, voices } => {
                    // the events due now; at the end the tune starts again
                    // (notes still sounding go on)
                    *wait -= UNITS / rate;
                    let mut guard = 0;
                    while *wait <= 0.0 && guard < 100_000 {
                        guard += 1;
                        if *next >= tune.events.len() {
                            *next = 0;
                            if tune.length() == 0 {
                                break;
                            }
                        }
                        match tune.events[*next] {
                            MusicEvent::Rest(n) => *wait += f64::from(n),
                            MusicEvent::Controller { part, controller, value } => {
                                let p = parts.entry(part).or_default();
                                match controller {
                                    7 => p.volume = f32::from(value) / 256.0 / 127.0,
                                    10 => p.pan = ((f32::from(value) - 256.0) / 256.0).clamp(0.0, 1.0),
                                    32 => p.bend = f32::from(value) / 256.0,
                                    _ => {}
                                }
                            }
                            MusicEvent::Note { part, pitch, velocity, duration } => {
                                if velocity > 0
                                    && let Some(zones) = instruments.get(&part)
                                {
                                    let key = f64::from(pitch) / 256.0;
                                    let k = key.round() as u8;
                                    if let Some((z, data)) = zones.iter().find(|(z, _)| (z.low..=z.high).contains(&k)) {
                                        let step = f64::from(z.rate) / 65536.0 / rate * 2f64.powf((key - f64::from(z.normal)) / 12.0);
                                        let looped = (z.loop_type == 0 && z.loop_end > z.loop_start + 1 && z.loop_start > 0)
                                            .then_some((f64::from(z.loop_start), f64::from(z.loop_end)));
                                        if voices.len() >= MAX_VOICES {
                                            voices.remove(0);
                                        }
                                        voices.push(Voice {
                                            part,
                                            data: data.clone(),
                                            pos: 0.0,
                                            step,
                                            gain: f32::from(velocity) / 127.0,
                                            hold: (f64::from(duration) / UNITS * rate) as u64,
                                            release: (RELEASE * rate) as u64,
                                            released: 0,
                                            looped,
                                        });
                                    }
                                }
                            }
                        }
                        *next += 1;
                    }
                    let (mut l, mut r) = (0.0f32, 0.0f32);
                    for v in voices.iter_mut() {
                        let p = parts.get(&v.part).copied().unwrap_or_default();
                        let i = v.pos as usize;
                        let Some(&a) = v.data.get(i) else {
                            v.release = 0;
                            continue;
                        };
                        let b = v.data.get(i + 1).copied().unwrap_or(a);
                        let fr = (v.pos - i as f64) as f32;
                        let x = ((f32::from(a) - 128.0) * (1.0 - fr) + (f32::from(b) - 128.0) * fr) / 128.0;
                        let env = if v.hold > 0 {
                            v.hold -= 1;
                            1.0
                        } else {
                            v.released += 1;
                            1.0 - v.released as f32 / v.release.max(1) as f32
                        };
                        let g = x * v.gain * p.volume * env.max(0.0);
                        l += g * (1.0 - p.pan).sqrt();
                        r += g * p.pan.sqrt();
                        v.pos += v.step * 2f64.powf(f64::from(p.bend) / 12.0);
                        if let Some((ls, le)) = v.looped
                            && v.pos >= le
                        {
                            v.pos -= le - ls;
                        }
                    }
                    voices.retain(|v| v.released < v.release && (v.pos as usize) < v.data.len());
                    (l, r)
                }
            };
            f[0] += l * gain;
            f[1] += r * gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> Option<harry_data::GameData> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re/work/harry-data.zip");
        harry_data::pack::read(&std::fs::read(root).ok()?).ok()
    }

    /// Render `seconds` of a tune (`HARRY_MUSIC_DUMP=<dir>`: as a WAV file).
    fn render(data: &harry_data::GameData, id: i16, seconds: f64) -> Vec<f32> {
        let mut p = Player::new(Library::new(&data.music), 22254);
        p.set_volume(9);
        p.play(id);
        let mut out = vec![0.0f32; (seconds * 22254.0) as usize * 2];
        p.mix(&mut out);
        if let Ok(dir) = std::env::var("HARRY_MUSIC_DUMP") {
            let r = Recording { rate: 22254, channels: 2, samples: out.iter().map(|x| (x.clamp(-1.0, 1.0) * 32767.0) as i16).collect() };
            let mut w = Vec::new();
            w.extend_from_slice(b"RIFF");
            w.extend_from_slice(&(36 + r.samples.len() as u32 * 2).to_le_bytes());
            w.extend_from_slice(b"WAVEfmt ");
            for v in [16u32, 0x0002_0001, 22254, 22254 * 4, 0x0010_0004] {
                w.extend_from_slice(&v.to_le_bytes());
            }
            w.extend_from_slice(b"data");
            w.extend_from_slice(&(r.samples.len() as u32 * 2).to_le_bytes());
            for s in r.samples {
                w.extend_from_slice(&s.to_le_bytes());
            }
            std::fs::write(std::path::Path::new(&dir).join(format!("tune{id}.wav")), w).unwrap();
        }
        out
    }

    #[test]
    fn every_tune_sounds_and_does_not_clip() {
        let Some(data) = pack() else { return };
        for &id in data.music.tunes.keys() {
            let out = render(&data, id, if std::env::var("HARRY_MUSIC_DUMP").is_ok() { 40.0 } else { 6.0 });
            let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            let rms = (out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32).sqrt();
            assert!(rms > 0.005, "Tune {id} is silent ({rms})");
            assert!(peak < 1.0, "Tune {id} clips ({peak})");
        }
    }

    #[test]
    fn a_fade_out_reaches_silence_in_its_time() {
        let Some(data) = pack() else { return };
        let mut p = Player::new(Library::new(&data.music), 22254);
        p.play(601);
        // FadeMusic(0, 1): 24 ticks = 0.4 s
        p.fade(true, 1);
        let mut out = vec![0.0f32; 22254 / 5 * 2];
        p.mix(&mut out);
        assert!(p.fade > 0.0);
        let mut out = vec![0.0f32; 22254 / 4 * 2];
        p.mix(&mut out);
        assert_eq!(p.fade, 0.0);
        assert!(out[out.len() - 2000..].iter().all(|&x| x == 0.0));
    }
}
