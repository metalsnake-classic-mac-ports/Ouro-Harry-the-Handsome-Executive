//! Co-workers against the original, recorded in Unicorn by
//! `re/tools/emu_coworker.py`.

mod common;

use common::{check_named, dump};
use harry_core::coworker::{handle_coworker, setup_coworker};
use harry_core::random::Random;
use harry_core::sprite::Sprite;

#[test]
fn coworkers_match_the_original() {
    check_named("coworker", "emu_coworker.py", |i| {
        let mut sp = Sprite::new(i.i16("kind"), i.i16("h"), i.i16("v"));
        setup_coworker(&mut sp);
        sp.dir = i.i16("dir");
        sp.layer = i.i16("layer");
        sp.frozen = i.flag("frozen");
        let mut rng = Random::new(i.u32("seed"));
        for _ in 0..i.i64("frames") {
            handle_coworker(&mut sp, &mut rng);
        }
        dump(&sp)
    });
}
