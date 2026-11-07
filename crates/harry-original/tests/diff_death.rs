//! Harry's death against the original, recorded in Unicorn by
//! `re/tools/emu_death.py`.

mod common;

use common::{check_named, dump};
use harry_core::death::{DeathEvent, handle_player_death};
use harry_core::frame::PlayerGlobals;
use harry_core::game::setup_player;
use harry_core::sprite::{Sprite, Task};

#[test]
fn the_death_matches_the_original() {
    check_named("death", "emu_death.py", |i| {
        let w = |k: &str| i.i16(k);
        let mut g = PlayerGlobals {
            comfort_shown: w("comfort"),
            favor_shown: w("favor"),
            cur_slot: w("slot"),
            shot_kind: w("shot"),
            recoil: w("recoil"),
            ..Default::default()
        };
        let mut sp = Sprite::new(69, w("h"), w("v"));
        setup_player(&mut sp, &mut PlayerGlobals::default());
        sp.task = Some(Task::PlayerDeath);
        sp.timer = w("timer");
        sp.dir = w("dir");
        sp.at = (w("v"), w("h"));
        let (mut shake, mut done) = (0, false);
        let mut evs = Vec::new();
        for _ in 0..w("frames") {
            let mut events = Vec::new();
            done |= handle_player_death(&mut sp, &mut g, &mut events);
            let mut l = Vec::new();
            for e in events {
                match e {
                    DeathEvent::FadeMusic => l.push("fade".to_string()),
                    DeathEvent::Sound(id) => l.push(format!("p:{id}")),
                    DeathEvent::Explosion { h, v, dir, behind } => l.push(format!("x:{h}:{v}:0:{dir}:{}", u8::from(behind))),
                    DeathEvent::Flame { kind, h, v } => l.push(format!("new:{kind}:{h}:{v}:3:{kind}:1")),
                    DeathEvent::Shake(s) => shake = s,
                }
            }
            evs.push(if l.is_empty() { "-".to_string() } else { l.join(",") });
        }
        format!(
            "{} | slot={} shot={} recoil={} shake={shake} done={} events={}",
            dump(&sp),
            g.cur_slot,
            g.shot_kind,
            g.recoil,
            u8::from(done),
            evs.join(";")
        )
    });
}
