//! Robots against the original, recorded in Unicorn by
//! `re/tools/emu_robot.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::Physics;
use harry_core::air::AirSound;
use harry_core::button::setup_sliding_door;
use harry_core::collider::setup_collider;
use harry_core::coworker::setup_coworker;
use harry_core::effect::setup_effect;
use harry_core::frame::PlayerGlobals;
use harry_core::furniture::{setup_barrel, setup_chair};
use harry_core::kinds::{setup_cooler, setup_door, setup_plant};
use harry_core::player::{HOT_RECT, KICK_HOT_RECT};
use harry_core::ramp::setup_ramp;
use harry_core::random::Random;
use harry_core::robot::{RobotWorld, handle_robot, hit_robot, hit_robot_tile, setup_robot};
use harry_core::sprite::{Sprite, Task};
use harry_core::stairs::setup_stairs;
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

/// The robots' face bounds (HrSp 4201… 32 × 32, 4701… 40 × 40).
fn bounds(face: i16) -> [i16; 4] {
    if face >= 4701 { [0, 0, 40, 40] } else { [0, 0, 32, 32] }
}

#[test]
fn robot_tasks_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/robottask.txt")) else {
        eprintln!("skipped: robottask.txt not found (run re/tools/emu_robot.py)");
        return;
    };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let levels: Vec<_> = (1..=15).map(|i| hlvl::read(ep.get(b"Hlvl", i).unwrap()).unwrap()).collect();
    // cicn 598 encoded as face 4251: the faces' mirrored bits
    let mask = masks::from_face(&harry_original::hrsp::decode(gfx.get(b"HrSp", 4251).unwrap()).unwrap());
    let obstacle = masks::from_cicn(&gfx, 290).unwrap();
    let walls = masks::walls(&app).unwrap();
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap() as i16;
        let mut level = levels[w("level") as usize - 1].clone();
        let index = w("index");
        level.sprites[index as usize].info[3] = w("info3");
        let mut g = PlayerGlobals { player_pos: (w("ph"), w("pv")), ..Default::default() };
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_robot(&mut sp, g.player_pos);
        sp.level_index = index;
        if w("hp") != -1 {
            sp.state = w("hp");
        }
        if w("mode") != -1 {
            sp.f7c = w("mode");
        }
        sp.timer = w("timer");
        sp.speed.h = w("speedh");
        sp.speed.v = w("speedv");
        sp.air = w("air");
        sp.frozen = w("frozen") != 0;
        let mut rng = Random::new(i["seed"].parse::<u32>().unwrap());
        let unset = 0; // the port's value (see BUGS.md)
        let (mut wall, mut obst) = (false, false);
        let mut evs = Vec::new();
        for step in i["steps"].split(',') {
            let (dh, dv) = step.split_once(':').unwrap();
            g.player_pos.0 = g.player_pos.0.wrapping_add(dh.parse().unwrap());
            g.player_pos.1 = g.player_pos.1.wrapping_add(dv.parse().unwrap());
            let mut events = Vec::new();
            let mut sounds = Vec::new();
            {
                let mut rw = RobotWorld { level: &mut level, g: &mut g, rng: &mut rng, wall_done: &mut wall, obstacle_done: &mut obst };
                handle_robot(&mut sp, &mut rw, unset, &mut events, &mut sounds);
            }
            let mut log: Vec<String> = events.iter().map(common::event).collect();
            log.extend(sounds.iter().filter(|s| matches!(s, AirSound::Land | AirSound::Bounce)).map(|_| "p:600".to_string()));
            let mut hits = Vec::new();
            touched_tiles(&level, sp.at, sp.hot_rect, bounds(sp.face), |h| hits.push(h));
            for h in hits {
                hit_robot_tile(&mut sp, h, &level, &mask, &walls, &obstacle, &mut wall, &mut obst);
            }
            evs.push(if log.is_empty() { "-".to_string() } else { log.join(",") });
        }
        let got = format!(
            "{} | entry={} score={} chatter={} events={}",
            dump(&sp),
            u8::from(level.sprites[index as usize].present),
            g.score,
            g.chatter,
            evs.join(";")
        );
        n += 1;
        report(a, b, &got, &mut bad);
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}

#[test]
fn robot_hits_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/robothit.txt")) else {
        eprintln!("skipped: robothit.txt not found (run re/tools/emu_robot.py)");
        return;
    };
    let physics = Physics::default();
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap() as i16;
        let mut rng = Random::new(i["seed"].parse::<u32>().unwrap());
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_robot(&mut sp, (w("harryh"), w("harryv")));
        let okind = w("okind");
        let p = i["other"];
        let mut o = Sprite::new(if p == "effect" { 0 } else { okind }, w("oh"), w("ov"));
        match p {
            "harry" => {
                o.task = Some(Task::Player);
                o.hot_rect = if w("kicking") != 0 { KICK_HOT_RECT } else { HOT_RECT };
            }
            "barrel" => setup_barrel(&mut o),
            "effect" => {
                setup_effect(&mut o);
                o.kind = okind;
            }
            "ramp" => setup_ramp(&mut o),
            "stairs" => setup_stairs(&mut o),
            "plant" => setup_plant(&mut o),
            "cooler" => setup_cooler(&mut o),
            "collider" => setup_collider(&mut o, &mut rng),
            "coworker" => setup_coworker(&mut o),
            "chair" => setup_chair(&mut o, &mut rng),
            "door" => setup_door(&mut o),
            "grate" => harry_core::grate::setup_grate(&mut o),
            "desk" => harry_core::desk::setup_desk(&mut o),
            _ => setup_sliding_door(&mut o),
        }
        o.at = (w("ov"), w("oh"));
        o.dir = w("odir");
        for (s, pre) in [(&mut sp, ""), (&mut o, "o")] {
            let k = |n: &str| i[format!("{pre}{n}").as_str()].parse::<i64>().unwrap();
            s.pos = (k("pv") as i16, k("ph") as i16);
            s.fix = (k("fixh") as i32, k("fixv") as i32);
            s.speed.h = k("speedh") as i16;
            s.speed.v = k("speedv") as i16;
            s.frozen = k("frozen") != 0;
            let hot: Vec<i16> = i[format!("{pre}hot").as_str()].split(':').map(|x| x.parse().unwrap()).collect();
            s.hot_rect = [hot[0], hot[1], hot[2], hot[3]];
        }
        sp.f7c = w("mode");
        sp.timer = w("timer");
        sp.blast_cooldown = w("blast");
        sp.f94 = w("f94");
        let mut contact = w("contact");
        let mut rng = Random::new(i["seed2"].parse::<u32>().unwrap());
        let mut events = Vec::new();
        hit_robot(&mut sp, &mut o, &physics, &mut rng, &mut contact, w("caffeine") > 0, &mut events);
        let got = format!("{} || {} | contact={contact} events={}", dump(&sp), dump(&o), common::events(&events));
        n += 1;
        report(a, b, &got, &mut bad);
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
