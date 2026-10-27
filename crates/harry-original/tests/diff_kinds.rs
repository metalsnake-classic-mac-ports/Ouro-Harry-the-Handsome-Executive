//! Simple sprite kinds (setup, frames of the task, the hit task) against
//! the original, recorded in Unicorn by `re/tools/emu_kinds.py`.

use std::path::PathBuf;

use harry_core::kinds::{
    LevelCounter, handle_bonus, handle_door, handle_item, hit_door, hit_exit, setup_bonus, setup_door, setup_exit,
    setup_item,
};
use harry_core::sprite::{HitTask, Sprite, Task};
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn task_name(t: Option<Task>) -> &'static str {
    match t {
        None => "-",
        Some(Task::Player) => "HandlePlayerSprite",
        Some(Task::HeldWeapon) => "HandleHeldWeaponSprite",
        Some(Task::PlayerShot) => "HandlePlayerShotSprite",
        Some(Task::Bonus) => "HandleBonusSprite",
        Some(Task::Item) => "HandleItemSprite",
        Some(Task::Exit) => "HandleExitSprite",
        Some(Task::Door) => "HandleDoorSprite",
        Some(Task::EnemyShot) => "HandleEnemyShotSprite",
    }
}

fn hit_name(t: Option<HitTask>) -> &'static str {
    match t {
        None => "-",
        Some(HitTask::Player) => "HitPlayerSprite",
        Some(HitTask::PlayerShot) => "HitPlayerShotSprite",
        Some(HitTask::Bonus) => "HitBonusSprite",
        Some(HitTask::Item) => "HitItemSprite",
        Some(HitTask::Exit) => "HitExitSprite",
        Some(HitTask::Door) => "HitDoorSprite",
    }
}

/// The recorder's dump (`re/tools/sprites.py`).
pub fn dump(s: &Sprite) -> String {
    let b = |x: bool| i32::from(x);
    let fields: Vec<(&str, i64)> = vec![
        ("kind", s.kind.into()),
        ("posV", s.pos.0.into()),
        ("posH", s.pos.1.into()),
        ("hotT", s.hot_rect[0].into()),
        ("hotL", s.hot_rect[1].into()),
        ("hotB", s.hot_rect[2].into()),
        ("hotR", s.hot_rect[3].into()),
        ("layer", s.layer.into()),
        ("speedV", s.speed.v.into()),
        ("speedH", s.speed.h.into()),
        ("scale0", s.scale[0].into()),
        ("scale1", s.scale[1].into()),
        ("scale2", s.scale[2].into()),
        ("scale3", s.scale[3].into()),
        ("atV", s.at.0.into()),
        ("atH", s.at.1.into()),
        ("fixH", s.fix.0.into()),
        ("fixV", s.fix.1.into()),
        ("dir", s.dir.into()),
        ("levelIndex", s.level_index.into()),
        ("f6e", s.f6e.into()),
        ("state", s.state.into()),
        ("timer", s.timer.into()),
        ("idleIndex", s.idle_index.into()),
        ("f78", s.f78.into()),
        ("f7c", s.f7c.into()),
        ("f82", s.f82.into()),
        ("f84", s.f84.into()),
        ("f86", s.f86.into()),
        ("b8a", b(s.b8a).into()),
        ("b8b", b(s.b8b).into()),
        ("b8c", b(s.b8c).into()),
        ("air", s.air.into()),
        ("flip", s.flip.into()),
        ("frozen", b(s.frozen).into()),
        ("f94", s.f94.into()),
        ("blast", s.blast_cooldown.into()),
        ("lastTileH", s.last_tile.0.into()),
        ("lastTileV", s.last_tile.1.into()),
        ("slide", s.slide.into()),
        ("face", s.face.into()),
    ];
    let mut out: Vec<String> = fields.iter().map(|(n, v)| format!("{n}={v}")).collect();
    out.push(format!("task={}", task_name(s.task)));
    out.push(format!("hit={}", hit_name(s.hit_task)));
    out.push(format!("tile={}", if s.hit_tile_task.is_some() { "?" } else { "-" }));
    out.join(" ")
}

#[test]
fn simple_kinds_match_the_original() {
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Episode 1.bin")) else { return };
    let res = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let level1 = hlvl::read(res.get(b"Hlvl", 1).unwrap()).unwrap();
    for kind in ["bonus", "item", "exit", "door"] {
        let Ok(text) = std::fs::read_to_string(root().join(format!("re/work/diff/kinds/{kind}.txt"))) else {
            eprintln!("skipped: kinds/{kind}.txt not found (run re/tools/emu_kinds.py)");
            return;
        };
        let mut bad = 0;
        let mut n = 0;
        for line in text.lines().skip(1) {
            let (a, b) = line.split_once(" -> ").unwrap();
            let (want, want_globals) = b.split_once(" | ").unwrap();
            let f: Vec<&str> = a.split(' ').collect();
            let num = |k: usize| f[k].parse::<i64>().unwrap();
            let (frames, frozen_at, other) = (num(3), num(4), num(5));
            let mut sp = Sprite::new(num(0) as i16, num(1) as i16, num(2) as i16);
            let mut level = level1.clone();
            let mut counter = LevelCounter::default();
            let mut door_index = -1;
            match kind {
                "bonus" => setup_bonus(&mut sp),
                "item" => setup_item(&mut sp),
                "door" => {
                    setup_door(&mut sp);
                    let e: Vec<i16> = f[6].split(':').map(|x| x.parse().unwrap()).collect();
                    door_index = e[0];
                    sp.level_index = e[0];
                    sp.dir = e[1];
                    if e[2] != 0 {
                        sp.hit_task = None;
                    }
                    if e[0] >= 0 {
                        level.sprites[e[0] as usize].id = 0x4444;
                    }
                }
                _ => {
                    setup_exit(&mut sp);
                    let e: Vec<i16> = f[6].split(':').map(|x| x.parse().unwrap()).collect();
                    sp.level_index = e[0];
                    level.sprites[e[0] as usize].info[0] = e[1];
                    counter = LevelCounter { current: e[2], previous: 0x5555 };
                    sp.timer = e[3];
                }
            }
            for fr in 0..frames {
                if fr == frozen_at {
                    sp.frozen = true;
                }
                match kind {
                    "bonus" => handle_bonus(&mut sp),
                    "item" => handle_item(&mut sp),
                    "door" => handle_door(&mut sp, &mut level),
                    _ => {}
                }
            }
            if frozen_at == frames {
                sp.frozen = true;
            }
            let mut o = Sprite::new(0, 0, 0);
            o.task = Some(match other {
                2 => Task::HeldWeapon,
                4 => Task::PlayerShot,
                5 => Task::EnemyShot,
                _ => Task::Player,
            });
            o.frozen = other == 3;
            if other != 0 {
                match kind {
                    "exit" => hit_exit(&mut sp, &o, &level, &mut counter),
                    "door" => hit_door(&sp, &mut o),
                    _ => {}
                }
            }
            let got = dump(&sp);
            let globals_ok = match kind {
                "exit" => want_globals == format!("{} {}", counter.current, counter.previous),
                "door" => {
                    let rec = if door_index >= 0 { level.sprites[door_index as usize].id } else { 0 };
                    want_globals == format!("{rec} {}", if other != 0 { u8::from(o.frozen) } else { 0 })
                }
                _ => true,
            };
            n += 1;
            if got != want || !globals_ok {
                bad += 1;
                if bad <= 5 {
                    let diffs: Vec<String> = want
                        .split(' ')
                        .zip(got.split(' '))
                        .filter(|(w, g)| w != g)
                        .map(|(w, g)| format!("want {w} got {g}"))
                        .collect();
                    eprintln!("{kind}: {a}\n  {} | globals want {want_globals} got {} {}", diffs.join(", "), counter.current, counter.previous);
                }
            }
        }
        assert_eq!(bad, 0, "{kind}: {bad} of {n} cases differ");
        eprintln!("{kind}: {n} cases");
    }
}
