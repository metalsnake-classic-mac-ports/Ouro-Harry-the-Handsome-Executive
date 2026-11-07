//! Fire pipes and their flames against the original, recorded in Unicorn
//! by `re/tools/emu_firepipe.py`.

mod common;

use common::{check_named, dump};
use harry_core::effect::{handle_effect, setup_effect};
use harry_core::firepipe::{handle_fire_pipe, setup_fire_pipe};
use harry_core::random::Random;
use harry_core::sprite::Sprite;

#[test]
fn fire_pipes_match_the_original() {
    check_named("firepipe", "emu_firepipe.py", |i| {
        let mut rng = Random::new(i.u32("seed"));
        let mut sp = Sprite::new(i.i16("kind"), i.i16("h"), i.i16("v"));
        setup_fire_pipe(&mut sp, &mut rng);
        sp.level_index = i.i16("index");
        if i.i64("f6e") != -1 {
            sp.f6e = i.i32("f6e");
        }
        sp.frozen = i.flag("frozen");
        let info1 = (i.i64("index") >= 0).then_some(i.i16("info1"));
        let mut evs = Vec::new();
        for _ in 0..i.i64("frames") {
            let mut events = Vec::new();
            handle_fire_pipe(&mut sp, info1, &mut rng, &mut events);
            evs.push(common::events(&events));
        }
        format!("{} | events={}", dump(&sp), evs.join(";"))
    });
}

#[test]
fn flames_match_the_original() {
    check_named("flame", "emu_firepipe.py", |i| {
        let pipe = i.i16("pipe");
        let mut sp = Sprite::new(pipe, i.i16("h"), i.i16("v"));
        setup_effect(&mut sp);
        sp.kind = i.i16("kind");
        sp.f7c = pipe;
        for f in 0..i.i64("frames") {
            if f == i.i64("touch") {
                sp.f6e = 3;
            }
            handle_effect(&mut sp);
        }
        dump(&sp)
    });
}
