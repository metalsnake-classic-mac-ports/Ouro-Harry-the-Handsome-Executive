//! The message sprite against the original, recorded in Unicorn by
//! `re/tools/emu_message.py` (ShowBalloon stubbed with recorded answers).

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use harry_core::random::Random;

use common::dump;
use harry_core::kinds::{BalloonText, handle_message, hit_message, setup_message};
use harry_core::sprite::{Sprite, Task};
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn message_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/message.txt")) else {
        eprintln!("skipped: message.txt not found (run re/tools/emu_message.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Episode 1.bin")) else { return };
    let res = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let mut level = hlvl::read(res.get(b"Hlvl", 1).unwrap()).unwrap();
    let Ok(app) = std::fs::read(root().join("re/work/installed/Harry 1.0.1.bin")) else { return };
    let app = Resources::parse(&MacBinary::parse(&app).unwrap().rsrc).unwrap();
    let base = harry_original::words::words(&app).unwrap();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap() as i16;
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_message(&mut sp);
        let li = w("levelindex");
        sp.level_index = li;
        if li >= 0 {
            level.sprites[li as usize].info[0] = w("info0");
            level.sprites[li as usize].info[1] = w("info1");
        }
        let mut other = Sprite::new(0, 0, 0);
        other.task = Some(if w("other") == 2 { Task::Bonus } else { Task::Player });
        other.frozen = w("other") == 3;
        other.hot_rect = [22, if w("kicking") != 0 { 0 } else { 22 }, 42, 42];
        let mut answers: Vec<bool> = i["answers"].chars().map(|c| c == '1').collect();
        answers.reverse();
        let mut calls = Vec::new();
        let mut words = base.clone();
        let mut rng = Random::new(i["seed"].parse().unwrap());
        let mut show = |_: &Sprite, t: BalloonText, mode: i16| {
            let ans = answers.pop().unwrap_or(false);
            let name = match t {
                BalloonText::World(n) => n.to_string(),
                BalloonText::Buzzwords(t) => format!("buzz:{}", t.iter().map(|x| format!("{x:02x}")).collect::<String>()),
                BalloonText::Phonewords(_) => "phone".into(),
            };
            calls.push(format!("b:{name}:{mode}:{}", u8::from(ans)));
            ans
        };
        let frames = w("frames");
        for f in 0..=frames {
            if f == w("hitframe") {
                if f == 0 {
                    sp.frozen = w("frozen") != 0;
                }
                hit_message(&mut sp, &other, &level, w("pref8") > 2, &mut words, &mut rng, &mut show);
            }
            if f < frames {
                handle_message(&mut sp, &level, &mut words, &mut rng, &mut show);
            }
        }
        let got = format!(
            "{} | pending={} calls={} seed={}",
            dump(&sp),
            u8::from(sp.balloon_pending),
            if calls.is_empty() { "-".into() } else { calls.join(",") },
            rng.seed
        );
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
