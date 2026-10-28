//! `snd ` resources (Inside Macintosh: Sound): format 1 (with modifiers)
//! or 2, a `bufferCmd` / `soundCmd` with the offset of a standard sound
//! header - all of Harry's sounds are 8-bit unsigned mono.

/// A decoded sound.
pub struct Snd {
    /// Samples per second (the header's Fixed rate, rounded).
    pub rate: u32,
    /// 8-bit unsigned samples, 0x80 silence.
    pub samples: Vec<u8>,
}

fn be16(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(o..o + 2)?.try_into().ok()?))
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

/// Decode a `snd ` resource; `None` if it is not a standard sampled sound.
pub fn decode(d: &[u8]) -> Option<Snd> {
    let commands_at = match be16(d, 0)? {
        1 => 4 + 6 * be16(d, 2)? as usize,
        2 => 4,
        _ => return None,
    };
    let n = be16(d, commands_at)? as usize;
    let header = (0..n).find_map(|i| {
        let at = commands_at + 2 + 8 * i;
        // bufferCmd (0x51) or soundCmd (0x50) with the offset flag
        matches!(be16(d, at)?, 0x8050 | 0x8051).then(|| be32(d, at + 4)).flatten()
    })? as usize;
    if *d.get(header + 20)? != 0 {
        return None;
    }
    let length = be32(d, header + 4)? as usize;
    let rate = (be32(d, header + 8)? + 0x8000) >> 16;
    Some(Snd { rate, samples: d.get(header + 22..header + 22 + length)?.to_vec() })
}
