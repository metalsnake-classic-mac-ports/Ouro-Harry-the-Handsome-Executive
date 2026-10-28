//! The mail guy against the original, recorded in Unicorn by
//! `re/tools/emu_mailguy.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::mailguy::{handle_mail_guy, hit_mail_guy_tile, setup_mail_guy};
use harry_core::random::Random;
use harry_core::sprite::Sprite;
use harry_core::tiles::touched_tiles;
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn mail_guys_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/mailguy.txt")) else {
        eprintln!("skipped: mailguy.txt not found (run re/tools/emu_mailguy.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Episode 1.bin")) else { return };
    let ep = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let levels: Vec<_> = (1..=15).map(|i| hlvl::read(ep.get(b"Hlvl", i).unwrap()).unwrap()).collect();
    let (mut bad, mut n) = (0, 0);
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap();
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
