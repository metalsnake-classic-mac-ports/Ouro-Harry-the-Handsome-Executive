//! The ripple's sine table against the original's `BuildSineTable`,
//! recorded in Unicorn by `re/tools/emu_sine.py`.

mod common;

use common::{Tally, installed, recording};
use harry_core::Level;
use harry_core::ripple::{LENGTH, WIDTH, sine_table};
use harry_original::hlvl;

#[test]
fn the_sine_table_matches_the_original() {
    let Some(text) = recording("sine", "emu_sine.py") else { return };
    let Some(ep) = installed("Episode 1.bin") else { return };
    let level: Level = hlvl::read(ep.get(b"Hlvl", 10).unwrap()).unwrap();
    let mut tally = Tally::new("sine", 8);
    for line in text.lines() {
        let (a, b) = line.split_once(" -> ").unwrap();
        let (len, width) = a.split_once(' ').unwrap();
        let mut l = level.clone();
        l.info[LENGTH] = len.parse().unwrap();
        l.info[WIDTH] = width.parse().unwrap();
        let t = sine_table(&mut l);
        let entries: Vec<String> = t.iter().map(i16::to_string).collect();
        let got = format!("{} {}", l.info[LENGTH], if entries.is_empty() { "-".to_string() } else { entries.join(",") });
        tally.case(got == b, || format!("{a}\n  want {b}\n   got {got}"));
    }
    tally.finish();
}
