//! The PowerPC fragment (PEF, in the application's data fork): its data
//! section, unpacked from PEF's pattern-initialised format. Some texts of
//! the game live only there (the balloons of the key cheats).

use crate::bytes::{try_be16, try_be32};

/// The unpacked data section (the first section of kind 2, "pattern
/// initialised data", or 1, "unpacked data"), without the relocations.
pub fn data_section(pef: &[u8]) -> Option<Vec<u8>> {
    if pef.get(0..8)? != b"Joy!peff" {
        return None;
    }
    let be16 = |o: usize| try_be16(pef, o);
    let be32 = |o: usize| try_be32(pef, o).map(|x| x as usize);
    let sections = usize::from(be16(0x20)?);
    for i in 0..sections {
        let h = 40 + 28 * i;
        let (unpacked, packed, offset, kind) = (be32(h + 12)?, be32(h + 16)?, be32(h + 20)?, *pef.get(h + 24)?);
        let raw = pef.get(offset..offset + packed)?;
        match kind {
            1 => return Some(raw.to_vec()),
            2 => return unpack(raw, unpacked),
            _ => {}
        }
    }
    None
}

/// PEF pattern initialisation: an opcode byte (3 bits operation, 5 bits
/// count; count 0 = a variable-length count follows), arguments as
/// big-endian 7-bit groups with the high bit set on all but the last.
fn unpack(p: &[u8], size: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(size);
    let mut i = 0;
    let arg = |i: &mut usize| -> Option<usize> {
        let mut v = 0usize;
        loop {
            let b = *p.get(*i)?;
            *i += 1;
            v = (v << 7) | usize::from(b & 0x7f);
            if b & 0x80 == 0 {
                return Some(v);
            }
        }
    };
    let take = |i: &mut usize, n: usize| -> Option<&[u8]> {
        let s = p.get(*i..*i + n)?;
        *i += n;
        Some(s)
    };
    while i < p.len() {
        let op = p[i];
        i += 1;
        let count = match usize::from(op & 0x1f) {
            0 => arg(&mut i)?,
            n => n,
        };
        match op >> 5 {
            // zero
            0 => out.resize(out.len() + count, 0),
            // block copy
            1 => out.extend_from_slice(take(&mut i, count)?),
            // repeated block: count bytes, repeat + 1 times
            2 => {
                let repeat = arg(&mut i)?;
                let block = take(&mut i, count)?.to_vec();
                for _ in 0..=repeat {
                    out.extend_from_slice(&block);
                }
            }
            // interleaved: common, then repeat × (custom, common)
            3 => {
                let (custom, repeat) = (arg(&mut i)?, arg(&mut i)?);
                let common = take(&mut i, count)?.to_vec();
                out.extend_from_slice(&common);
                for _ in 0..repeat {
                    out.extend_from_slice(take(&mut i, custom)?);
                    out.extend_from_slice(&common);
                }
            }
            // interleaved with zeros: zeros, then repeat × (custom, zeros)
            4 => {
                let (custom, repeat) = (arg(&mut i)?, arg(&mut i)?);
                out.resize(out.len() + count, 0);
                for _ in 0..repeat {
                    out.extend_from_slice(take(&mut i, custom)?);
                    out.resize(out.len() + count, 0);
                }
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Pascal strings of 256 bytes each at `offset` in the data section.
pub fn strings(data: &[u8], offset: usize, n: usize) -> Vec<Vec<u8>> {
    (0..n)
        .filter_map(|k| {
            let o = offset + 256 * k;
            let len = usize::from(*data.get(o)?);
            Some(data.get(o + 1..o + 1 + len)?.to_vec())
        })
        .collect()
}
