//! The colour tables of the tints against the original, recorded in
//! Unicorn by `re/tools/emu_tint.py`: every colour `Color2Index` is asked
//! for, in order, and the tables built from the answers.

mod common;

use common::recording;
use harry_core::tint::Tables;
use harry_data::clut8;

#[test]
fn the_tint_tables_match_the_original() {
    let Some(text) = recording("tint", "emu_tint.py") else { return };
    let mut asked = Vec::new();
    let mut calls = 0u32;
    let t = Tables::build(
        &clut8(),
        &mut |rgb| {
            asked.push(format!("q {} {} {}", rgb[0], rgb[1], rgb[2]));
            (asked.len() - 1) as u8
        },
        &mut || {
            calls += 1;
            ((calls - 1) * 7919 % 5000) as u16
        },
    );
    let want: Vec<&str> = text.lines().filter(|l| l.starts_with("q ")).collect();
    assert_eq!(asked.len(), want.len(), "number of colours");
    let bad: Vec<usize> = (0..want.len()).filter(|&i| asked[i] != want[i]).collect();
    for &i in bad.iter().take(8) {
        eprintln!("colour {i}: want {}, got {}", want[i], asked[i]);
    }
    assert!(bad.is_empty(), "{} of {} colours differ", bad.len(), want.len());
    let row = |e: &[u8; 256]| e.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with("q ")) {
        let mut f = line.split(' ');
        let (name, k, entries) = (f.next().unwrap(), f.next().unwrap().parse::<usize>().unwrap(), f.next().unwrap());
        let got = match name {
            "tint" => row(&t.tint[k]),
            "transluc" => row(&t.transluc[k]),
            "posterize" => row(&t.posterize),
            "redden" => row(&t.redden[k]),
            _ => row(&t.heat[k]),
        };
        assert_eq!(got, entries, "{name} {k}");
        n += 1;
    }
    eprintln!("{} colours, {n} tables", want.len());
}
