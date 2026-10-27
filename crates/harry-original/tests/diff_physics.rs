//! The player physics against the original's PPC routines, recorded in
//! Unicorn by `re/tools/emu_physics.py` (re/work/diff/*.txt, one case per
//! line: inputs -> outputs). Without the recordings the tests are skipped.

use std::path::PathBuf;

use harry_core::moves::{Chair, MoveInput, Moves, move_chair};
use harry_core::physics::{Floor, Physics, Speed, apply_moded_impulse, enforce_max_speed, player_glide};

fn cases(name: &str) -> Option<Vec<(Vec<i32>, Vec<i32>)>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/work/diff").join(format!("{name}.txt"));
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("skipped: {} not found (run re/tools/emu_physics.py)", path.display());
        return None;
    };
    let parse = |s: &str| s.split_whitespace().map(|n| n.parse().unwrap()).collect();
    Some(
        text.lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let (a, b) = l.split_once("->").unwrap();
                (parse(a), parse(b))
            })
            .collect(),
    )
}

fn check(name: &str, f: impl Fn(&[i32]) -> Vec<i32>) {
    let Some(cases) = cases(name) else { return };
    let mut bad = 0;
    for (i, (input, want)) in cases.iter().enumerate() {
        let got = f(input);
        if &got != want {
            bad += 1;
            if bad <= 10 {
                eprintln!("{name} case {i}: in {input:?}\n  original {want:?}\n  port     {got:?}");
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {} {name} cases differ", cases.len());
}

fn physics(v: &[i32]) -> Physics {
    Physics { scoot: v[0] as i16, kick: v[1] as i16, friction: v[2] as i16, slippery: v[3] as i16, bounce: v[4] as i16 }
}

#[test]
fn moded_impulse_matches() {
    check("impulse", |v| {
        let mut s = Speed { h: v[2] as i16, v: v[3] as i16 };
        apply_moded_impulse(&mut s, v[0] as i16, v[1] as i16);
        vec![s.h.into(), s.v.into()]
    });
}

#[test]
fn max_speed_matches() {
    check("maxspeed", |v| {
        let mut s = Speed { h: v[1] as i16, v: v[2] as i16 };
        enforce_max_speed(&mut s, v[0]);
        vec![s.h.into(), s.v.into()]
    });
}

#[test]
fn glide_matches() {
    check("glide", |v| {
        let ph = physics(v);
        let floor = Floor { hurt: v[5] as i16, kind: v[6] as i16, wet: v[7] as i16, glide: v[8] as i16 };
        let mut s = Speed { h: v[10] as i16, v: v[11] as i16 };
        let surface = player_glide(&mut s, floor, &ph).unwrap_or(v[9] as i16);
        vec![s.h.into(), s.v.into(), surface.into()]
    });
}

#[test]
fn move_chair_matches() {
    check("movechair", |v| {
        let ph = physics(v);
        let v = &v[5..];
        let mut chair = Chair {
            speed: Speed { h: v[12] as i16, v: v[13] as i16 },
            dir: v[10] as i16,
            hurt: v[0] as i16,
            hot_rect: [22, 22, 42, 42],
        };
        let mut m = Moves {
            scoot: v[1] as i16,
            kick: v[2] as i16,
            inch: v[3] as i16,
            pose: v[4] as i16,
            impulse: v[5] as i16,
            kick_power: v[9] as i16,
        };
        let input = MoveInput { caffeine: v[6] as i16, wet: v[7] as i16, floor_kind: v[11] as i16, kick_contact: v[8] as i16 };
        let mut sounds = Vec::new();
        move_chair(&mut chair, &mut m, &ph, &input, &mut sounds);
        let mut out: Vec<i32> = [m.scoot, m.kick, m.inch, m.pose, m.impulse, m.kick_power, chair.speed.h, chair.speed.v]
            .iter()
            .map(|&x| x.into())
            .collect();
        out.extend(chair.hot_rect.iter().map(|&x| i32::from(x)));
        out.push(sounds.len() as i32);
        out
    });
}

#[test]
fn wall_bounce_matches() {
    use harry_core::bounce::{Body, wall_bounce};
    check("wallbounce", |v| {
        let mut b = Body {
            pos: (v[6] as i16, v[7] as i16),
            at: (v[4] as i16, v[5] as i16),
            fix: (0x7777, 0x7777),
            speed: Speed { h: v[8] as i16, v: v[9] as i16 },
        };
        let mut calls = 0;
        let r = wall_bounce(&mut b, v[0] as i16, (v[2] as i16, v[3] as i16), 32, v[10] as i16, [22, 22, 42, 42], v[1] != 0, &mut |_| {
            calls += 1;
            v[11] != 0
        });
        vec![r, b.at.0.into(), b.at.1.into(), b.pos.0.into(), b.pos.1.into(), b.speed.h.into(), b.speed.v.into(), b.fix.0, b.fix.1, calls]
    });
}

#[test]
fn rect_bounce_matches() {
    use harry_core::bounce::{Body, rect_bounce_fake};
    check("rectbounce", |v| {
        let mut b = Body {
            pos: (0, 0),
            at: (v[6] as i16, v[7] as i16),
            fix: (0x7777, 0x7777),
            speed: Speed { h: v[8] as i16, v: v[9] as i16 },
        };
        let rect = [v[2] as i16, v[3] as i16, v[4] as i16, v[5] as i16];
        rect_bounce_fake(&mut b, (v[0] as i16, v[1] as i16), rect, 32, v[10], [22, 22, 42, 42]);
        vec![b.at.0.into(), b.at.1.into(), b.speed.h.into(), b.speed.v.into(), b.fix.0, b.fix.1]
    });
}

#[test]
fn bg_tile_bounce_matches() {
    use harry_core::bounce::{Body, bg_tile_bounce};
    check("bgbounce", |v| {
        let kinds = [v[0] as i16, v[1] as i16, v[2] as i16, v[3] as i16];
        let (tv, th) = (v[5] as i16, v[6] as i16);
        let (x, y) = (i32::from(th) >> 5, i32::from(tv) >> 5);
        let fg = |a: i32, b: i32| match (a - x, b - y) {
            (0, -1) => kinds[0],
            (1, 0) => kinds[1],
            (-1, 0) => kinds[2],
            (0, 1) => kinds[3],
            _ => panic!("unexpected tile lookup"),
        };
        let mut b = Body {
            pos: (v[7] as i16, v[8] as i16),
            at: (v[7] as i16, v[8] as i16),
            fix: (0x7777, 0x7777),
            speed: Speed { h: v[9] as i16, v: v[10] as i16 },
        };
        let mut calls = 0;
        let r = bg_tile_bounce(&mut b, (tv, th), 32, v[11] as i16, [22, 22, 42, 42], v[4] != 0, &mut |_| {
            calls += 1;
            v[12] != 0
        }, &fg);
        vec![r, b.at.0.into(), b.at.1.into(), b.speed.h.into(), b.speed.v.into(), b.fix.0, b.fix.1, calls]
    });
}

fn load_mask(name: &str) -> harry_core::mask::Mask {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/work/diff/masks").join(format!("{name}.txt"));
    let text = std::fs::read_to_string(path).unwrap();
    let rows: Vec<&str> = text.lines().collect();
    let bits = rows.iter().flat_map(|r| r.chars().map(|c| c == '#')).collect();
    harry_core::mask::Mask::new(rows[0].len() as i32, rows.len() as i32, bits)
}

#[test]
fn mask_test_matches() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/work/diff/masktest.txt");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("skipped: {} not found (run re/tools/emu_mask.py)", path.display());
        return;
    };
    let body = load_mask("body");
    let mut cache = std::collections::HashMap::new();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let (a, b) = line.split_once("->").unwrap();
        let mut it = a.split_whitespace();
        let name = it.next().unwrap().to_string();
        let v: Vec<i16> = it.map(|x| x.parse().unwrap()).collect();
        let want: Vec<i32> = b.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let tile = cache.entry(name.clone()).or_insert_with(|| load_mask(&name));
        let mut pos = (v[2], v[3]);
        let hit = harry_core::mask::mask_hit(&mut pos, &body, (v[0], v[1]), tile);
        let got = vec![hit as i32, pos.0.into(), pos.1.into()];
        n += 1;
        if got != want {
            bad += 1;
            if bad <= 10 {
                eprintln!("masktest: {line}\n  port {got:?}");
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} mask cases differ");
}

#[test]
fn wall_bounce_with_masks_matches() {
    use harry_core::bounce::{Body, wall_bounce};
    use harry_core::mask::mask_hit;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/work/diff/masks/body.txt");
    if !path.exists() {
        eprintln!("skipped: masks not found (run re/tools/emu_mask.py)");
        return;
    }
    let body_mask = load_mask("body");
    let walls: Vec<_> = (0..32).map(|k| load_mask(&format!("wall{k}"))).collect();
    check("wallchain", |v| {
        let mut b = Body {
            pos: (v[4] as i16, v[5] as i16),
            at: (v[4] as i16, v[5] as i16),
            fix: (0x7777, 0x7777),
            speed: Speed { h: v[6] as i16, v: v[7] as i16 },
        };
        let tile = (v[2] as i16, v[3] as i16);
        let wall = &walls[v[0] as usize];
        let r = wall_bounce(&mut b, v[0] as i16, tile, 32, 150, [22, 22, 42, 42], v[1] != 0, &mut |body| {
            mask_hit(&mut body.pos, &body_mask, tile, wall)
        });
        vec![r, b.at.0.into(), b.at.1.into(), b.pos.0.into(), b.pos.1.into(), b.speed.h.into(), b.speed.v.into(), b.fix.0, b.fix.1]
    });
}

#[test]
fn bg_tile_bounce_with_masks_matches() {
    use harry_core::bounce::{Body, bg_tile_bounce};
    use harry_core::mask::mask_hit;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/work/diff/masks/obstacle.txt");
    if !path.exists() {
        eprintln!("skipped: masks not found (run re/tools/emu_mask.py)");
        return;
    }
    let body_mask = load_mask("body");
    let obstacle = load_mask("obstacle");
    check("bgchain", |v| {
        let kinds = [v[0] as i16, v[1] as i16, v[2] as i16, v[3] as i16];
        let tile = (v[5] as i16, v[6] as i16);
        let (x, y) = (i32::from(tile.1) >> 5, i32::from(tile.0) >> 5);
        let fg = |a: i32, b: i32| match (a - x, b - y) {
            (0, -1) => kinds[0],
            (1, 0) => kinds[1],
            (-1, 0) => kinds[2],
            (0, 1) => kinds[3],
            _ => panic!("unexpected tile lookup"),
        };
        let mut b = Body {
            pos: (v[7] as i16, v[8] as i16),
            at: (v[7] as i16, v[8] as i16),
            fix: (0x7777, 0x7777),
            speed: Speed { h: v[9] as i16, v: v[10] as i16 },
        };
        let r = bg_tile_bounce(&mut b, tile, 32, 150, [22, 22, 42, 42], v[4] != 0, &mut |body| {
            mask_hit(&mut body.pos, &body_mask, tile, &obstacle)
        }, &fg);
        vec![r, b.at.0.into(), b.at.1.into(), b.pos.0.into(), b.pos.1.into(), b.speed.h.into(), b.speed.v.into(), b.fix.0, b.fix.1]
    });
}
