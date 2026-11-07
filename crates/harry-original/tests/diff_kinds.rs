//! Simple sprite kinds (setup, frames of the task, the hit task) against
//! the original, recorded in Unicorn by `re/tools/emu_kinds.py`.

use harry_core::kinds::{
    LevelCounter, handle_bonus, handle_door, handle_item, hit_door, hit_exit, setup_bonus, setup_door, setup_exit,
    setup_item,
};
mod common;

use common::{Tally, dump, installed, recording, word_diff};
use harry_core::sprite::{Sprite, Task};
use harry_original::hlvl;

#[test]
fn simple_kinds_match_the_original() {
    let Some(res) = installed("Episode 1.bin") else { return };
    let level1 = hlvl::read(res.get(b"Hlvl", 1).unwrap()).unwrap();
    for kind in ["bonus", "item", "exit", "door"] {
        let Some(text) = recording(&format!("kinds/{kind}"), "emu_kinds.py") else { return };
        let mut tally = Tally::new(kind, 5);
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
            tally.case(got == want && globals_ok, || {
                format!("{a}\n  {} | globals want {want_globals} got {} {}", word_diff(want, &got), counter.current, counter.previous)
            });
        }
        tally.finish();
    }
}
