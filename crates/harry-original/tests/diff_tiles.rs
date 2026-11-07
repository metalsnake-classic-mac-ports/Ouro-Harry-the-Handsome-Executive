//! One frame of tile collisions for Harry against the original
//! (`CheckTileCollisions` with `HitPlayerTileSprite`, recorded in Unicorn by
//! `re/tools/emu_tiles.py`), on the real levels with the real masks read
//! from the original files.

mod common;

use common::{Tally, installed, levels_of, numeric_cases, recording, work_text};
use harry_core::Physics;
use harry_core::bounce::Body;
use harry_core::player::{HOT_RECT, KICK_HOT_RECT, Masks, Player, TileState, hit_player_tile};
use harry_core::physics::Speed;
use harry_core::tiles::touched_tiles;
use harry_original::{hrsp, masks};

#[test]
fn tile_collisions_match_the_original() {
    let Some(text) = recording("tiles", "emu_tiles.py") else { return };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin")) else {
        return;
    };
    let faces = (0..144)
        .map(|f| (f >= 107).then(|| masks::from_face(&hrsp::decode(gfx.get(b"HrSp", 1101 + f as i16).unwrap()).unwrap())))
        .collect();
    let ms = Masks {
        body: masks::from_cicn(&gfx, 299).unwrap(),
        obstacle: masks::from_cicn(&gfx, 290).unwrap(),
        walls: masks::walls(&app).unwrap(),
        faces,
    };
    let levels = levels_of(&ep);
    let physics = Physics::default();
    let mut tally = Tally::new("tiles", 8);
    for (line, v, want) in numeric_cases(&text) {
        let level = &levels[v[0] as usize - 1];
        let at = (v[1] as i16, v[2] as i16);
        let kicking = v[6] == 108;
        let mut p = Player {
            body: Body { pos: at, at, fix: (0x7777, 0x7777), speed: Speed { h: v[3] as i16, v: v[4] as i16 } },
            dir: v[5] as i16,
            face: (v[5] + v[6]) as i16,
            air: v[7] as i16,
            hot_rect: if kicking { KICK_HOT_RECT } else { HOT_RECT },
        };
        let mut st = TileState {
            wall_contact: v[9] != 0,
            bg_contact: v[10] != 0,
            ouch_cooldown: v[11] as i16,
            comfort_delta: 0,
            wet: v[8] as i16,
            floor_kind: 1,
            kick_contact: 0,
        };
        let mut sounds = Vec::new();
        let hot = p.hot_rect;
        touched_tiles(level, at, hot, [0, 0, 64, 64], |hit| {
            hit_player_tile(&mut p, &mut st, level, &ms, &physics, hit, &mut sounds)
        });
        let ouches = sounds.iter().filter(|s| matches!(s, harry_core::player::TileSound::Ouch)).count() as i32;
        let got = vec![
            p.body.at.0.into(), p.body.at.1.into(), p.body.pos.0.into(), p.body.pos.1.into(), p.body.fix.0, p.body.fix.1,
            p.body.speed.h.into(), p.body.speed.v.into(), st.wall_contact as i32, st.bg_contact as i32,
            st.ouch_cooldown.into(), st.comfort_delta.into(), st.wet.into(), st.floor_kind.into(), st.kick_contact.into(),
            ouches, sounds.len() as i32 - ouches,
        ];
        tally.numbers(line, &want, &got);
    }
    tally.finish();
}

/// The shape tables against the ones the original computes
/// (`re/tools/emu_tables.py` > re/work/tables/tile_rects.txt).
#[test]
fn tables_match_the_original() {
    use harry_core::tiles::{KICK_RECTS, bg_rect, fg_rect};
    let Some(text) = work_text("tables/tile_rects.txt", "emu_tables.py") else { return };
    let rect = |s: &str| -> [i16; 4] {
        let v: Vec<i16> = s.trim_matches(|c| c == '(' || c == ')').split(", ").map(|x| x.parse().unwrap()).collect();
        [v[0], v[1], v[2], v[3]]
    };
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let (what, rest) = line.split_once(' ').unwrap();
        let rest = rest.trim_start();
        let (k, rects) = rest.split_once(' ').unwrap();
        let k: i16 = k.parse().unwrap();
        let first = &rects[..rects.find(')').unwrap() + 1];
        match what {
            "fg" => assert_eq!(fg_rect(k), rect(first), "fg {k}"),
            "bg" => assert_eq!(bg_rect(k), rect(first), "bg {k}"),
            "kick" => assert_eq!(KICK_RECTS[k as usize], rect(first), "kick {k}"),
            _ => panic!("{line}"),
        }
        n += 1;
    }
    assert_eq!(n, 64 + 96 + 36);
}
