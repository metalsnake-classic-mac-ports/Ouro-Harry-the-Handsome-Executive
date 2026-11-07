//! A player shot hitting other sprites (`HitPlayerShotSprite`) against
//! the original, recorded in Unicorn by `re/tools/emu_playershot.py`.

mod common;

use common::{Tally, dump, named_cases, recording};
use harry_core::random::Random;
use harry_core::shots::{hit_player_shot, setup_player_shot};
use harry_core::sprite::Sprite;

#[test]
fn player_shot_hits_match_the_original() {
    let Some(text) = recording("playershot", "emu_playershot.py") else { return };
    let mut tally = Tally::new("playershot", 8);
    for (a, i, b) in named_cases(&text) {
        let w = |k: &str| i.i16(k);
        let mut rng = Random::new(i.u32("seed"));
        let mut sp = Sprite::new(w("shot"), w("h"), w("v"));
        setup_player_shot(&mut sp);
        let mut o = Sprite::new(w("okind"), w("oh"), w("ov"));
        match i.str("other") {
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
        let mut rng = Random::new(i.str("seed2").parse().unwrap());
        let mut events = Vec::new();
        hit_player_shot(&mut sp, &mut o, &mut favor, &mut rng, &mut events);
        let got = format!("{} || {} | favor={favor} events={}", dump(&sp), dump(&o), common::events(&events));
        // a blob set up facing 24 or more takes its face from past its
        // table in the original (BUGS.md); the port has none
        let b = if i.str("other") == "blob" && o.face == 0 && o.dir >= 24 {
            let (left, right) = b.split_once(" || ").unwrap();
            let right: Vec<String> =
                right.split(' ').map(|f| if f.starts_with("face=") { "face=0".to_string() } else { f.to_string() }).collect();
            format!("{left} || {}", right.join(" "))
        } else {
            b.to_string()
        };
        let b = b.as_str();
        tally.words(a, b, &got);
    }
    tally.finish();
}
