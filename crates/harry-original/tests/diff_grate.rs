//! Grates, fences and ghosts against the original, recorded in Unicorn by
//! `re/tools/emu_grate.py`.

mod common;

use common::{Fields, check_named, dump, installed};
use harry_core::ghost::{handle_ghost, hit_ghost, setup_ghost};
use harry_core::grate::{grate_bounce, handle_grate, hit_grate, setup_grate};
use harry_core::random::Random;
use harry_core::sprite::{Sprite, SpriteEvent, Task};
use harry_original::hlvl;

fn check(name: &str, run: impl FnMut(&Fields) -> String) {
    check_named(name, "emu_grate.py", run);
}

/// The recorder's events: explosions carry the timer of their record
/// (0 unless the routine sets it); points and cleared entries are given
/// as totals.
fn log(events: &[SpriteEvent], score: &mut i32, forget: &mut Vec<i16>) -> String {
    let mut out = Vec::new();
    for e in events {
        match *e {
            SpriteEvent::Score(n) => *score += n,
            SpriteEvent::Forget(i) => forget.push(i),
            SpriteEvent::Explosion { h, v, a, b } => out.push(format!("x:{h}:{v}:{a}:{b}:t0")),
            ref e => out.push(common::event(e)),
        }
    }
    if out.is_empty() { "-".into() } else { out.join(",") }
}

#[test]
fn grates_match_the_original() {
    check("grate", |i| {
        let w = |k: &str| i.i16(k);
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_grate(&mut sp);
        if w("hp") != -1 {
            sp.state = w("hp");
        }
        sp.frozen = w("frozen") != 0;
        let mut rng = Random::new(i.u32("seed"));
        let (mut score, mut forget) = (0, Vec::new());
        let mut evs = Vec::new();
        for h in i.str("hits").split(',') {
            let mut events = Vec::new();
            handle_grate(&mut sp, &mut events);
            if h != "-" {
                let mut o = Sprite::new(0, 0, 0);
                o.task = Some(if h.starts_with('e') { Task::Effect } else { Task::Player });
                if let Some(t) = h.strip_prefix('e') {
                    o.timer = t.parse().unwrap();
                }
                o.frozen = h == "pf";
                hit_grate(&mut sp, &o, &mut rng);
            }
            evs.push(log(&events, &mut score, &mut forget));
        }
        format!("{} | score={score} events={}", dump(&sp), evs.join(";"))
    });
}

#[test]
fn grate_bounces_match_the_original() {
    check("gratebounce", |i| {
        let w = |k: &str| i.i64(k);
        let mut g = Sprite::new(w("kind") as i16, w("gh") as i16, w("gv") as i16);
        setup_grate(&mut g);
        let hot: Vec<i16> = i.str("hot").split(':').map(|x| x.parse().unwrap()).collect();
        let mut sp = Sprite::new(0, 0, 0);
        sp.at = (w("v") as i16, w("h") as i16);
        sp.speed.v = w("speedv") as i16;
        sp.speed.h = w("speedh") as i16;
        sp.fix = (w("fixh") as i32, w("fixv") as i32);
        grate_bounce(&mut sp, &g, w("half") as i16, w("bounce") as i32, [hot[0], hot[1], hot[2], hot[3]]);
        format!(
            "{} | v={} h={} speedv={} speedh={} fixh={} fixv={}",
            dump(&g),
            sp.at.0,
            sp.at.1,
            sp.speed.v,
            sp.speed.h,
            sp.fix.0,
            sp.fix.1
        )
    });
}

#[test]
fn ghosts_match_the_original() {
    let Some(ep) = installed("Episode 1.bin") else { return };
    let level = hlvl::read(ep.get(b"Hlvl", 6).unwrap()).unwrap();
    check("ghost", |i| {
        let w = |k: &str| i.i16(k);
        let mut player = (w("ph"), w("pv"));
        let mut view = (w("vv"), w("vh"));
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_ghost(&mut sp, player);
        sp.level_index = w("index");
        if w("hp") != -1 {
            sp.state = w("hp");
        }
        if w("mode") != -1 {
            sp.f6e = i32::from(w("mode"));
        }
        sp.frozen = w("frozen") != 0;
        let mut rng = Random::new(i.u32("seed"));
        let (mut score, mut forget) = (0, Vec::new());
        let mut evs = Vec::new();
        for step in i.str("steps").split(',') {
            let p: Vec<&str> = step.split(':').collect();
            let (dh, dv): (i16, i16) = (p[0].parse().unwrap(), p[1].parse().unwrap());
            player = (player.0.wrapping_add(dh), player.1.wrapping_add(dv));
            view = (view.0.wrapping_add(dv), view.1.wrapping_add(dh));
            let mut events = Vec::new();
            handle_ghost(&mut sp, view, player, &mut rng, &mut events);
            let mut shot = None;
            if p[2] != "-" {
                let mut o = Sprite::new(0, 0, 0);
                o.task = Some(Task::PlayerShot);
                o.frozen = p[2] == "sf";
                hit_ghost(&mut sp, &mut o, &mut events);
                shot = Some(u8::from(o.frozen));
            }
            let mut l = log(&events, &mut score, &mut forget);
            if let Some(s) = shot {
                l = if l == "-" { format!("shot:{s}") } else { format!("{l},shot:{s}") };
            }
            evs.push(l);
        }
        let index = w("index");
        let entry = if index >= 0 { i64::from(level.sprites[index as usize].present && !forget.contains(&index)) } else { -1 };
        format!("{} | entry={entry} score={score} events={}", dump(&sp), evs.join(";"))
    });
}
