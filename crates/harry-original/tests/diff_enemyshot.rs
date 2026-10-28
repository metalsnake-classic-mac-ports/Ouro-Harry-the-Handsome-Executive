//! Enemy shots against the original, recorded in Unicorn by
//! `re/tools/emu_enemyshot.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::collider::setup_collider;
use harry_core::enemy_shot::{ShotMasks, handle_enemy_shot, hit_enemy_shot, hit_enemy_shot_tile, setup_enemy_shot};
use harry_core::kinds::{setup_bonus, setup_cooler, setup_plant};
use harry_core::random::Random;
use harry_core::sprite::{Sprite, Task};
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

type Fields<'a> = HashMap<&'a str, &'a str>;

fn fields(s: &str) -> Fields<'_> {
    s.split(' ').map(|kv| kv.split_once('=').unwrap()).collect()
}

/// The setup with the recorder's overrides (−9999 = keep).
fn shot(i: &Fields, kind: i16, v: i16, h: i16) -> Sprite {
    let w = |k: &str| i[k].parse::<i32>().unwrap();
    let mut sp = Sprite::new(kind, h, v);
    setup_enemy_shot(&mut sp);
    let set = |k: &str| Some(w(k)).filter(|&x| x != -9999);
    if let Some(x) = set("speedv") {
        sp.speed.v = x as i16;
    }
    if let Some(x) = set("speedh") {
        sp.speed.h = x as i16;
    }
    if let Some(x) = set("dir") {
        sp.dir = x as i16;
    }
    if let Some(x) = set("f7c") {
        sp.f7c = x as i16;
    }
    if let Some(x) = set("timer") {
        sp.timer = x as i16;
    }
    if let Some(x) = set("f78") {
        sp.f78 = x;
    }
    if let Some(x) = set("f6e") {
        sp.f6e = x;
    }
    sp
}

fn compare(text: &str, mut run: impl FnMut(&Fields) -> String) {
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i = fields(a);
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
fn enemy_shots_fly_like_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/enemyshot.txt")) else {
        eprintln!("skipped: enemyshot.txt not found (run re/tools/emu_enemyshot.py)");
        return;
    };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let levels: Vec<_> = (1..=15).map(|i| hlvl::read(ep.get(b"Hlvl", i).unwrap()).unwrap()).collect();
    let shot_mask = masks::from_cicn(&gfx, 1299).unwrap();
    let obstacle = masks::from_cicn(&gfx, 290).unwrap();
    let walls = masks::walls(&app).unwrap();
    let ms = ShotMasks { shot: &shot_mask, walls: &walls, obstacle: &obstacle };
    compare(&text, |i| {
        let w = |k: &str| i[k].parse::<i32>().unwrap() as i16;
        let level = &levels[w("level") as usize - 1];
        let mut sp = shot(i, w("kind"), w("v"), w("h"));
        let harry = (w("hv").wrapping_add(32), w("hh").wrapping_add(32));
        let mut events = Vec::new();
        for _ in 0..w("frames") {
            handle_enemy_shot(&mut sp, harry, &mut events);
            let mut hits = Vec::new();
            touched_tiles(level, sp.at, sp.hot_rect, [0, 0, 8, 8], |h| hits.push(h));
            for h in hits {
                hit_enemy_shot_tile(&mut sp, h, level, &ms);
            }
        }
        format!("{} | events={}", dump(&sp), common::events(&events))
    });
}

#[test]
fn enemy_shot_hits_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/enemyshothit.txt")) else {
        eprintln!("skipped: enemyshothit.txt not found (run re/tools/emu_enemyshot.py)");
        return;
    };
    compare(&text, |i| {
        let w = |k: &str| i[k].parse::<i32>().unwrap() as i16;
        let mut sp = shot(i, w("kind"), w("v"), w("h"));
        sp.frozen = w("frozen") != 0;
        let okind = w("okind");
        let mut o = Sprite::new(okind, w("oh"), w("ov"));
        match i["other"] {
            "playershot" => {
                o.task = Some(Task::PlayerShot);
                o.at = (w("ov"), w("oh"));
                o.hot_rect = [0, 0, 8, 8];
            }
            "collider" => setup_collider(&mut o, &mut Random::new(1)),
            "cooler" => setup_cooler(&mut o),
            "plant" => setup_plant(&mut o),
            "enemyshot" => {
                setup_enemy_shot(&mut o);
                if (1101..=1299).contains(&okind) {
                    o.timer = 0;
                }
            }
            _ => setup_bonus(&mut o),
        }
        o.frozen = w("ofrozen") != 0;
        let mut events = Vec::new();
        hit_enemy_shot(&mut sp, &mut o, &mut events);
        format!("{} || {} | events={}", dump(&sp), dump(&o), common::events(&events))
    });
}

#[test]
fn aiming_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/aim.txt")) else {
        eprintln!("skipped: aim.txt not found (run re/tools/emu_enemyshot.py)");
        return;
    };
    compare(&text, |i| {
        let w = |k: &str| i[k].parse::<i16>().unwrap();
        harry_core::physics::find_desired_direction((w("tov"), w("toh")), (w("fromv"), w("fromh"))).to_string()
    });
}
