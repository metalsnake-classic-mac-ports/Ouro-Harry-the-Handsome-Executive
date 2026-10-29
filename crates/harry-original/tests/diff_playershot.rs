//! A player shot hitting other sprites (`HitPlayerShotSprite`) against
//! the original, recorded in Unicorn by `re/tools/emu_playershot.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::random::Random;
use harry_core::shots::{hit_player_shot, setup_player_shot};
use harry_core::sprite::Sprite;

#[test]
fn player_shot_hits_match_the_original() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/work/diff/playershot.txt");
    let Ok(text) = std::fs::read_to_string(path) else {
        eprintln!("skipped: playershot.txt not found (run re/tools/emu_playershot.py)");
        return;
    };
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap() as i16;
        let mut rng = Random::new(i["seed"].parse().unwrap());
        let mut sp = Sprite::new(w("shot"), w("h"), w("v"));
        setup_player_shot(&mut sp);
        let mut o = Sprite::new(w("okind"), w("oh"), w("ov"));
        match i["other"] {
            "collider" => harry_core::collider::setup_collider(&mut o, &mut rng),
            "sliding" => harry_core::button::setup_sliding_door(&mut o),
            "dart" => harry_core::dart_enemy::setup_dart_enemy(&mut o),
            "coworker" => harry_core::coworker::setup_coworker(&mut o),
            "scientist" => harry_core::scientist::setup_scientist(&mut o),
            "robot" => harry_core::robot::setup_robot(&mut o, (0, 0)),
            "blob" => harry_core::blob::setup_blob(&mut o, &mut rng),
            "grate" => harry_core::grate::setup_grate(&mut o),
            "plant" => harry_core::kinds::setup_plant(&mut o),
            _ => harry_core::effect::setup_effect(&mut o),
        }
        if w("ostate") != -1 {
            o.state = w("ostate");
        }
        sp.frozen = w("frozen") != 0;
        o.frozen = w("ofrozen") != 0;
        let mut favor = 100;
        let mut rng = Random::new(i["seed2"].parse().unwrap());
        let mut events = Vec::new();
        hit_player_shot(&mut sp, &mut o, &mut favor, &mut rng, &mut events);
        let got = format!("{} || {} | favor={favor} events={}", dump(&sp), dump(&o), common::events(&events));
        // a blob set up facing 24 or more takes its face from past its
        // table in the original (BUGS.md); the port has none
        let b = if i["other"] == "blob" && o.face == 0 && o.dir >= 24 {
            let (left, right) = b.split_once(" || ").unwrap();
            let right: Vec<String> =
                right.split(' ').map(|f| if f.starts_with("face=") { "face=0".to_string() } else { f.to_string() }).collect();
            format!("{left} || {}", right.join(" "))
        } else {
            b.to_string()
        };
        let b = b.as_str();
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
