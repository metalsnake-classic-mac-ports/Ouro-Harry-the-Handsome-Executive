//! The plant's hit task against the original, recorded in Unicorn by
//! `re/tools/emu_planthit.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::Physics;
use harry_core::kinds::{PlantHitMasks, hit_plant, setup_cooler, setup_door, setup_plant};
use harry_core::player::{HOT_RECT, KICK_HOT_RECT};
use harry_core::random::Random;
use harry_core::sprite::{Sprite, Task};
use harry_original::masks;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn plant_hits_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/planthit.txt")) else {
        eprintln!("skipped: planthit.txt not found (run re/tools/emu_planthit.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Harry Graphics.bin")) else { return };
    let gfx = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let body = masks::from_cicn(&gfx, 299).unwrap();
    let plant = masks::from_cicn(&gfx, 599).unwrap();
    let barrel = masks::from_cicn(&gfx, 598).unwrap();
    let ms = PlantHitMasks { body: &body, plant: &plant, barrel: &barrel };
    let physics = Physics::default();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let mut sp = Sprite::new(601, w("h") as i16, w("v") as i16);
        setup_plant(&mut sp);
        let kind = w("other");
        let okind = match kind {
            2 => 601,
            3 => 500,
            _ => w("okind") as i16,
        };
        let mut o = Sprite::new(okind, w("oh") as i16, w("ov") as i16);
        match kind {
            2 => setup_plant(&mut o),
            3 => setup_cooler(&mut o),
            4 => setup_door(&mut o),
            _ => {
                o.task = Some(match kind {
                    1 => Task::Player,
                    5 => Task::PlayerShot,
                    _ => Task::EnemyShot,
                });
                o.at = (w("ov") as i16, w("oh") as i16);
                if kind == 1 {
                    o.hot_rect = if w("kicking") != 0 { KICK_HOT_RECT } else { HOT_RECT };
                }
            }
        }
        for (s, pre) in [(&mut sp, ""), (&mut o, "o")] {
            let k = |n: &str| i[format!("{pre}{n}").as_str()].parse::<i64>().unwrap();
            s.pos = (k("pv") as i16, k("ph") as i16);
            s.fix = (k("fixh") as i32, k("fixv") as i32);
            s.speed.h = k("speedh") as i16;
            s.speed.v = k("speedv") as i16;
            s.air = k("air") as i16;
            s.frozen = k("frozen") != 0;
        }
        o.dir = w("odir") as i16;
        let mut contact = w("contact") as i16;
        let mut rng = Random::new(w("seed") as u32);
        let mut events = Vec::new();
        hit_plant(&mut sp, &mut o, &ms, &physics, &mut rng, &mut contact, w("busy") != 0, &mut events);
        let got = format!(
            "{} || {} | contact={contact} events={}",
            dump(&sp),
            dump(&o),
            common::events(&events)
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
