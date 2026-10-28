//! Co-workers against the original, recorded in Unicorn by
//! `re/tools/emu_coworker.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::coworker::{handle_coworker, setup_coworker};
use harry_core::random::Random;
use harry_core::sprite::Sprite;

#[test]
fn coworkers_match_the_original() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/work/diff/coworker.txt");
    let Ok(text) = std::fs::read_to_string(path) else {
        eprintln!("skipped: coworker.txt not found (run re/tools/emu_coworker.py)");
        return;
    };
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let mut sp = Sprite::new(w("kind") as i16, w("h") as i16, w("v") as i16);
        setup_coworker(&mut sp);
        sp.dir = w("dir") as i16;
        sp.layer = w("layer") as i16;
        sp.frozen = w("frozen") != 0;
        let mut rng = Random::new(w("seed") as u32);
        for _ in 0..w("frames") {
            handle_coworker(&mut sp, &mut rng);
        }
        let got = dump(&sp);
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 8 {
                let diffs: Vec<String> =
                    b.split(' ').zip(got.split(' ')).filter(|(x, y)| x != y).map(|(x, y)| format!("want {x} got {y}")).collect();
                eprintln!("{a}\n  {}", diffs.join(", "));
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
