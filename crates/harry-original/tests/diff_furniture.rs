//! Barrels and chairs against the original, recorded in Unicorn by
//! `re/tools/emu_furniture.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::Physics;
use harry_core::air::AirSound;
use harry_core::effect::setup_effect;
use harry_core::furniture::{FurnitureMasks, handle_barrel, handle_chair, hit_barrel, hit_chair, setup_barrel, setup_chair};
use harry_core::kinds::{ObjectMasks, hit_object_tile, setup_cooler, setup_door, setup_plant};
use harry_core::player::{HOT_RECT, KICK_HOT_RECT};
use harry_core::ramp::setup_ramp;
use harry_core::stairs::setup_stairs;
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

/// The recorded events without the sound 0 the original passes to
/// `HandleHalfPipe` and `HandleRampCollision` (the port plays none).
fn without_silence(b: &str) -> String {
    let (head, ev) = b.rsplit_once("events=").unwrap();
    let ev: Vec<&str> = ev.split(',').filter(|e| *e != "p:0" && *e != "-").collect();
    format!("{head}events={}", if ev.is_empty() { "-".into() } else { ev.join(",") })
}

fn report(a: &str, want: &str, got: &str, bad: &mut usize) {
    if want != got {
        *bad += 1;
        if *bad <= 8 {
            let diffs: Vec<String> =
                want.split(' ').zip(got.split(' ')).filter(|(x, y)| x != y).map(|(x, y)| format!("want {x} got {y}")).collect();
            eprintln!("{a}\n  {}", diffs.join(", "));
        }
    }
}

#[test]
fn barrel_and_chair_tasks_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/furnituretask.txt")) else {
        eprintln!("skipped: furnituretask.txt not found (run re/tools/emu_furniture.py)");
        return;
    };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let levels: Vec<_> = (1..=15).map(|i| hlvl::read(ep.get(b"Hlvl", i).unwrap()).unwrap()).collect();
    let plant = masks::from_cicn(&gfx, 599).unwrap();
    let barrel = masks::from_cicn(&gfx, 598).unwrap();
    let obstacle = masks::from_cicn(&gfx, 290).unwrap();
    let walls = masks::walls(&app).unwrap();
    let physics = Physics::default();
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let mut level = levels[w("level") as usize - 1].clone();
        let kind = w("kind") as i16;
        let is_barrel = kind == 550;
        let half = if is_barrel { 12 } else { 18 };
        let ms = ObjectMasks { plant: if is_barrel { &barrel } else { &plant }, walls: &walls, obstacle: &obstacle };
        let mut rng = Random::new(w("seed") as u32);
        let mut sp = Sprite::new(kind, w("h") as i16, w("v") as i16);
        if is_barrel {
            setup_barrel(&mut sp);
            sp.dir = w("dir") as i16;
        } else {
            setup_chair(&mut sp, &mut rng);
        }
        sp.speed.h = w("speedh") as i16;
        sp.speed.v = w("speedv") as i16;
        sp.air = w("air") as i16;
        sp.slide = w("slide") as i16;
        sp.timer = w("timer") as i16;
        sp.level_index = w("index") as i16;
        let unset = w("rec") as u16 as i16;
        let mut shake = 0;
        let mut events = Vec::new();
        let mut log = Vec::new();
        for _ in 0..w("frames") {
            let mut sounds = Vec::new();
            if is_barrel {
                handle_barrel(&mut sp, &mut level, &physics, &mut rng, &mut shake, unset, &mut events, &mut sounds);
            } else {
                handle_chair(&mut sp, &level, &physics, &mut rng, unset, &mut sounds);
            }
            log.extend(events.drain(..).map(|e| common::event(&e)));
            for s in sounds {
                let id = match (is_barrel, s) {
                    (true, AirSound::Land | AirSound::Bounce) => 800,
                    (false, AirSound::Land | AirSound::Bounce) => 257,
                    (false, AirSound::HalfPipeOut) => 258,
                    _ => continue,
                };
                log.push(format!("p:{id}"));
            }
            let mut hits = Vec::new();
            touched_tiles(&level, sp.at, sp.hot_rect, [0, 0, 2 * half, 2 * half], |h| hits.push(h));
            for h in hits {
                hit_object_tile(&mut sp, h, &level, &ms, half, &physics);
            }
        }
        let index = w("index");
        let entry = if (0..256).contains(&index) { i64::from(level.sprites[index as usize].present) } else { -1 };
        let ev = if log.is_empty() { "-".into() } else { log.join(",") };
        let got = format!("{} | entry={entry} shake={shake} events={ev}", dump(&sp));
        n += 1;
        report(a, &without_silence(b), &got, &mut bad);
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}

#[test]
fn barrel_and_chair_hits_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/furniturehit.txt")) else {
        eprintln!("skipped: furniturehit.txt not found (run re/tools/emu_furniture.py)");
        return;
    };
    let Some(gfx) = installed("Harry Graphics.bin") else { return };
    let body = masks::from_cicn(&gfx, 299).unwrap();
    let plant = masks::from_cicn(&gfx, 599).unwrap();
    let barrel = masks::from_cicn(&gfx, 598).unwrap();
    let ms = FurnitureMasks { body: &body, plant: &plant, barrel: &barrel };
    let physics = Physics::default();
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let kind = w("kind") as i16;
        let mut rng = Random::new(w("seed") as u32);
        let mut sp = Sprite::new(kind, w("h") as i16, w("v") as i16);
        if kind == 550 {
            setup_barrel(&mut sp);
        } else {
            setup_chair(&mut sp, &mut rng);
        }
        sp.at = (w("v") as i16, w("h") as i16);
        let ok = w("other");
        let okind = match ok {
            1 => 69,
            2 => 550,
            3 => 560,
            4 => 601,
            5 => 500,
            _ => w("okind") as i16,
        };
        let mut o = Sprite::new(okind, w("oh") as i16, w("ov") as i16);
        match ok {
            2 => setup_barrel(&mut o),
            3 => setup_chair(&mut o, &mut rng),
            4 => setup_plant(&mut o),
            5 => setup_cooler(&mut o),
            6 => setup_door(&mut o),
            8 => setup_ramp(&mut o),
            12 => setup_stairs(&mut o),
            11 => {
                o.kind = 0;
                setup_effect(&mut o);
                o.kind = okind;
                o.f7c = okind;
            }
            _ => {
                o.task = Some(match ok {
                    1 => Task::Player,
                    7 => Task::Collider,
                    9 => Task::PlayerShot,
                    _ => Task::EnemyShot,
                });
                if ok == 1 {
                    o.hot_rect = if w("kicking") != 0 { KICK_HOT_RECT } else { HOT_RECT };
                }
            }
        }
        o.at = (w("ov") as i16, w("oh") as i16);
        for (s, pre) in [(&mut sp, ""), (&mut o, "o")] {
            let k = |n: &str| i[format!("{pre}{n}").as_str()].parse::<i64>().unwrap();
            s.pos = (k("pv") as i16, k("ph") as i16);
            s.fix = (k("fixh") as i32, k("fixv") as i32);
            s.speed.h = k("speedh") as i16;
            s.speed.v = k("speedv") as i16;
            s.air = k("air") as i16;
            s.frozen = k("frozen") != 0;
            s.f6e = k("f6e") as i32;
            let hot: Vec<i16> = i[format!("{pre}hot").as_str()].split(':').map(|x| x.parse().unwrap()).collect();
            s.hot_rect = [hot[0], hot[1], hot[2], hot[3]];
        }
        o.dir = w("odir") as i16;
        sp.dir = w("dir") as i16;
        sp.timer = w("timer") as i16;
        sp.blast_cooldown = w("blast") as i16;
        sp.f94 = w("f94") as i16;
        let mut contact = w("contact") as i16;
        let mut rng = Random::new(w("seed2") as u32);
        let busy = w("busy") != 0;
        let mut events = Vec::new();
        if kind == 550 {
            hit_barrel(&mut sp, &mut o, &ms, &physics, &mut rng, &mut contact, w("caffeine") > 0, busy, &mut events);
        } else {
            hit_chair(&mut sp, &mut o, &ms, &physics, &mut contact, busy, &mut events);
        }
        let got = format!("{} || {} | contact={contact} events={}", dump(&sp), dump(&o), common::events(&events));
        n += 1;
        report(a, &without_silence(b), &got, &mut bad);
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
