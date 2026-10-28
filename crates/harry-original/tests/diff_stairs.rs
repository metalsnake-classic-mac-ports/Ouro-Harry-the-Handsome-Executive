//! The stairs against the original, recorded in Unicorn by
//! `re/tools/emu_stairs.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::bounce::Body;
use harry_core::physics::Speed;
use harry_core::sprite::Sprite;
use harry_core::stairs::{setup_stairs, stairs_collision};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn stairs_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/stairs.txt")) else {
        eprintln!("skipped: stairs.txt not found (run re/tools/emu_stairs.py)");
        return;
    };
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let mut stairs = Sprite::new(w("kind") as i16, w("sh") as i16, w("sv") as i16);
        setup_stairs(&mut stairs);
        let hot: Vec<i16> = i["hot"].split(':').map(|x| x.parse().unwrap()).collect();
        let mut body = Body {
            pos: (w("pv") as i16, w("ph") as i16),
            at: (w("v") as i16, w("h") as i16),
            fix: (w("fixh") as i32, w("fixv") as i32),
            speed: Speed { h: w("speedh") as i16, v: w("speedv") as i16 },
        };
        let (mut air, mut flip) = (w("air") as i16, w("flip") as i16);
        let hot = [hot[0], hot[1], hot[2], hot[3]];
        stairs_collision(&mut body, &mut air, &mut flip, w("half") as i16, hot, &stairs, w("bounce") as i16);
        let got = format!(
            "{} | pv={} ph={} v={} h={} speedv={} speedh={} air={air} flip={flip} fixh={} fixv={}",
            dump(&stairs),
            body.pos.0,
            body.pos.1,
            body.at.0,
            body.at.1,
            body.speed.v,
            body.speed.h,
            body.fix.0,
            body.fix.1
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
