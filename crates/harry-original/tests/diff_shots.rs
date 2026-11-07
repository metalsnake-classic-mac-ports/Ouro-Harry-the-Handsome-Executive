//! The held weapon, new shots and their flight against the original,
//! recorded in Unicorn by `re/tools/emu_shots.py`.

mod common;

use common::{Tally, installed, levels_of, recording};
use harry_core::frame::PlayerGlobals;
use harry_core::keys::Ammo;
use harry_core::physics::Speed;
use harry_core::random::Random;
use harry_core::shots::{
    HeldWeapon, Holder, ShotFace, WeaponEvent, handle_held_weapon, handle_shot, hit_shot_tile, new_shot,
};
use harry_core::tiles::touched_tiles;
use harry_original::{hrsp, masks};

/// The recording `name` (a header, then numbers -> words): `run` makes the
/// port's words from the numbers.
fn compare(name: &str, mut run: impl FnMut(&[i64]) -> Vec<String>) {
    let Some(text) = recording(name, "emu_shots.py") else { return };
    let mut tally = Tally::new(name, 10);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: Vec<i64> = a.split(' ').map(|x| x.parse().unwrap()).collect();
        let want: Vec<&str> = b.split(' ').collect();
        let got = run(&i);
        tally.case(want == got, || format!("{a}\n  want {}\n  got  {}", want.join(" "), got.join(" ")));
    }
    tally.finish();
}

#[test]
fn held_weapon_matches_the_original() {
    compare("weapon", |i| {
        let w16 = |k: usize| i[k] as i16;
        let mut g = PlayerGlobals {
            player_dir: w16(0),
            recoil: w16(1),
            fire_hold: w16(2),
            ammo: Ammo { badass: w16(3), ..Default::default() },
            shot_kind: w16(4),
            cur_slot: 0,
            ..Default::default()
        };
        g.inventory = [(-1, 0); 12];
        g.inventory[0].0 = w16(5);
        let holder = Holder { at: (w16(6), w16(7)), scale0: w16(8), scale1: w16(9), face_bounds: [w16(10), w16(11), w16(12), w16(13)] };
        let mut w = HeldWeapon { gun: 0x55, frozen: i[14] != 0, ..Default::default() };
        let mut rng = Random::new(i[15] as u32);
        let mut shots = Vec::new();
        let mut events = Vec::new();
        handle_held_weapon(&mut w, &holder, &mut g, &mut rng, &mut shots, &mut events);
        let shots: Vec<String> =
            shots.iter().map(|s| format!("{}:{}:{}:{}:{}", s.kind, s.h, s.v, s.speed.v, s.speed.h)).collect();
        let events: Vec<String> = events
            .iter()
            .map(|e| match *e {
                WeaponEvent::Sound(s) => format!("p:{s}"),
                WeaponEvent::Sound3D(s) => format!("3d:{s}"),
                WeaponEvent::Explosion { h, v, a, b } => format!("x:{h}:{v}:{a}:{b}"),
            })
            .collect();
        let dash = |v: Vec<String>| if v.is_empty() { "-".to_string() } else { v.join(",") };
        vec![
            w.pos.0.to_string(),
            w.pos.1.to_string(),
            w.at.0.to_string(),
            w.at.1.to_string(),
            w.face.map_or("-".into(), |f| format!("t0.{f}")),
            w.gun.to_string(),
            u8::from(w.charging).to_string(),
            u8::from(w.flag8b).to_string(),
            g.ammo.badass.to_string(),
            dash(shots),
            dash(events),
        ]
    });
}

#[test]
fn shot_flight_matches_the_original() {
    compare("shot", |i| {
        let kind = i[0] as i16;
        let mut s = new_shot(kind, (i[1] as i16, i[2] as i16), Speed { v: i[3] as i16, h: i[4] as i16 }, i[5] as i16);
        let mut states = Vec::new();
        for f in 0..=i[6] {
            if f > 0 {
                handle_shot(&mut s);
            }
            let face = match s.face {
                ShotFace::None => "-".to_string(),
                ShotFace::Frame(n) => format!("t{}.{n}", match kind {
                    0 => 1,
                    1 => 2,
                    _ => 3,
                }),
                ShotFace::Harry(n) => format!("h{n}"),
            };
            states.push(format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                s.pos.0,
                s.pos.1,
                s.at.0,
                s.at.1,
                s.fix.0,
                s.fix.1,
                s.dir,
                s.timer,
                face,
                u8::from(s.flag8b)
            ));
        }
        let r = s.hot_rect;
        vec![states.join(","), format!("{}:{}:{}:{}", r[0], r[1], r[2], r[3]), "11".into()]
    });
}

#[test]
fn shot_tile_hits_match_the_original() {
    let (Some(episode), Some(gfx), Some(app)) =
        (installed("Episode 1.bin"), installed("Harry Graphics.bin"), installed("Harry 1.0.1.bin"))
    else {
        return;
    };
    let levels = levels_of(&episode);
    let walls = masks::walls(&app).unwrap();
    let shot_mask = masks::from_face(&hrsp::decode(gfx.get(b"HrSp", 4021).unwrap()).unwrap());
    compare("shottiles", |i| {
        let w16 = |k: usize| i[k] as i16;
        let level = &levels[i[0] as usize - 1];
        let mut s = new_shot(w16(1), (w16(2), w16(3)), Speed { v: w16(4), h: w16(5) }, 0);
        s.timer = w16(6);
        s.flag8b = false;
        let mut hits = Vec::new();
        touched_tiles(level, s.at, s.hot_rect, [0, 0, w16(7), w16(8)], |h| hits.push(h));
        let mut events = Vec::new();
        for h in hits {
            hit_shot_tile(&mut s, &shot_mask, &walls, h, &mut events);
        }
        let events: Vec<String> = events
            .iter()
            .map(|e| match *e {
                WeaponEvent::Sound(s) => format!("p:{s}"),
                WeaponEvent::Sound3D(s) => format!("3d:{s}"),
                WeaponEvent::Explosion { h, v, a, b } => format!("x:{h}:{v}:{a}:{b}"),
            })
            .collect();
        vec![
            s.pos.0.to_string(),
            s.pos.1.to_string(),
            s.at.0.to_string(),
            s.at.1.to_string(),
            s.fix.0.to_string(),
            s.fix.1.to_string(),
            s.speed.v.to_string(),
            s.speed.h.to_string(),
            u8::from(s.frozen).to_string(),
            u8::from(s.flag8b).to_string(),
            if events.is_empty() { "-".into() } else { events.join(",") },
        ]
    });
}
