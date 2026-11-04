//! QuickTime Music Architecture data: tunes (`Tune`: a `musi` header with
//! one note request per part, then 32/64-bit events) and sampled
//! instruments (`ssai`: a QuickTime atom container). See NOTES.md,
//! "Music `Tune`".

use std::collections::BTreeMap;

use harry_data::Sound;
use harry_data::music::{Instrument, MusicEvent, Zone};

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn be16(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

/// A ToneDescription: synthesizer type, synthesizer name, instrument
/// name, instrument number, GM number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToneDescription {
    pub synthesizer: String,
    pub instrument: String,
    pub number: i32,
    pub gm: i32,
}

/// A Str31. Two parts of Tune 614 have a name with a length byte of 0
/// but the text behind it: then the text up to its first zero.
fn pstring(d: &[u8]) -> String {
    let n = match usize::from(*d.first().unwrap_or(&0)) {
        0 => d.iter().skip(1).position(|&b| b == 0).unwrap_or(0),
        n => n,
    }
    .min(d.len().saturating_sub(1));
    harry_core::text::mac_roman(&d[1..1 + n]).trim_end().to_string()
}

fn tone(d: &[u8]) -> Option<ToneDescription> {
    Some(ToneDescription {
        synthesizer: pstring(d.get(4..36)?),
        instrument: pstring(d.get(36..68)?),
        number: be32(d, 68)? as i32,
        gm: be32(d, 72)? as i32,
    })
}

/// One atom of a container: type, id, child count, and where its data
/// (or its children) lie.
struct Atom {
    kind: [u8; 4],
    id: u32,
    children: u16,
    start: usize,
    end: usize,
}

/// The atoms from `o` to `end` (20-byte headers: size, type, id,
/// reserved, child count, reserved).
fn atoms(d: &[u8], mut o: usize, end: usize) -> Vec<Atom> {
    let mut out = Vec::new();
    while o + 20 <= end {
        let Some(size) = be32(d, o).map(|s| s as usize) else { break };
        if size < 20 || o + size > end {
            break;
        }
        out.push(Atom {
            kind: d[o + 4..o + 8].try_into().unwrap(),
            id: be32(d, o + 8).unwrap_or(0),
            children: be16(d, o + 14).unwrap_or(0),
            start: o + 20,
            end: o + size,
        });
        o += size;
    }
    out
}

/// The ToneDescription of an atom container (after its 12-byte head):
/// `sean` / `tone`.
fn container_tone(d: &[u8]) -> Option<ToneDescription> {
    let root = atoms(d, 12, d.len()).into_iter().next()?;
    let t = atoms(d, root.start, root.end).into_iter().find(|a| &a.kind == b"tone")?;
    tone(&d[t.start..t.end])
}

/// A tune: the part's tone requests and the events, up to the end marker.
pub fn tune(d: &[u8]) -> Option<(BTreeMap<u16, ToneDescription>, Vec<MusicEvent>)> {
    let header = be32(d, 0)? as usize;
    let mut parts = BTreeMap::new();
    let mut o = 0x14;
    while o + 4 <= header {
        let w = be32(d, o)?;
        if w >> 28 != 0xf {
            break;
        }
        let (part, words) = (((w >> 16) & 0xfff) as u16, (w & 0xffff) as usize);
        let body = d.get(o + 4..o + 4 * words - 4)?;
        let kind = (be32(d, o + 4 * words - 4)? >> 16) & 0x3fff;
        let t = match kind {
            // note request: flags, polyphony, typical polyphony, tone
            1 => tone(body.get(8..)?),
            // atomic instrument
            6 => container_tone(body),
            _ => None,
        };
        if let Some(t) = t {
            parts.insert(part, t);
        }
        o += 4 * words;
    }
    let mut events = Vec::new();
    let mut o = header;
    while let Some(w) = be32(d, o) {
        if w == 0x6000_0000 {
            break;
        }
        let signed = |v: u32| v as u16 as i16;
        match w >> 29 {
            0 => {
                events.push(MusicEvent::Rest(w & 0xff_ffff));
                o += 4;
            }
            1 => {
                events.push(MusicEvent::Note {
                    part: ((w >> 24) & 0x1f) as u16,
                    pitch: ((((w >> 18) & 0x3f) + 32) << 8) as u16,
                    velocity: ((w >> 11) & 0x7f) as u8,
                    duration: w & 0x7ff,
                });
                o += 4;
            }
            2 => {
                events.push(MusicEvent::Controller {
                    part: ((w >> 24) & 0x1f) as u16,
                    controller: ((w >> 16) & 0xff) as u16,
                    value: signed(w),
                });
                o += 4;
            }
            3 => o += 4,
            _ => match w >> 28 {
                // extended note: a note number, or 8.8 above 127
                9 => {
                    let w2 = be32(d, o + 4)?;
                    let p = w & 0xffff;
                    events.push(MusicEvent::Note {
                        part: ((w >> 16) & 0xfff) as u16,
                        pitch: if p > 127 { p as u16 } else { (p << 8) as u16 },
                        velocity: ((w2 >> 22) & 0x7f) as u8,
                        duration: w2 & 0x3f_ffff,
                    });
                    o += 8;
                }
                0xa => {
                    let w2 = be32(d, o + 4)?;
                    events.push(MusicEvent::Controller {
                        part: ((w >> 16) & 0xfff) as u16,
                        controller: (w & 0xffff) as u16,
                        value: signed(w2),
                    });
                    o += 8;
                }
                0xb => o += 8,
                0xf => o += 4 * (w & 0xffff).max(1) as usize,
                _ => o += 4,
            },
        }
    }
    Some((parts, events))
}

/// A sampled instrument: its tone, knobs, zones and the sample data by
/// id (`sdat` atoms, directly or inside `smin`).
pub struct Ssai {
    pub tone: ToneDescription,
    pub instrument: Instrument,
    /// Zone sample ids (in `instrument.zones` order).
    pub zone_samples: Vec<u32>,
    pub samples: BTreeMap<u32, Sound>,
}

fn knobs(d: &[u8]) -> Vec<(u32, i32)> {
    let n = be32(d, 4).unwrap_or(0) as usize;
    (0..n.min(64)).filter_map(|k| Some((be32(d, 8 + 8 * k)?, be32(d, 12 + 8 * k)? as i32))).collect()
}

pub fn ssai(d: &[u8]) -> Option<Ssai> {
    let root = atoms(d, 12, d.len()).into_iter().next()?;
    let mut tone_d = None;
    let mut inst = Instrument::default();
    let mut zone_samples = Vec::new();
    let mut descs = Vec::new();
    let mut data: BTreeMap<u32, &[u8]> = BTreeMap::new();
    for a in atoms(d, root.start, root.end) {
        let body = &d[a.start..a.end];
        match &a.kind {
            b"tone" => tone_d = tone(body),
            b"knbl" => inst.knobs = knobs(body),
            b"sinf" => {
                let kids = atoms(d, a.start, a.end);
                let sd = kids.iter().find(|k| &k.kind == b"sdsc")?;
                let zk = kids.iter().find(|k| &k.kind == b"knbl").map(|k| knobs(&d[k.start..k.end])).unwrap_or_default();
                descs.push((sd.start, zk));
            }
            b"sdat" => {
                data.insert(a.id, body);
            }
            b"smin" if a.children > 0 => {
                for k in atoms(d, a.start, a.end) {
                    if &k.kind == b"sdat" {
                        data.insert(a.id, &d[k.start..k.end]);
                    }
                }
            }
            _ => {}
        }
    }
    let tone = tone_d?;
    inst.name = tone.instrument.clone();
    let mut samples = BTreeMap::new();
    for (s, zk) in descs {
        let fmt = &d[s..s + 4];
        let (bits, rate, id) = (be16(d, s + 6)?, be32(d, s + 8)?, u32::from(be16(d, s + 12)?));
        let (offset, count) = (be32(d, s + 14)? as usize, be32(d, s + 18)? as usize);
        inst.zones.push(Zone {
            loop_type: be32(d, s + 22)?,
            loop_start: be32(d, s + 26)?,
            loop_end: be32(d, s + 30)?,
            normal: be32(d, s + 34)? as u8,
            low: be32(d, s + 38)? as u8,
            high: be32(d, s + 42)? as u8,
            rate,
            knobs: zk,
            sample: String::new(),
        });
        zone_samples.push(id);
        // 8-bit, as unsigned ('raw ' is offset binary, 'twos' signed);
        // the samples of another instrument are taken from there
        if bits == 8
            && let Some(raw) = data.get(&id)
        {
            let raw = raw.get(offset..(offset + count).min(raw.len()))?;
            let samples_u8 = if fmt == b"twos" { raw.iter().map(|b| b ^ 0x80).collect() } else { raw.to_vec() };
            samples.insert(id, Sound { rate: rate >> 16, samples: samples_u8 });
        }
    }
    Some(Ssai { tone, instrument: inst, zone_samples, samples })
}
