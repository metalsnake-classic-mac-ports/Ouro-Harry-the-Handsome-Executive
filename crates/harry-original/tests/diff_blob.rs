//! Scientists and blobs against the original, recorded in Unicorn by
//! `re/tools/emu_blob.py`.

mod common;

use std::collections::HashMap;

use common::{Tally, dump, installed, levels_of, named_cases, recording, work};
use harry_core::Physics;
use harry_core::air::AirSound;
use harry_core::blob::{handle_blob, hit_blob, hit_blob_tile, setup_blob};
use harry_core::kinds::{setup_cooler, setup_door, setup_plant};
use harry_core::random::Random;
use harry_core::scientist::{handle_scientist, hit_scientist, setup_scientist};
use harry_core::sprite::{Sprite, SpriteEvent, Task};
use harry_core::tiles::touched_tiles;
use harry_original::masks;

fn bounds(face: i16) -> [i16; 4] {
    let Ok(b) = std::fs::read(work(&format!("res/Harry_Graphics/HrSp_{face}.bin"))) else { return [0; 4] };
    let s = |i: usize| i16::from_be_bytes([b[2 * i], b[2 * i + 1]]);
    [s(0), s(1), s(2), s(3)]
}

fn partner(name: &str, sp: &Sprite, dir: i16) -> Sprite {
    let (mut ov, mut oh) = (sp.at.0 + 4, sp.at.1 + 4);
    if name == "kick" {
        ov -= 30;
        oh -= 30;
    }
    let mut o = Sprite::new(0, oh, ov);
    match name {
        "plant" | "door" | "cooler" => {
            o.kind = match name {
                "plant" => 601,
                "door" => 902,
                _ => 500,
            };
            match name {
                "plant" => setup_plant(&mut o),
                "door" => setup_door(&mut o),
                _ => setup_cooler(&mut o),
            }
        }
        _ => {
            o.task = Some(match name {
                "shot" | "shotf" => Task::PlayerShot,
                "kick" => Task::Player,
                _ => Task::Effect,
            });
            if name == "fx" {
                o.hot_rect = [4, 4, 60, 60];
            }
            o.frozen = name == "shotf";
            if name == "kick" {
                o.dir = dir.rem_euclid(36);
            }
        }
    }
    o.at = (ov, oh);
    o
}

fn run(group: &str) {
    let Some(text) = recording(group, "emu_blob.py") else { return };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let levels = levels_of(&ep);
    // cicn 598 encoded as face 1451: the faces' mirrored bits
    let mask = masks::from_face(&harry_original::hrsp::decode(gfx.get(b"HrSp", 1451).unwrap()).unwrap());
    let walls = masks::walls(&app).unwrap();
    let physics = Physics::default();
    let mut face_bounds: HashMap<i16, [i16; 4]> = HashMap::new();
    let mut tally = Tally::new(group, 8);
    for (a, i, b) in named_cases(&text) {
        let w = |k: &str| i.i16(k);
        let mut level = levels[w("level") as usize - 1].clone();
        let index = w("index");
        let mut rng = Random::new(i.u32("seed"));
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        if group == "scientist" {
            setup_scientist(&mut sp);
            if w("mode") != -1 {
                sp.f6e = i32::from(w("mode"));
            }
        } else {
            level.sprites[index as usize].info[3] = w("info3");
            setup_blob(&mut sp, &mut rng);
            if w("hp") != -1 {
                sp.state = w("hp");
            }
            sp.air = w("air");
        }
        sp.level_index = index;
        let mut player = (w("ph"), w("pv"));
        let mut view = (w("vv"), w("vh"));
        let (mut wall, mut obst) = (false, false);
        let (mut score, mut favor, mut contact) = (0, 100i16, 0i16);
        let mut evs = Vec::new();
        for step in i.str("steps").split(',') {
            let p: Vec<&str> = step.split(':').collect();
            let (dh, dv): (i16, i16) = (p[0].parse().unwrap(), p[1].parse().unwrap());
            player = (player.0.wrapping_add(dh), player.1.wrapping_add(dv));
            view = (view.0.wrapping_add(dv), view.1.wrapping_add(dh));
            let mut events = Vec::new();
            let mut sounds = Vec::new();
            if group == "scientist" {
                handle_scientist(&mut sp, view, player, &mut rng, &mut events);
            } else {
                let drop = level.sprites[index as usize].info[3];
                handle_blob(&mut sp, &level, player, drop, &mut rng, &mut wall, &mut obst, &mut events, &mut sounds);
                let fb = *face_bounds.entry(sp.face).or_insert_with(|| bounds(sp.face));
                let mut hits = Vec::new();
                touched_tiles(&level, sp.at, sp.hot_rect, fb, |h| hits.push(h));
                for h in hits {
                    hit_blob_tile(&mut sp, h, &mask, &walls, &mut wall, &mut obst);
                }
            }
            let mut l = Vec::new();
            for e in &events {
                match *e {
                    SpriteEvent::Score(s) => score += s,
                    ref e => l.push(common::event(e)),
                }
            }
            l.extend(sounds.iter().filter(|s| matches!(s, AirSound::HalfPipeOut | AirSound::Land | AirSound::Bounce)).map(|_| "p:604".to_string()));
            if p[2] != "-" {
                let mut o = partner(p[2], &sp, dh);
                let mut events = Vec::new();
                if group == "scientist" {
                    hit_scientist(&mut sp, &mut o, &mut favor, &mut rng, &mut events);
                } else {
                    hit_blob(&mut sp, &o, &physics, &mut contact, &mut events);
                }
                l.extend(events.iter().map(common::event));
                l.push(format!("o:{}", u8::from(o.frozen)));
            }
            evs.push(if l.is_empty() { "-".to_string() } else { l.join(",") });
        }
        let got = format!(
            "{} | entry={} score={score} favor={favor} contact={contact} events={}",
            dump(&sp),
            u8::from(level.sprites[index as usize].present),
            evs.join(";")
        );
        tally.words(a, b, &got);
    }
    tally.finish();
}

#[test]
fn scientists_match_the_original() {
    run("scientist");
}

#[test]
fn blobs_match_the_original() {
    run("blob");
}
