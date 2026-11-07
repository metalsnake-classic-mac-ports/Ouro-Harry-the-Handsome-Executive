//! The memo and phone sentences against the original, recorded in Unicorn
//! by `re/tools/emu_words.py`.

mod common;

use common::{Tally, installed, recording};
use harry_core::random::Random;
use harry_original::words::words;

#[test]
fn sentences_match_the_original() {
    let Some(text) = recording("words", "emu_words.py") else { return };
    let Some(app) = installed("Harry 1.0.1.bin") else { return };
    let base = words(&app).unwrap();
    let mut tally = Tally::new("words", 8);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let f: Vec<&str> = a.split(' ').map(|kv| kv.split_once('=').unwrap().1).collect();
        let mut rng = Random::new(f[0].parse().unwrap());
        let mut w = base.clone();
        for (k, v) in f[1].split(':').enumerate() {
            w.last[k] = v.parse().unwrap();
        }
        let texts: Vec<String> = f[2]
            .chars()
            .map(|c| {
                let t = if c == 'b' { w.buzzwords(&mut rng) } else { w.phonewords(&mut rng) };
                if t.is_empty() { "-".into() } else { t.iter().map(|x| format!("{x:02x}")).collect() }
            })
            .collect();
        let last: Vec<String> = w.last.iter().map(|x| x.to_string()).collect();
        let got = format!("{} seed={} last={}", texts.join(","), rng.seed, last.join(":"));
        tally.case(got == b, || format!("{a}\n  want {b}\n  got  {got}"));
    }
    tally.finish();
}
