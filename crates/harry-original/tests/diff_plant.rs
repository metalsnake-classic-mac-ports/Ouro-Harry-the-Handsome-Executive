//! The plant against the original, recorded in Unicorn by
//! `re/tools/emu_plant.py`.

mod common;

use common::{Tally, dump, installed, levels_of, named_cases, recording};
use harry_core::Physics;
use harry_core::kinds::{ObjectMasks, handle_plant, hit_plant_tile, setup_plant};
use harry_core::random::Random;
use harry_core::sprite::Sprite;
use harry_core::tiles::touched_tiles;
use harry_original::masks;

#[test]
fn plant_movement_matches_the_original() {
    let Some(text) = recording("plant", "emu_plant.py") else { return };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let levels = levels_of(&ep);
    let plant = masks::from_cicn(&gfx, 599).unwrap();
    let obstacle = masks::from_cicn(&gfx, 290).unwrap();
    let walls = masks::walls(&app).unwrap();
    let ms = ObjectMasks { plant: &plant, walls: &walls, obstacle: &obstacle };
    let physics = Physics::default();
    let mut tally = Tally::new("plant", 8);
    for (a, i, b) in named_cases(&text) {
        let w = |k: &str| i.i64(k);
        let level = &levels[w("level") as usize - 1];
        let mut sp = Sprite::new(601, w("h") as i16, w("v") as i16);
        setup_plant(&mut sp);
        sp.speed.h = w("speedh") as i16;
        sp.speed.v = w("speedv") as i16;
        sp.dir = w("dir") as i16;
        sp.air = w("air") as i16;
        sp.slide = w("slide") as i16;
        sp.face = 0;
        let mut rng = Random::new(w("seed") as u32);
        let mut flag = true;
        let mut sounds = Vec::new();
        for _ in 0..w("frames") {
            handle_plant(&mut sp, level, &physics, &mut rng, &mut flag, &mut sounds);
            let mut hits = Vec::new();
            touched_tiles(level, sp.at, sp.hot_rect, [0, 0, 36, 36], |h| hits.push(h));
            for h in hits {
                hit_plant_tile(&mut sp, h, level, &ms, &physics);
            }
        }
        let got = format!("{} | flag={} sounds={}", dump(&sp), if flag { 0x5555 } else { 0x55 }, sounds.len());
        tally.words(a, b, &got);
    }
    tally.finish();
}
