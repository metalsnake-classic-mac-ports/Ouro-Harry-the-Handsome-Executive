//! The music: the tunes (QuickTime Music Architecture sequences from
//! `Harry Music`) and the sampled instruments they play (`ssai` of `Harry
//! Instruments` and of QuickTime Musical Instruments). The game
//! synthesizes a tune from these; a finished recording of a tune, when the
//! pack has one, is played instead.

use std::collections::BTreeMap;

use crate::Sound;

/// One event of a tune; times in units of 1/600 s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicEvent {
    /// Time goes on (the only event that advances it).
    Rest(u32),
    /// A note: its pitch in semitones × 256 (MIDI numbering, 60 = middle
    /// C), velocity 0–127, how long it sounds.
    Note { part: u16, pitch: u16, velocity: u8, duration: u32 },
    /// A controller: 7 volume, 10 pan, 32 pitch bend, 33 (aftertouch?), …;
    /// the value as the tune has it (mostly 8.8 fixed point).
    Controller { part: u16, controller: u16, value: i16 },
}

/// A tune: which instrument each part plays (`instruments` key), the
/// events, its length (the sum of the rests).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tune {
    pub parts: BTreeMap<u16, String>,
    pub events: Vec<MusicEvent>,
}

impl Tune {
    /// The length in units of 1/600 s.
    pub fn length(&self) -> u64 {
        self.events.iter().map(|e| if let MusicEvent::Rest(n) = e { u64::from(*n) } else { 0 }).sum()
    }
}

/// A knob setting of an instrument or zone (`knbl`): number, value.
pub type Knob = (u32, i32);

/// One key range of an instrument (`sinf` / `sdsc`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    pub low: u8,
    pub high: u8,
    /// The key the sample sounds at its own rate.
    pub normal: u8,
    /// Sample rate, 16.16 fixed point.
    pub rate: u32,
    /// `sdsc` loop type (0: loop from start to end while the note holds).
    pub loop_type: u32,
    pub loop_start: u32,
    pub loop_end: u32,
    pub knobs: Vec<Knob>,
    /// The sample's key in `Music::samples`.
    pub sample: String,
}

/// A sampled instrument (`ssai`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Instrument {
    pub name: String,
    pub knobs: Vec<Knob>,
    pub zones: Vec<Zone>,
}

/// A finished recording of a tune: 16-bit samples, interleaved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recording {
    pub rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

/// Everything about the music.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Music {
    /// By `Tune` id.
    pub tunes: BTreeMap<i16, Tune>,
    /// By key: `harry-<ssai id>` for `Harry Instruments`, `qt-<number>` for
    /// QuickTime Musical Instruments.
    pub instruments: BTreeMap<String, Instrument>,
    /// 8-bit unsigned mono samples by key (rate: the zone's).
    pub samples: BTreeMap<String, Sound>,
    /// Recordings that replace the synthesized tune, by `Tune` id.
    pub recordings: BTreeMap<i16, Recording>,
}

/// A tune as text: `part <part> <instrument>` lines, then one event a
/// line: `r <units>`, `n <part> <pitch × 256> <velocity> <duration>`, `c
/// <part> <controller> <value>`.
pub fn tune_text(t: &Tune) -> String {
    let mut s = String::new();
    for (p, i) in &t.parts {
        s.push_str(&format!("part {p} {i}\n"));
    }
    for e in &t.events {
        match *e {
            MusicEvent::Rest(n) => s.push_str(&format!("r {n}\n")),
            MusicEvent::Note { part, pitch, velocity, duration } => s.push_str(&format!("n {part} {pitch} {velocity} {duration}\n")),
            MusicEvent::Controller { part, controller, value } => s.push_str(&format!("c {part} {controller} {value}\n")),
        }
    }
    s
}

pub fn parse_tune(text: &str) -> Option<Tune> {
    let mut t = Tune::default();
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        let n = |i: usize| w.get(i)?.parse::<i64>().ok();
        match w.first().copied() {
            Some("part") => {
                t.parts.insert(n(1)? as u16, w.get(2)?.to_string());
            }
            Some("r") => t.events.push(MusicEvent::Rest(n(1)? as u32)),
            Some("n") => t.events.push(MusicEvent::Note {
                part: n(1)? as u16,
                pitch: n(2)? as u16,
                velocity: n(3)? as u8,
                duration: n(4)? as u32,
            }),
            Some("c") => t.events.push(MusicEvent::Controller { part: n(1)? as u16, controller: n(2)? as u16, value: n(3)? as i16 }),
            None => {}
            _ => return None,
        }
    }
    Some(t)
}

/// An instrument as text: `name <name>`, `knob <number> <value>` (the
/// instrument's), then per zone `zone <low> <high> <normal> <rate>
/// <loop type> <loop start> <loop end> <sample>` with its `zknob` lines.
pub fn instrument_text(i: &Instrument) -> String {
    let mut s = format!("name {}\n", i.name);
    for (k, v) in &i.knobs {
        s.push_str(&format!("knob {k:#x} {v}\n"));
    }
    for z in &i.zones {
        s.push_str(&format!(
            "zone {} {} {} {} {} {} {} {}\n",
            z.low, z.high, z.normal, z.rate, z.loop_type, z.loop_start, z.loop_end, z.sample
        ));
        for (k, v) in &z.knobs {
            s.push_str(&format!("zknob {k:#x} {v}\n"));
        }
    }
    s
}

pub fn parse_instrument(text: &str) -> Option<Instrument> {
    let mut i = Instrument::default();
    let num = |s: &str| -> Option<i64> {
        match s.strip_prefix("0x") {
            Some(h) => i64::from_str_radix(h, 16).ok(),
            None => s.parse().ok(),
        }
    };
    for line in text.lines() {
        let (k, rest) = line.split_once(' ').unwrap_or((line, ""));
        let w: Vec<&str> = rest.split_whitespace().collect();
        match k {
            "name" => i.name = rest.to_string(),
            "knob" | "zknob" => {
                let knob = (num(w.first()?)? as u32, num(w.get(1)?)? as i32);
                if k == "knob" { i.knobs.push(knob) } else { i.zones.last_mut()?.knobs.push(knob) }
            }
            "zone" => i.zones.push(Zone {
                low: num(w.first()?)? as u8,
                high: num(w.get(1)?)? as u8,
                normal: num(w.get(2)?)? as u8,
                rate: num(w.get(3)?)? as u32,
                loop_type: num(w.get(4)?)? as u32,
                loop_start: num(w.get(5)?)? as u32,
                loop_end: num(w.get(6)?)? as u32,
                knobs: Vec::new(),
                sample: w.get(7)?.to_string(),
            }),
            "" => {}
            _ => return None,
        }
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tunes_and_instruments_round_trip_as_text() {
        let mut t = Tune::default();
        t.parts.insert(0, "qt-74".into());
        t.events = vec![
            MusicEvent::Controller { part: 0, controller: 7, value: 0x7f00 },
            MusicEvent::Note { part: 0, pitch: 60 * 256 + 128, velocity: 100, duration: 300 },
            MusicEvent::Rest(150),
            MusicEvent::Controller { part: 0, controller: 32, value: -512 },
        ];
        assert_eq!(parse_tune(&tune_text(&t)), Some(t.clone()));
        assert_eq!(t.length(), 150);
        let i = Instrument {
            name: "Tech Drum Kit 1".into(),
            knobs: vec![(0x0200_000c, 17)],
            zones: vec![Zone {
                low: 50,
                high: 53,
                normal: 60,
                rate: 22050 << 16,
                loop_type: 0,
                loop_start: 10494,
                loop_end: 10495,
                knobs: vec![(0x0200_0012, -6151)],
                sample: "harry-514-1".into(),
            }],
        };
        assert_eq!(parse_instrument(&instrument_text(&i)), Some(i));
    }
}
