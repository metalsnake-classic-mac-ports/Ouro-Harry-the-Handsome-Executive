//! The plant against the original, recorded in Unicorn by
//! `re/tools/emu_plant.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::Physics;
use harry_core::kinds::{ObjectMasks, handle_plant, hit_plant_tile, setup_plant};
use harry_core::random::Random;
use harry_core::sprite::Sprite;
use harry_core::tiles::touched_tiles;
use harry_original::resfork::{MacBinary, Resources};
use harry_original::{hlvl, masks};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn installed(name: &str) -> Option<Resources> {
    let bytes = std::fs::read(root().join("re/work/installed").join(name)).ok()?;
    Some(Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap())
}

#[test]
fn plant_movement_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/plant.txt")) else {
        eprintln!("skipped: plant.txt not found (run re/tools/emu_plant.py)");
        return;
    };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let levels: Vec<_> = (1..=15).map(|i| hlvl::read(ep.get(b"Hlvl", i).unwrap()).unwrap()).collect();
    let plant = masks::from_cicn(&gfx, 599).unwrap();
    let obstacle = masks::from_cicn(&gfx, 290).unwrap();
    let walls = masks::walls(&app).unwrap();
    let ms = ObjectMasks { plant: &plant, walls: &walls, obstacle: &obstacle };
    let physics = Physics::default();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let level = &levels[w("level") as usize - 1];
        let mut sp = Sprite::new(601, w("h") as i16, w("v") as i16);
        setup_plant(&mut sp);
        sp.speed.h = w("speedh") as i16;
        sp.speed.v = w("speedv") as i16;
        sp.dir = w("dir") as i16;
        sp.air = w("air") as i16;
        sp.slide = w("slide") as i16;
        sp.face = 0;
        let mut rng = Random::new(w("seed") as u32);
        let mut flag = true;
        let mut sounds = Vec::new();
        for _ in 0..w("frames") {
            handle_plant(&mut sp, level, &physics, &mut rng, &mut flag, &mut sounds);
            let mut hits = Vec::new();
            touched_tiles(level, sp.at, sp.hot_rect, [0, 0, 36, 36], |h| hits.push(h));
            for h in hits {
                hit_plant_tile(&mut sp, h, level, &ms, &physics);
            }
        }
        let got = format!("{} | flag={} sounds={}", dump(&sp), if flag { 0x5555 } else { 0x55 }, sounds.len());
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
