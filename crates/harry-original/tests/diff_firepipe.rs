//! Fire pipes and their flames against the original, recorded in Unicorn
//! by `re/tools/emu_firepipe.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::effect::{handle_effect, setup_effect};
use harry_core::firepipe::{handle_fire_pipe, setup_fire_pipe};
use harry_core::random::Random;
use harry_core::sprite::Sprite;

fn check(name: &str, run: impl Fn(&HashMap<&str, &str>) -> String) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../re/work/diff/{name}.txt"));
    let Ok(text) = std::fs::read_to_string(path) else {
        eprintln!("skipped: {name}.txt not found (run re/tools/emu_firepipe.py)");
        return;
    };
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let got = run(&i);
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

#[test]
fn fire_pipes_match_the_original() {
    check("firepipe", |i| {
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let mut rng = Random::new(w("seed") as u32);
        let mut sp = Sprite::new(w("kind") as i16, w("h") as i16, w("v") as i16);
        setup_fire_pipe(&mut sp, &mut rng);
        sp.level_index = w("index") as i16;
        if w("f6e") != -1 {
            sp.f6e = w("f6e") as i32;
        }
        sp.frozen = w("frozen") != 0;
        let info1 = (w("index") >= 0).then_some(w("info1") as i16);
        let mut evs = Vec::new();
        for _ in 0..w("frames") {
            let mut events = Vec::new();
            handle_fire_pipe(&mut sp, info1, &mut rng, &mut events);
            evs.push(common::events(&events));
        }
        format!("{} | events={}", dump(&sp), evs.join(";"))
    });
}

#[test]
fn flames_match_the_original() {
    check("flame", |i| {
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let pipe = w("pipe") as i16;
        let mut sp = Sprite::new(pipe, w("h") as i16, w("v") as i16);
        setup_effect(&mut sp);
        sp.kind = w("kind") as i16;
        sp.f7c = pipe;
        for f in 0..w("frames") {
            if f == w("touch") {
                sp.f6e = 3;
            }
            handle_effect(&mut sp);
        }
        dump(&sp)
    });
}
