//! The colliders against the original, recorded in Unicorn by
//! `re/tools/emu_collider.py`.

mod common;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::Speed;
use harry_core::collider::{ColliderWorld, centre, handle_collider, hit_collider, is_repelled, repel, setup_collider};
use harry_core::frame::PlayerGlobals;
use harry_core::kinds::BalloonText;
use harry_core::random::Random;
use harry_core::sprite::{Sprite, Task};
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fields(s: &str) -> HashMap<&str, &str> {
    s.split(' ').map(|kv| kv.split_once('=').unwrap()).collect()
}

fn task(name: &str) -> Task {
    match name {
        "player" => Task::Player,
        "playershot" => Task::PlayerShot,
        "enemyshot" => Task::EnemyShot,
        "plant" => Task::Plant,
        "bonus" => Task::Bonus,
        "barrel" => Task::Barrel,
        "chair" => Task::Chair,
        "robot" => Task::Robot,
        _ => Task::Door,
    }
}

#[test]
fn colliders_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/collider.txt")) else {
        eprintln!("skipped: collider.txt not found (run re/tools/emu_collider.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Episode 1.bin")) else { return };
    let ep = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let level1 = hlvl::read(ep.get(b"Hlvl", 1).unwrap()).unwrap();
    let Ok(app) = std::fs::read(root().join("re/work/installed/Harry 1.0.1.bin")) else { return };
    let app = Resources::parse(&MacBinary::parse(&app).unwrap().rsrc).unwrap();
    let base_words = harry_original::words::words(&app).unwrap();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i = fields(a);
        let w = |k: &str| i[k].parse::<i32>().unwrap();
        let mut level = level1.clone();
        let index = w("levelindex") as i16;
        if index >= 0 {
            let e = &mut level.sprites[index as usize];
            e.present = true;
            e.id = w("kind") as i16;
            e.info[0] = w("info0") as i16;
            e.info[1] = w("info1") as i16;
            let link = w("info0").unsigned_abs() as usize;
            if link != index as usize && link < 256 {
                level.sprites[link].info[1] = w("linkinfo1") as i16;
            }
        }
        let mut g = PlayerGlobals {
            player_pos: (w("ph") as i16, w("pv") as i16),
            caffeine: w("caffeine") as i16,
            comfort: w("comfort") as i16,
            ..Default::default()
        };
        g.ammo.staple = w("staple") as i16;
        g.ammo.badass = w("badass") as i16;
        g.ammo.shrapnel = w("shrapnel") as i16;
        let mut harry_speed = Speed::default();
        let mut magnet = w("magnet") as i16;
        let mut rng = Random::new(w("seed") as u32);
        let mut sp = Sprite::new(w("kind") as i16, w("h") as i16, w("v") as i16);
        setup_collider(&mut sp, &mut rng);
        sp.level_index = index;
        if w("settimer") != -9999 {
            sp.timer = w("settimer") as i16;
        }
        if w("setdir") != -9999 {
            sp.dir = w("setdir") as i16;
        }
        if w("setf6e") != -9999 {
            sp.f6e = w("setf6e");
        }
        let mut targets: Vec<Sprite> = (0..w("ntargets"))
            .map(|t| {
                let p: Vec<&str> = i[format!("t{t}").as_str()].split(':').collect();
                let x = |k: usize| p[k].parse::<i16>().unwrap();
                let mut s = Sprite::new(0, 0, 0);
                s.task = Some(task(p[0]));
                s.at = (x(1), x(2));
                s.hot_rect = [x(3), x(4), x(5), x(6)];
                s.speed = Speed { v: x(7), h: x(8) };
                s
            })
            .collect();
        let mut other = Sprite::new(0, 0, 0);
        other.task = Some(if w("other") == 3 { Task::Bonus } else { Task::Player });
        other.frozen = w("other") == 2;
        let log = RefCell::new(Vec::<String>::new());
        let mut save = || {
            log.borrow_mut().push(format!("save:{}", w("save")));
            w("save") != 0
        };
        let mut words = base_words.clone();
        let mut show = |_: &Sprite, t: BalloonText, mode: i16| {
            let BalloonText::Phonewords(t) = t else { panic!("not a phone balloon") };
            let hex: String = t.iter().map(|x| format!("{x:02x}")).collect();
            log.borrow_mut().push(format!("b:phone:{hex}:{mode}:{}", w("show")));
            w("show") != 0
        };
        let frames = w("frames");
        for f in 0..=frames {
            if f == w("frozenframe") {
                sp.frozen = true;
            }
            if f == w("hitframe") {
                hit_collider(&mut sp, &other, &mut level, &mut words, &mut rng, &mut save, &mut show);
            }
            if f < frames {
                let mut events = Vec::new();
                let mut world = ColliderWorld {
                    level: &mut level,
                    g: &mut g,
                    harry_speed: &mut harry_speed,
                    magnet: &mut magnet,
                    fan_sound_playing: w("busy") != 0,
                    rng: &mut rng,
                };
                if handle_collider(&mut sp, &mut world, &mut events) {
                    for t in targets.iter_mut().filter(|t| t.task.is_some_and(is_repelled)) {
                        let c = centre(t);
                        repel(&sp, c, &mut t.speed);
                    }
                }
                log.borrow_mut().extend(events.iter().map(common::event));
            }
        }
        let mut outs = Vec::new();
        if index >= 0 {
            let e = &level.sprites[index as usize];
            outs.push(format!("present={} id={} e0={} e1={}", u8::from(e.present), e.id, e.info[0], e.info[1]));
        }
        outs.push(format!("comfort={} magnet={magnet} harryspeedv={}", g.comfort, harry_speed.v));
        for (k, t) in targets.iter().enumerate() {
            outs.push(format!("t{k}={}:{}", t.speed.v, t.speed.h));
        }
        let log = log.into_inner();
        outs.push(format!("events={}", if log.is_empty() { "-".into() } else { log.join(",") }));
        let got = format!("{} | {}", dump(&sp), outs.join(" "));
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 8 {
                let diffs: Vec<String> =
                    b.split(' ').zip(got.split(' ')).filter(|(x, y)| x != y).map(|(x, y)| format!("want {x} got {y}")).collect();
                eprintln!("{a}\n  {}", diffs.join(", "));
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
