//! The mail guy against the original, recorded in Unicorn by
//! `re/tools/emu_mailguy.py`.

mod common;

use common::{Tally, dump, episode1_levels, named_cases, recording};
use harry_core::mailguy::{handle_mail_guy, hit_mail_guy_tile, setup_mail_guy};
use harry_core::random::Random;
use harry_core::sprite::Sprite;
use harry_core::tiles::touched_tiles;

#[test]
fn mail_guys_match_the_original() {
    let Some(text) = recording("mailguy", "emu_mailguy.py") else { return };
    let Some(levels) = episode1_levels() else { return };
    let mut tally = Tally::new("mailguy", 8);
    for (a, i, b) in named_cases(&text) {
        let w = |k: &str| i.i64(k);
        let mut level = levels[w("level") as usize - 1].clone();
        let index = w("index") as i16;
        level.sprites[index as usize].info[0] = w("info0") as i16;
        level.sprites[index as usize].info[1] = w("info1") as i16;
        let mut sp = Sprite::new(w("kind") as i16, w("h") as i16, w("v") as i16);
        setup_mail_guy(&mut sp);
        sp.level_index = index;
        sp.frozen = w("frozen") != 0;
        let mut rng = Random::new(w("seed") as u32);
        let mut evs = Vec::new();
        for _ in 0..w("frames") {
            let mut events = Vec::new();
            handle_mail_guy(&mut sp, &mut level, &mut rng, &mut events);
            evs.push(common::events(&events));
            let bounds = if matches!(sp.face, 3701 | 3702) { [0, 0, 64, 34] } else { [0, 0, 34, 64] };
            let mut hits = Vec::new();
            touched_tiles(&level, sp.at, sp.hot_rect, bounds, |h| hits.push(h));
            for h in hits {
                hit_mail_guy_tile(&mut sp, h);
            }
        }
        let e = &level.sprites[index as usize];
        let got = format!("{} | entry={}:{}:{} events={}", dump(&sp), e.id, e.info[0], e.info[1], evs.join(";"));
        tally.words(a, b, &got);
    }
    tally.finish();
}
