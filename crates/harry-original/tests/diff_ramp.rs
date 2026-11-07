//! Ramps against the original, recorded in Unicorn by
//! `re/tools/emu_ramp.py`.

mod common;

use common::{Fields, check_named, dump, installed, levels_of};
use harry_core::Physics;
use harry_core::bounce::Body;
use harry_core::physics::Speed;
use harry_core::ramp::{RampMasks, Rider, handle_ramp, handle_ramp_collision, hit_ramp, hit_ramp_tile, setup_ramp};
use harry_core::sprite::{Sprite, Task};
use harry_core::tiles::touched_tiles;
use harry_original::{hlvl, masks};

fn compare(name: &str, run: impl FnMut(&Fields) -> String) {
    check_named(name, "emu_ramp.py", run);
}

#[test]
fn ramp_collisions_match_the_original() {
    compare("rampcollide", |i| {
        let w = |k: &str| i.i32(k);
        let mut ramp = Sprite::new(w("kind") as i16, w("rh") as i16, w("rv") as i16);
        setup_ramp(&mut ramp);
        ramp.fix = (w("rfixh"), w("rfixv"));
        ramp.speed = Speed { v: w("rspeedv") as i16, h: w("rspeedh") as i16 };
        let hot: Vec<i16> = i.str("hot").split(':').map(|x| x.parse().unwrap()).collect();
        let mut body = Body {
            pos: (w("spv") as i16, w("sph") as i16),
            at: (0, 0),
            fix: (w("fixh"), w("fixv")),
            speed: Speed { v: w("speedv") as i16, h: w("speedh") as i16 },
        };
        let (mut air, mut cooldown) = (w("air") as i16, w("f94") as i16);
        let mut r = Rider { body: &mut body, air: &mut air, cooldown: &mut cooldown, player: w("player") != 0 };
        let sound = handle_ramp_collision(&mut r, w("half") as i16, [hot[0], hot[1], hot[2], hot[3]], &mut ramp, w("bounce") as i16);
        format!(
            "{} | pv={} ph={} fixh={} fixv={} speedv={} speedh={} air={air} f94={cooldown} sound={}",
            dump(&ramp),
            body.pos.0,
            body.pos.1,
            body.fix.0,
            body.fix.1,
            body.speed.v,
            body.speed.h,
            u8::from(sound)
        )
    });
}

#[test]
fn ramp_task_matches_the_original() {
    let Some(ep) = installed("Episode 1.bin") else { return };
    let level1 = hlvl::read(ep.get(b"Hlvl", 1).unwrap()).unwrap();
    let physics = Physics::default();
    compare("ramptask", |i| {
        let w = |k: &str| i.i16(k);
        let mut level = level1.clone();
        let li = w("levelindex");
        if li >= 0 {
            level.sprites[li as usize].info[1] = w("info1");
        }
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_ramp(&mut sp);
        sp.level_index = li;
        sp.speed = Speed { v: w("speedv"), h: w("speedh") };
        sp.air = w("air");
        sp.frozen = w("frozen") != 0;
        for _ in 0..w("frames") {
            handle_ramp(&mut sp, &level, &physics);
        }
        dump(&sp)
    });
}

#[test]
fn ramps_meeting_match_the_original() {
    compare("ramphit", |i| {
        let w = |k: &str| i.i32(k);
        let make = |kind: i32, v: i32, h: i32| {
            let mut s = Sprite::new(kind as i16, h as i16, v as i16);
            if kind != 0 {
                setup_ramp(&mut s);
            } else {
                s.task = Some(Task::Bonus);
                s.at = (v as i16, h as i16);
            }
            s
        };
        let mut sp = make(w("kind"), w("v"), w("h"));
        let mut o = make(w("okind"), w("ov"), w("oh"));
        sp.fix = (w("fixh"), w("fixv"));
        sp.speed = Speed { v: w("speedv") as i16, h: w("speedh") as i16 };
        sp.frozen = w("frozen") != 0;
        o.frozen = w("ofrozen") != 0;
        hit_ramp(&mut sp, &o, w("bounce") as i16);
        format!("{} || {}", dump(&sp), dump(&o))
    });
}

#[test]
fn pushed_ramps_move_like_the_original() {
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let levels = levels_of(&ep);
    let ramps: Vec<_> = (571..=574).map(|id| masks::from_cicn(&gfx, id).unwrap()).collect();
    let obstacle = masks::from_cicn(&gfx, 290).unwrap();
    let walls = masks::walls(&app).unwrap();
    let ms = RampMasks { ramps: [&ramps[0], &ramps[1], &ramps[2], &ramps[3]], walls: &walls, obstacle: &obstacle };
    let physics = Physics::default();
    compare("rampflight", |i| {
        let w = |k: &str| i.i16(k);
        let mut level = levels[w("level") as usize - 1].clone();
        let li = w("levelindex");
        level.sprites[li as usize].info[1] = 1;
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_ramp(&mut sp);
        sp.level_index = li;
        sp.speed = Speed { v: w("speedv"), h: w("speedh") };
        for _ in 0..w("frames") {
            handle_ramp(&mut sp, &level, &physics);
            let mut hits = Vec::new();
            touched_tiles(&level, sp.at, sp.hot_rect, [0, 0, 64, 64], |h| hits.push(h));
            for h in hits {
                hit_ramp_tile(&mut sp, h, &level, &ms, physics.bounce);
            }
        }
        dump(&sp)
    });
}
