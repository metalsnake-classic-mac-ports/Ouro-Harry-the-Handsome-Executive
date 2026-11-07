//! The level start, the idle sprites and `EraseSprites` against the
//! original, recorded in Unicorn by `re/tools/emu_world.py`.

mod common;

use common::{Tally, dump, hit_name, installed, named_cases, recording, task_name, tile_name};
use harry_core::random::Random;
use harry_core::world::World;
use harry_original::hlvl;

/// The object ids whose kinds the port has (as the recorder's filter).
fn ported(k: i16) -> bool {
    matches!(k, 500 | 501 | 601 | 700..=799 | 900..=949 | 1000..=1049 | 1609 | 1619 | 1629 | 1639 | 2301..=2304
        | 3000 | 3100 | 3101 | 9199 | 9200 | 9300 | 9600 | 9800 | 9401..=9499 | 9700..=9799 | 9990..=9999
        | 11001..=19999 | 21000.. | 550 | 560 | 1601 | 1611 | 1621 | 1631 | 950..=999 | 1300..=1449 | 1800..=1849
        | 2000..=2099 | 4200..=4299 | 9500..=9599 | 9900..=9909 | 3900..=3999 | 5000..=5999 | 1850..=1999 | 6000..=6099 | 1100..=1299 | 1700..=1799 | 2200..=2299)
}

/// A blob set up facing 24 or more takes its face from past its table in
/// the original (BUGS.md), and its idle slot keeps it; the port has none.
fn blob_faces(b: &str) -> String {
    let blob = |k: &str| k.parse::<i16>().is_ok_and(|k| (1700..1800).contains(&k));
    b.split(' ')
        .map(|part| {
            if let Some(idle) = part.strip_prefix("idle=") {
                let slots: Vec<String> = idle
                    .split(';')
                    .map(|slot| {
                        let mut f: Vec<String> = slot.split(':').map(str::to_string).collect();
                        if f.len() > 5 && blob(&f[4]) && f[5].parse::<i16>().is_ok_and(|d| d >= 24) {
                            f[2] = "0".into();
                        }
                        f.join(":")
                    })
                    .collect();
                format!("idle={}", slots.join(";"))
            } else if let Some(list) = part.strip_prefix("list=") {
                let recs: Vec<String> = list
                    .split(';')
                    .map(|r| {
                        let kind = r.split(':').next().and_then(|k| k.split("kind=").nth(1)).unwrap_or("");
                        let dir = r.split(':').find_map(|f| f.strip_prefix("dir=")).and_then(|d| d.parse::<i16>().ok());
                        if blob(kind) && dir.is_some_and(|d| d >= 24) {
                            r.split(':').map(|f| if f.starts_with("face=") { "face=0" } else if f.starts_with("f86=") { "f86=0" } else { f }).collect::<Vec<_>>().join(":")
                        } else {
                            r.to_string()
                        }
                    })
                    .collect();
                format!("list={}", recs.join(";"))
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn state(w: &World, before: &harry_core::Level, level: &harry_core::Level) -> String {
    let list: Vec<String> = w.sprites.list.iter().map(|&i| format!("{i}/{}", dump(w.rec(i)).replace(' ', ":"))).collect();
    let free: Vec<String> = w.sprites.free.iter().rev().map(|i| i.to_string()).collect();
    let idle: Vec<String> = w
        .idle
        .iter()
        .enumerate()
        .filter(|(_, s)| s.used)
        .map(|(n, s)| {
            let act = s.active.map_or(-1, |i| i as i64);
            format!(
                "{n}:{act}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                s.face,
                u8::from(s.b8a),
                s.kind,
                s.dir,
                s.layer,
                s.at.0,
                s.at.1,
                s.level_index,
                s.state,
                s.timer,
                s.f7c,
                s.hot_rect[0],
                s.hot_rect[1],
                s.hot_rect[2],
                s.hot_rect[3],
                s.speed.v,
                s.speed.h,
                s.f6e,
                u8::from(s.b8b),
                u8::from(s.b8c),
                s.scale[0],
                s.scale[1],
                s.air,
                task_name(s.task),
                hit_name(s.hit_task),
                tile_name(s.hit_tile_task)
            )
        })
        .collect();
    let lev: Vec<String> = (0..256)
        .filter(|&i| level.sprites[i] != before.sprites[i])
        .map(|i| {
            let e = &level.sprites[i];
            format!("{i}:{}:{}:{}:{}", u8::from(e.present), e.id, e.v, e.h)
        })
        .collect();
    let props = |p: &harry_core::props::Props| {
        let v: Vec<String> = p.slots.iter().map(|x| format!("{}:{}:{}", x.at.0, x.at.1, x.face)).collect();
        if v.is_empty() { "-".to_string() } else { v.join(",") }
    };
    let or = |v: Vec<String>, sep: &str| if v.is_empty() { "-".to_string() } else { v.join(sep) };
    format!(
        "list={} free={} idle={} level={} props={}|{}",
        or(list, ";"),
        or(free, ","),
        or(idle, ";"),
        or(lev, ";"),
        props(&w.bg_props),
        props(&w.fg_props)
    )
}

#[test]
fn level_start_and_idle_sprites_match_the_original() {
    let Some(text) = recording("world", "emu_world.py") else { return };
    let Some(ep) = installed("Episode 1.bin") else { return };
    let levels: Vec<_> = (1..=15)
        .map(|n| {
            let mut l = hlvl::read(ep.get(b"Hlvl", n).unwrap()).unwrap();
            for e in l.sprites.iter_mut() {
                if e.present && !ported(e.id) {
                    e.present = false;
                }
            }
            l
        })
        .collect();
    let mut tally = Tally::new("world", 8);
    for (a, i, b) in named_cases(&text) {
        let mut level = levels[i.str("level").parse::<usize>().unwrap() - 1].clone();
        let mut rng = Random::new(i.u32("seed"));
        let mut w = World::default();
        let missing = w.setup_level_sprites(&level, &mut rng);
        assert!(missing.is_empty(), "{missing:?}");
        let before = level.clone();
        if i.str("steps") != "-" {
            for step in i.str("steps").split(',') {
                let p: Vec<&str> = step.splitn(3, ':').collect();
                w.handle_idle_sprites((p[0].parse().unwrap(), p[1].parse().unwrap()));
                if p[2] != "-" {
                    let (pos, mode) = p[2].split_once(':').unwrap();
                    if let Some(&r) = w.sprites.list.get(pos.parse::<usize>().unwrap()) {
                        if mode == "f" {
                            w.sprites.recs[r].frozen = true;
                        } else {
                            w.sprites.recs[r].task = None;
                        }
                    }
                }
                w.erase_sprites(&mut level);
            }
        }
        let got = state(&w, &before, &level);
        let b = blob_faces(b);
        let b = b.as_str();
        tally.words(a, b, &got);
    }
    tally.finish();
}
