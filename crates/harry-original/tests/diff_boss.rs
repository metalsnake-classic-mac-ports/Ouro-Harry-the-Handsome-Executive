//! The boss against the original, recorded in Unicorn by
//! `re/tools/emu_boss.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::boss::{BossState, handle_boss, handle_boss_death, hit_boss, setup_boss};
use harry_core::random::Random;
use harry_core::sprite::{Sprite, SpriteEvent, Task};
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The recorder's form of an event (see `emu_boss.py`).
fn event(e: &SpriteEvent) -> Option<String> {
    Some(match *e {
        SpriteEvent::Shot { kind, h, v, speed, dir, fix, timer, f7c } => {
            let (fh, fv) = fix.unwrap_or((i32::from(h) << 8, i32::from(v) << 8));
            let mut s = format!("shot:{kind}:{h}:{v}:{}:{}:{dir}:{fh}:{fv}", speed.v, speed.h);
            if kind == 1211 {
                s += &format!(":t{}:f{}", timer.unwrap_or(0), f7c.unwrap_or(0));
            }
            s
        }
        SpriteEvent::Score(_) | SpriteEvent::GhostsDie => return None,
        ref e => common::event(e),
    })
}

#[test]
fn the_boss_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/boss.txt")) else {
        eprintln!("skipped: boss.txt not found (run re/tools/emu_boss.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Episode 1.bin")) else { return };
    let ep = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let level = hlvl::read(ep.get(b"Hlvl", 15).unwrap()).unwrap();
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap() as i16;
        let mut rng = Random::new(i["seed"].parse().unwrap());
        let mut boss = BossState { growing: w("growing") != 0, anim: w("anim"), blast_cooldown: w("blast"), ..Default::default() };
        let mut sp = Sprite::new(2201, w("h"), w("v"));
        setup_boss(&mut sp, &mut rng, &mut boss);
        sp.level_index = w("index");
        if w("count") != -1 {
            boss.count = w("count");
        }
        if w("spin") != -1 {
            boss.spin = w("spin");
        }
        if w("cooldown") != -1 {
            boss.cooldown = w("cooldown");
        }
        if w("change") != -1 {
            boss.change = w("change");
        }
        if w("phase") != -1 {
            boss.phase = w("phase");
        }
        if w("hp") != -1 {
            sp.state = w("hp");
        }
        sp.scale[0] = w("scale");
        sp.scale[1] = w("scale");
        let mut player = (w("ph"), w("pv"));
        let mut view = (w("vv"), w("vh"));
        let (mut score, mut shake, mut contact, mut ghost) = (0, 0i16, 0i16, 3i16);
        let mut evs = Vec::new();
        for step in i["steps"].split(',') {
            let p: Vec<&str> = step.split(':').collect();
            let (dh, dv): (i16, i16) = (p[0].parse().unwrap(), p[1].parse().unwrap());
            player = (player.0.wrapping_add(dh), player.1.wrapping_add(dv));
            view = (view.0.wrapping_add(dv), view.1.wrapping_add(dh));
            let mut events = Vec::new();
            if sp.task == Some(Task::BossDeath) {
                handle_boss_death(&mut sp, &mut boss, &mut rng, &mut shake, &mut events);
            } else {
                handle_boss(&mut sp, &mut boss, &level, view, player, &mut rng, &mut shake, &mut events);
            }
            if p[2] != "-" && sp.hit_task.is_some() {
                let (mut ov, mut oh) = (sp.at.0 + 8, sp.at.1 + 8);
                if p[2] == "kick" {
                    ov -= 40;
                    oh -= 40;
                }
                let mut o = Sprite::new(0, 0, 0);
                o.task = Some(match p[2] {
                    "shot" => Task::PlayerShot,
                    "kick" => Task::Player,
                    _ => Task::Effect,
                });
                if p[2] == "fx" {
                    o.hot_rect = [4, 4, 60, 60];
                }
                o.dir = p[3].parse().unwrap();
                o.at = (ov, oh);
                hit_boss(&mut sp, &mut o, &mut boss, &mut rng, &mut contact, &mut events);
                events.push(SpriteEvent::Sound(-1 - i16::from(o.frozen)));
            }
            let mut l = Vec::new();
            for e in &events {
                match *e {
                    SpriteEvent::Score(s) => score += s,
                    SpriteEvent::GhostsDie => ghost = -5,
                    // the partner's frozen flag, as the recorder logs it
                    SpriteEvent::Sound(x) if x < 0 => l.push(format!("o:{}", -1 - x)),
                    _ => {}
                }
                if let Some(s) = event(e).filter(|_| !matches!(e, SpriteEvent::Sound(x) if *x < 0)) {
                    l.push(s);
                }
            }
            evs.push(if l.is_empty() { "-".to_string() } else { l.join(",") });
        }
        let got = format!(
            "{} | phase={} growing={} change={} anim={} count={} spin={} angle={} cooldown={} blast={} score={score} shake={shake} contact={contact} ghost={ghost} events={}",
            dump(&sp),
            boss.phase,
            u8::from(boss.growing),
            boss.change,
            boss.anim,
            boss.count,
            boss.spin,
            boss.angle,
            boss.cooldown,
            boss.blast_cooldown,
            evs.join(";")
        );
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 6 {
                let diffs: Vec<String> =
                    b.split(' ').zip(got.split(' ')).filter(|(x, y)| x != y).map(|(x, y)| format!("want {x}\n   got {y}")).collect();
                eprintln!("{}\n  {}", &a[..a.len().min(250)], diffs.join("\n  ").chars().take(1500).collect::<String>());
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
