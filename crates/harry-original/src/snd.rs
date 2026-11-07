//! `snd ` resources (Inside Macintosh: Sound): format 1 (with modifiers)
//! or 2, a `bufferCmd` / `soundCmd` with the offset of a standard sound
//! header - all of Harry's sounds are 8-bit unsigned mono.

use crate::bytes::{try_be16, try_be32};

/// A decoded sound.
pub struct Snd {
    /// Samples per second (the header's Fixed rate, rounded).
    pub rate: u32,
    /// 8-bit unsigned samples, 0x80 silence.
    pub samples: Vec<u8>,
}

/// Decode a `snd ` resource; `None` if it is not a standard sampled sound.
pub fn decode(d: &[u8]) -> Option<Snd> {
    let commands_at = match try_be16(d, 0)? {
        1 => 4 + 6 * try_be16(d, 2)? as usize,
        2 => 4,
        _ => return None,
    };
    let n = try_be16(d, commands_at)? as usize;
    let header = (0..n).find_map(|i| {
        let at = commands_at + 2 + 8 * i;
        // bufferCmd (0x51) or soundCmd (0x50) with the offset flag
        matches!(try_be16(d, at)?, 0x8050 | 0x8051).then(|| try_be32(d, at + 4)).flatten()
    })? as usize;
    if *d.get(header + 20)? != 0 {
        return None;
    }
    let length = try_be32(d, header + 4)? as usize;
    let rate = (try_be32(d, header + 8)? + 0x8000) >> 16;
    Some(Snd { rate, samples: d.get(header + 22..header + 22 + length)?.to_vec() })
}
