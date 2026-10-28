//! The random sentences of memos and the phone (`GenerateBuzzwords`,
//! `GeneratePhonewords`, `PickAString`): a template from one string list
//! with its placeholders `^0` … `^4` replaced by strings from the others.
//! Texts are Mac Roman bytes.

use crate::random::Random;

/// The string lists (`STR#` 6900–6905: buzzword templates and their five
/// filler lists; 6800–6801: phone templates and names) and the index each
/// list gave last (globals 0x10066e6c … 0x10066e7a, kept for the whole
/// run; a list never gives the same string twice in a row).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    pub buzz: [Vec<Vec<u8>>; 6],
    pub phone: [Vec<Vec<u8>>; 2],
    /// Buzzword template and fillers 1–5, phone template and filler.
    pub last: [u16; 8],
}

/// Parse an `STR#` resource.
pub fn parse_str_list(d: &[u8]) -> Vec<Vec<u8>> {
    let n = d.get(..2).map_or(0, |b| u16::from_be_bytes([b[0], b[1]]) as usize);
    let mut out = Vec::with_capacity(n);
    let mut p = 2;
    for _ in 0..n {
        let Some(&len) = d.get(p) else { break };
        out.push(d[p + 1..(p + 1 + len as usize).min(d.len())].to_vec());
        p += 1 + len as usize;
    }
    out
}

/// `PickAString`: a string other than the last one (QuickDraw `Random`,
/// as unsigned, modulo the count).
fn pick(list: &[Vec<u8>], last: &mut u16, rng: &mut Random) -> Vec<u8> {
    let n = list.len() as u32;
    if n == 0 {
        return Vec::new();
    }
    let i = loop {
        let r = u32::from(rng.next() as u16) % n;
        if r as u16 != *last {
            break r;
        }
    };
    *last = i as u16;
    list[i as usize].clone()
}

/// Fill a template: for each placeholder, a string is picked and put in
/// place of its next occurrence (`Munger`) until there is none - one pick
/// more than there are occurrences. A lowercase first letter (or the
/// second after a quotation mark) becomes uppercase. Like the original the
/// result keeps only its length modulo 256 (a Pascal string).
fn generate(lists: &[Vec<Vec<u8>>], last: &mut [u16], rng: &mut Random) -> Vec<u8> {
    let mut text = pick(&lists[0], &mut last[0], rng);
    for k in 1..lists.len() {
        let key = [b'^', b'0' + (k - 1) as u8];
        let mut from = 0;
        loop {
            let s = pick(&lists[k], &mut last[k], rng);
            let Some(pos) = text.get(from..).and_then(|t| t.windows(2).position(|w| w == key)).map(|p| p + from) else {
                break;
            };
            text.splice(pos..pos + 2, s.iter().copied());
            from = pos + s.len();
        }
    }
    let upper = |c: &mut u8| {
        if c.is_ascii_lowercase() {
            *c -= 0x20;
        }
    };
    if let Some(c) = text.first_mut() {
        upper(c);
    }
    if matches!(text.first(), Some(b'"' | 0xd2))
        && let Some(c) = text.get_mut(1)
    {
        upper(c);
    }
    text.truncate(text.len() & 0xff);
    text
}

impl Words {
    /// `GenerateBuzzwords`.
    pub fn buzzwords(&mut self, rng: &mut Random) -> Vec<u8> {
        generate(&self.buzz, &mut self.last[..6], rng)
    }

    /// `GeneratePhonewords`.
    pub fn phonewords(&mut self, rng: &mut Random) -> Vec<u8> {
        generate(&self.phone, &mut self.last[6..], rng)
    }
}
