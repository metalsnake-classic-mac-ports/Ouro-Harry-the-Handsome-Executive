//! The water cooler against the original, recorded in Unicorn by
//! `re/tools/emu_cooler.py`.

mod common;

use common::{Fields, Tally, dump, installed, recording, word_diff};
use harry_core::kinds::{CoolerMasks, handle_cooler, hit_cooler, setup_cooler};
use harry_core::player::{HOT_RECT, KICK_HOT_RECT};
use harry_core::sprite::{Sprite, Task};
use harry_original::{hlvl, hrsp, masks};

#[test]
fn cooler_matches_the_original() {
    let Some(text) = recording("cooler", "emu_cooler.py") else { return };
    let (Some(gfx), Some(ep)) = (installed("Harry Graphics.bin"), installed("Episode 1.bin")) else { return };
    let body = masks::from_cicn(&gfx, 299).unwrap();
    let cooler = masks::from_face(&hrsp::decode(gfx.get(b"HrSp", 2002).unwrap()).unwrap());
    let ms = CoolerMasks { body: &body, cooler: &cooler };
    let mut level = hlvl::read(ep.get(b"Hlvl", 1).unwrap()).unwrap();
    let mut tally = Tally::new("cooler", 8);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let (want_dump, want_out) = b.split_once(" | ").unwrap();
        let i = Fields::parse(a);
        let w = |k: &str| i.i16(k);
        let mut sp = Sprite::new(w("kind"), w("ch"), w("cv"));
        setup_cooler(&mut sp);
        sp.dir = w("dir");
        let li = w("levelindex");
        sp.level_index = li;
        if li >= 0 {
            level.sprites[li as usize].present = true;
            level.sprites[li as usize].info[1] = w("info1");
        }
        let mut events = Vec::new();
        for _ in 0..w("frames") {
            handle_cooler(&mut sp, &mut level, &mut events);
        }
        sp.frozen = w("frozen") != 0;
        let mut o = Sprite::new(if w("other") == 5 { 9610 } else { 0 }, w("oah"), w("oav"));
        o.task = Some(match w("other") {
            1 => Task::Player,
            2 => Task::PlayerShot,
            3 => Task::EnemyShot,
            4 | 5 => Task::Effect,
            _ => Task::Bonus,
        });
        o.frozen = w("ofrozen") != 0;
        o.air = w("oair");
        o.dir = w("odir");
        o.at = (w("oav"), w("oah"));
        o.fix = (0x7777, 0x7777);
        o.speed.v = w("ospeedv");
        o.speed.h = w("ospeedh");
        o.hot_rect = if w("kicking") != 0 { KICK_HOT_RECT } else { HOT_RECT };
        let mut contact = w("contact");
        hit_cooler(&mut sp, &mut o, &ms, &mut contact, &mut events);
        let present = if li >= 0 { i16::from(level.sprites[li as usize].present) } else { 9 };
        let got_out = format!(
            "present={present} contact={contact} ofrozen={} opv={} oph={} oav={} oah={} ofixh={} ofixv={} ospeedv={} ospeedh={} events={}",
            u8::from(o.frozen),
            o.pos.0,
            o.pos.1,
            o.at.0,
            o.at.1,
            o.fix.0,
            o.fix.1,
            o.speed.v,
            o.speed.h,
            common::events(&events)
        );
        let got_dump = dump(&sp);
        tally.case(got_dump == want_dump && got_out == want_out, || {
            format!("{a}\n  {}", word_diff(&format!("{want_dump} {want_out}"), &format!("{got_dump} {got_out}")))
        });
    }
    tally.finish();
}
