//! Turrets, reversors and boulders against the original, recorded in
//! Unicorn by `re/tools/emu_turret.py`.

mod common;

use std::collections::HashMap;

use common::{Tally, dump, installed, levels_of, named_cases, recording, work};
use harry_core::bounce::{FirstBounce, TileMasks};
use harry_core::Physics;
use harry_core::air::AirSound;
use harry_core::boulder::{handle_boulder, hit_boulder, hit_boulder_tile, setup_boulder};
use harry_core::furniture::setup_barrel;
use harry_core::kinds::{setup_door, setup_plant};
use harry_core::random::Random;
use harry_core::reversor::{handle_reversor, hit_reversor, hit_reversor_tile, setup_reversor};
use harry_core::sprite::{Sprite, SpriteEvent, Task};
use harry_core::tiles::touched_tiles;
use harry_core::turret::{handle_turret, hit_turret, setup_turret};
use harry_original::masks;

/// A face's bounds from its `HrSp` header.
fn bounds(face: i16) -> [i16; 4] {
    let Ok(b) = std::fs::read(work(&format!("res/Harry_Graphics/HrSp_{face}.bin"))) else { return [0; 4] };
    let s = |i: usize| i16::from_be_bytes([b[2 * i], b[2 * i + 1]]);
    [s(0), s(1), s(2), s(3)]
}

/// The partner of a hit, made as the recorder makes it next to the sprite.
fn partner(name: &str, sp: &Sprite) -> Sprite {
    let (ov, oh) = (sp.at.0 + 4, sp.at.1 + 4);
    let mut o = Sprite::new(0, oh, ov);
    match name {
        "barrel" | "plant" | "door" | "desk" => {
            o.kind = match name {
                "barrel" => 550,
                "plant" => 601,
                "desk" => 301,
                _ => 901,
            };
            match name {
                "barrel" => setup_barrel(&mut o),
                "plant" => setup_plant(&mut o),
                "desk" => harry_core::desk::setup_desk(&mut o),
                _ => setup_door(&mut o),
            }
        }
        _ => {
            o.task = Some(match name {
                "shot" => Task::PlayerShot,
                "eshot" => Task::EnemyShot,
                "robot" => Task::Robot,
                _ => Task::Effect,
            });
            if name == "fx" {
                o.hot_rect = [4, 4, 60, 60];
            }
            if name == "shot" || name == "eshot" {
                o.speed.h = 700;
                o.speed.v = -300;
                o.fix = (i32::from(oh) << 8, i32::from(ov) << 8);
            }
            if name == "robot" {
                o.state = 3;
            }
        }
    }
    o.at = (ov, oh);
    o
}

fn log(events: &[SpriteEvent], score: &mut i32) -> Vec<String> {
    let mut out = Vec::new();
    for e in events {
        match *e {
            SpriteEvent::Score(n) => *score += n,
            ref e => out.push(common::event(e)),
        }
    }
    out
}

fn run(group: &str) {
    let Some(text) = recording(group, "emu_turret.py") else { return };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let levels = levels_of(&ep);
    // cicn 598 encoded as face 4251: the faces' mirrored bits
    let mask = masks::from_face(&harry_original::hrsp::decode(gfx.get(b"HrSp", 4251).unwrap()).unwrap());
    let obstacle = masks::from_cicn(&gfx, 290).unwrap();
    let walls = masks::walls(&app).unwrap();
    let physics = Physics::default();
    let mut face_bounds: HashMap<i16, [i16; 4]> = HashMap::new();
    let mut tally = Tally::new(group, 8);
    for (a, i, b) in named_cases(&text) {
        let w = |k: &str| i.i16(k);
        let mut level = levels[w("level") as usize - 1].clone();
        let index = w("index");
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        match group {
            "turret" => {
                level.sprites[index as usize].info[3] = w("info3");
                setup_turret(&mut sp);
                if w("hp") != -1 {
                    sp.state = w("hp");
                }
            }
            "boulder" => {
                for k in 0..3 {
                    level.sprites[index as usize].info[k] = w(&format!("info{k}"));
                }
                setup_boulder(&mut sp);
            }
            _ => {
                setup_reversor(&mut sp);
                sp.air = w("air");
            }
        }
        sp.level_index = index;
        let mut player = (w("ph"), w("pv"));
        let mut rng = Random::new(i.u32("seed"));
        let mut done = FirstBounce::default();
        let mut score = 0;
        let mut evs = Vec::new();
        for step in i.str("steps").split(',') {
            let p: Vec<&str> = step.split(':').collect();
            player.0 = player.0.wrapping_add(p[0].parse().unwrap());
            player.1 = player.1.wrapping_add(p[1].parse().unwrap());
            let mut events = Vec::new();
            let mut extra = Vec::new();
            match group {
                "turret" => {
                    let drop = level.sprites[index as usize].info[3];
                    handle_turret(&mut sp, player, drop, &mut rng, &mut events);
                }
                "boulder" => handle_boulder(&mut sp, &mut level, &mut rng, &mut events),
                _ => {
                    let mut sounds = Vec::new();
                    handle_reversor(&mut sp, &level, &mut rng, &mut done, &mut sounds);
                    extra.extend(sounds.iter().filter(|s| matches!(s, AirSound::Land | AirSound::Bounce)).map(|_| "p:600".to_string()));
                }
            }
            let mut l = log(&events, &mut score);
            l.extend(extra);
            if group != "turret" {
                let fb = *face_bounds.entry(sp.face).or_insert_with(|| bounds(sp.face));
                let mut hits = Vec::new();
                touched_tiles(&level, sp.at, sp.hot_rect, fb, |h| hits.push(h));
                for h in hits {
                    if group == "boulder" {
                        hit_boulder_tile(&mut sp, h);
                    } else {
                        hit_reversor_tile(&mut sp, h, &TileMasks { level: &level, mask: &mask, walls: &walls, obstacle: &obstacle }, &mut done);
                    }
                }
            }
            if p[2] != "-" {
                let mut o = partner(p[2], &sp);
                let mut events = Vec::new();
                match group {
                    "turret" => hit_turret(&mut sp, &mut o, &mut events),
                    "boulder" => hit_boulder(&sp, &mut o),
                    _ => hit_reversor(&mut sp, &mut o, &physics, &mut events),
                }
                l.extend(log(&events, &mut score));
                l.push(format!("o:{}:{}:{}:{}:{}:{}", u8::from(o.frozen), o.state, o.dir, o.speed.h, o.speed.v, o.fix.0));
            }
            evs.push(if l.is_empty() { "-".to_string() } else { l.join(",") });
        }
        let e = &level.sprites[index as usize];
        let got = format!(
            "{} | entry={}:{}:{}:{}:{} score={score} events={}",
            dump(&sp),
            u8::from(e.present),
            e.id,
            e.info[0],
            e.info[1],
            e.info[2],
            evs.join(";")
        );
        tally.words(a, b, &got);
    }
    tally.finish();
}

#[test]
fn turrets_match_the_original() {
    run("turret");
}

#[test]
fn reversors_match_the_original() {
    run("reversor");
}

#[test]
fn boulders_match_the_original() {
    run("boulder");
}
