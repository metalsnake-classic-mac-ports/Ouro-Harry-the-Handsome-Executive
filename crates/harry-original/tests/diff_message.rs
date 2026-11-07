//! The message sprite against the original, recorded in Unicorn by
//! `re/tools/emu_message.py` (ShowBalloon stubbed with recorded answers).

mod common;

use harry_core::random::Random;

use common::{Tally, dump, installed, named_cases, recording};
use harry_core::kinds::{BalloonText, handle_message, hit_message, setup_message};
use harry_core::sprite::{Sprite, Task};
use harry_original::hlvl;

#[test]
fn message_matches_the_original() {
    let Some(text) = recording("message", "emu_message.py") else { return };
    let Some(res) = installed("Episode 1.bin") else { return };
    let mut level = hlvl::read(res.get(b"Hlvl", 1).unwrap()).unwrap();
    let Some(app) = installed("Harry 1.0.1.bin") else { return };
    let base = harry_original::words::words(&app).unwrap();
    let mut tally = Tally::new("message", 8);
    for (a, i, b) in named_cases(&text) {
        let w = |k: &str| i.i16(k);
        let mut sp = Sprite::new(w("kind"), w("h"), w("v"));
        setup_message(&mut sp);
        let li = w("levelindex");
        sp.level_index = li;
        if li >= 0 {
            level.sprites[li as usize].info[0] = w("info0");
            level.sprites[li as usize].info[1] = w("info1");
        }
        let mut other = Sprite::new(0, 0, 0);
        other.task = Some(if w("other") == 2 { Task::Bonus } else { Task::Player });
        other.frozen = w("other") == 3;
        other.hot_rect = [22, if w("kicking") != 0 { 0 } else { 22 }, 42, 42];
        let mut answers: Vec<bool> = i.str("answers").chars().map(|c| c == '1').collect();
        answers.reverse();
        let mut calls = Vec::new();
        let mut words = base.clone();
        let mut rng = Random::new(i.u32("seed"));
        let mut show = |_: &Sprite, t: BalloonText, mode: i16| {
            let ans = answers.pop().unwrap_or(false);
            let name = match t {
                BalloonText::World(n) => n.to_string(),
                BalloonText::Buzzwords(t) => format!("buzz:{}", t.iter().map(|x| format!("{x:02x}")).collect::<String>()),
                BalloonText::Phonewords(_) => "phone".into(),
                BalloonText::Message(n) => format!("message:{n}"),
            };
            calls.push(format!("b:{name}:{mode}:{}", u8::from(ans)));
            ans
        };
        let frames = w("frames");
        for f in 0..=frames {
            if f == w("hitframe") {
                if f == 0 {
                    sp.frozen = w("frozen") != 0;
                }
                hit_message(&mut sp, &other, &level, w("pref8") > 2, &mut words, &mut rng, &mut show);
            }
            if f < frames {
                handle_message(&mut sp, &level, &mut words, &mut rng, &mut show);
            }
        }
        let got = format!(
            "{} | pending={} calls={} seed={}",
            dump(&sp),
            u8::from(sp.balloon_pending),
            if calls.is_empty() { "-".into() } else { calls.join(",") },
            rng.seed
        );
        tally.words(a, b, &got);
    }
    tally.finish();
}
