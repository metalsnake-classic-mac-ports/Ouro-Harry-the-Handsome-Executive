//! The dart enemy against the original, recorded in Unicorn by
//! `re/tools/emu_dart.py`.

mod common;

use common::{check_named, dump, installed};
use harry_core::dart_enemy::{handle_dart_enemy, hit_dart_enemy, setup_dart_enemy};
use harry_core::frame::PlayerGlobals;
use harry_core::random::Random;
use harry_core::sprite::{Sprite, Task};
use harry_original::hlvl;

#[test]
fn dart_enemy_matches_the_original() {
    let Some(ep) = installed("Episode 1.bin") else { return };
    let level1 = hlvl::read(ep.get(b"Hlvl", 1).unwrap()).unwrap();
    check_named("dart", "emu_dart.py", |i| {
        let w = |k: &str| i.i32(k);
        let mut level = level1.clone();
        let li = w("levelindex") as i16;
        if li >= 0 {
            level.sprites[li as usize].info[0] = w("info0") as i16;
        }
        let mut g = PlayerGlobals { favor: w("favor") as i16, ..Default::default() };
        let mut rng = Random::new(w("seed") as u32);
        let mut sp = Sprite::new(w("kind") as i16, w("h") as i16, w("v") as i16);
        setup_dart_enemy(&mut sp);
        sp.level_index = li;
        if w("settimer") != -9999 {
            sp.timer = w("settimer") as i16;
        }
        if w("setf6e") != -9999 {
            sp.f6e = w("setf6e");
        }
        if w("setlayer") != -9999 {
            sp.layer = w("setlayer") as i16;
        }
        let mut other = Sprite::new(0, 0, 0);
        other.task = Some(if i.str("other") == "bonus" { Task::Bonus } else { Task::PlayerShot });
        other.frozen = i.str("other") == "frozenshot";
        let view = (w("viewv") as i16, w("viewh") as i16);
        let mut events = Vec::new();
        let frames = w("frames");
        for f in 0..=frames {
            if f == w("frozenframe") {
                sp.frozen = true;
            }
            if f == w("hitframe") {
                hit_dart_enemy(&mut sp, &mut other, &mut g, &mut rng, &mut events);
            }
            if f < frames {
                handle_dart_enemy(&mut sp, view, &level, &mut rng, &mut events);
            }
        }
        format!("{} | favor={} ofrozen={} events={}", dump(&sp), g.favor, u8::from(other.frozen), common::events(&events))
    });
}
