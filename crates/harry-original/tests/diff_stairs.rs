//! The stairs against the original, recorded in Unicorn by
//! `re/tools/emu_stairs.py`.

mod common;

use common::{Fields, Tally, dump, recording};
use harry_core::bounce::Body;
use harry_core::physics::Speed;
use harry_core::sprite::Sprite;
use harry_core::stairs::{setup_stairs, stairs_collision};

#[test]
fn stairs_match_the_original() {
    let Some(text) = recording("stairs", "emu_stairs.py") else { return };
    let mut tally = Tally::new("stairs", 8);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i = Fields::parse(a);
        let w = |k: &str| i.i64(k);
        let mut stairs = Sprite::new(w("kind") as i16, w("sh") as i16, w("sv") as i16);
        setup_stairs(&mut stairs);
        let hot: Vec<i16> = i.str("hot").split(':').map(|x| x.parse().unwrap()).collect();
        let mut body = Body {
            pos: (w("pv") as i16, w("ph") as i16),
            at: (w("v") as i16, w("h") as i16),
            fix: (w("fixh") as i32, w("fixv") as i32),
            speed: Speed { h: w("speedh") as i16, v: w("speedv") as i16 },
        };
        let (mut air, mut flip) = (w("air") as i16, w("flip") as i16);
        let hot = [hot[0], hot[1], hot[2], hot[3]];
        stairs_collision(&mut body, &mut air, &mut flip, w("half") as i16, hot, &stairs, w("bounce") as i16);
        let got = format!(
            "{} | pv={} ph={} v={} h={} speedv={} speedh={} air={air} flip={flip} fixh={} fixv={}",
            dump(&stairs),
            body.pos.0,
            body.pos.1,
            body.at.0,
            body.at.1,
            body.speed.v,
            body.speed.h,
            body.fix.0,
            body.fix.1
        );
        tally.words(a, b, &got);
    }
    tally.finish();
}
