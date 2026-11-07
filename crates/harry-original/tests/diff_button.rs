//! Buttons and sliding doors against the original, recorded in Unicorn by
//! `re/tools/emu_button.py`.

mod common;

use common::{Fields, check_named, dump, installed};
use harry_core::button::{handle_button, handle_sliding_door, hit_button, setup_button, setup_sliding_door};
use harry_core::sprite::{Sprite, Task};
use harry_original::hlvl;

fn level() -> Option<harry_core::Level> {
    let ep = installed("Episode 1.bin")?;
    Some(hlvl::read(ep.get(b"Hlvl", 2).unwrap()).unwrap())
}

fn check(name: &str, run: impl FnMut(&Fields) -> String) {
    check_named(name, "emu_button.py", run);
}

#[test]
fn buttons_match_the_original() {
    let Some(base) = level() else { return };
    check("button", |i| {
        let w = |k: &str| i.i16(k);
        let mut level = base.clone();
        let index = w("index");
        level.sprites[index as usize].info[0] = w("info0");
        level.sprites[index as usize].info[1] = w("info1");
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_button(&mut sp);
        sp.level_index = index;
        sp.frozen = w("frozen") != 0;
        let mut evs = Vec::new();
        for hit in i.str("hits").split(',') {
            let mut events = Vec::new();
            handle_button(&mut sp, &mut level, &mut events);
            if hit != "-" {
                let p: Vec<&str> = hit.split(':').collect();
                let n = |k: usize| p[k].parse::<i16>().unwrap();
                let mut o = Sprite::new(0, 0, 0);
                o.task = Some(match p[0] {
                    "harry" | "kick" => Task::Player,
                    "plant" => Task::Plant,
                    "barrel" => Task::Barrel,
                    "chair" => Task::Chair,
                    "mailguy" => Task::MailGuy,
                    "robot" => Task::Robot,
                    "reversor" => Task::Reversor,
                    "playershot" => Task::PlayerShot,
                    "enemyshot" => Task::EnemyShot,
                    _ => Task::Bonus,
                });
                o.at = (n(1), n(2));
                o.dir = n(3);
                o.frozen = n(4) != 0;
                o.hot_rect = if p[0] == "kick" { [0, 0, 64, 64] } else { [22, 22, 42, 42] };
                hit_button(&mut sp, &o, &mut events);
            }
            evs.push(common::events(&events));
        }
        format!("{} | info1={} events={}", dump(&sp), level.sprites[index as usize].info[1], evs.join(";"))
    });
}

#[test]
fn sliding_doors_match_the_original() {
    let Some(base) = level() else { return };
    check("sliding", |i| {
        let w = |k: &str| i.i16(k);
        let mut level = base.clone();
        let (index, link) = (w("index"), w("link"));
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_sliding_door(&mut sp);
        sp.level_index = index;
        sp.dir = w("dir");
        sp.frozen = w("frozen") != 0;
        let valid = (0..256).contains(&index);
        if valid {
            level.sprites[index as usize].info[0] = link;
        }
        let mut evs = Vec::new();
        for s in i.str("states").split(',') {
            if valid {
                level.sprites[link.unsigned_abs() as usize].info[1] = s.parse().unwrap();
            }
            let mut events = Vec::new();
            handle_sliding_door(&mut sp, &level, &mut events);
            evs.push(common::events(&events));
        }
        format!("{} | events={}", dump(&sp), evs.join(";"))
    });
}
