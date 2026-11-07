//! The plant's hit task against the original, recorded in Unicorn by
//! `re/tools/emu_planthit.py`.

mod common;

use common::{Tally, dump, installed, named_cases, recording};
use harry_core::Physics;
use harry_core::effect::setup_effect;
use harry_core::furniture::{setup_barrel, setup_chair};
use harry_core::ramp::setup_ramp;
use harry_core::stairs::setup_stairs;
use harry_core::button::setup_sliding_door;
use harry_core::coworker::setup_coworker;
use harry_core::mailguy::setup_mail_guy;
use harry_core::robot::setup_robot;
use harry_core::grate::setup_grate;
use harry_core::kinds::{PlantHitMasks, hit_plant, setup_cooler, setup_door, setup_plant};
use harry_core::player::{HOT_RECT, KICK_HOT_RECT};
use harry_core::random::Random;
use harry_core::sprite::{Sprite, Task};
use harry_original::masks;

#[test]
fn plant_hits_match_the_original() {
    let Some(text) = recording("planthit", "emu_planthit.py") else { return };
    let Some(gfx) = installed("Harry Graphics.bin") else { return };
    let body = masks::from_cicn(&gfx, 299).unwrap();
    let plant = masks::from_cicn(&gfx, 599).unwrap();
    let barrel = masks::from_cicn(&gfx, 598).unwrap();
    let ms = PlantHitMasks { body: &body, plant: &plant, barrel: &barrel };
    let physics = Physics::default();
    let mut tally = Tally::new("planthit", 8);
    for (a, i, b) in named_cases(&text) {
        let w = |k: &str| i.i64(k);
        let mut sp = Sprite::new(601, w("h") as i16, w("v") as i16);
        setup_plant(&mut sp);
        let mut rng = Random::new(w("seed0") as u32);
        let kind = w("other");
        let okind = match kind {
            2 => 601,
            3 => 500,
            8 => 550,
            9 => 560,
            _ => w("okind") as i16,
        };
        let blast = w("blast") as i16;
        let mut o = Sprite::new(okind, w("oh") as i16, w("ov") as i16);
        match kind {
            2 => setup_plant(&mut o),
            3 => setup_cooler(&mut o),
            4 => setup_door(&mut o),
            8 => setup_barrel(&mut o),
            9 => setup_chair(&mut o, &mut rng),
            10 => setup_ramp(&mut o),
            12 => setup_stairs(&mut o),
            13 => setup_sliding_door(&mut o),
            14 => setup_coworker(&mut o),
            15 => setup_mail_guy(&mut o),
            16 => setup_robot(&mut o, (0, 0)),
            17 => setup_grate(&mut o),
            18 => harry_core::desk::setup_desk(&mut o),
            7 => {
                setup_effect(&mut o);
                o.kind = okind;
            }
            _ => {
                o.task = Some(match kind {
                    1 => Task::Player,
                    5 => Task::PlayerShot,
                    11 => Task::Collider,
                    _ => Task::EnemyShot,
                });
                o.at = (w("ov") as i16, w("oh") as i16);
                if kind == 1 {
                    o.hot_rect = if w("kicking") != 0 { KICK_HOT_RECT } else { HOT_RECT };
                }
            }
        }
        for (s, pre) in [(&mut sp, ""), (&mut o, "o")] {
            let k = |n: &str| i.i64(&format!("{pre}{n}"));
            s.pos = (k("pv") as i16, k("ph") as i16);
            s.fix = (k("fixh") as i32, k("fixv") as i32);
            s.speed.h = k("speedh") as i16;
            s.speed.v = k("speedv") as i16;
            s.air = k("air") as i16;
            s.frozen = k("frozen") != 0;
            s.level_index = k("li") as i16;
            s.at = (k("atv") as i16, k("ath") as i16);
            let hot: Vec<i16> = i.str(&format!("{pre}hot")).split(':').map(|x| x.parse().unwrap()).collect();
            s.hot_rect = [hot[0], hot[1], hot[2], hot[3]];
        }
        o.dir = w("odir") as i16;
        sp.blast_cooldown = blast;
        sp.ramp_cooldown = w("f94") as i16;
        let mut contact = w("contact") as i16;
        let mut rng = Random::new(w("seed") as u32);
        let mut events = Vec::new();
        hit_plant(&mut sp, &mut o, &ms, &physics, &mut rng, &mut contact, w("busy") != 0, &mut events);
        let got = format!(
            "{} || {} | contact={contact} events={}",
            dump(&sp),
            dump(&o),
            common::events(&events)
        );
        tally.words(a, b, &got);
    }
    tally.finish();
}
