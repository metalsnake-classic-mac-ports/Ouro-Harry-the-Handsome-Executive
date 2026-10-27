//! Half pipes, jumps and holes against the original (`HandleHalfPipe`,
//! `JumpScale`, recorded in Unicorn by `re/tools/emu_air.py` on the real
//! levels).

use std::path::PathBuf;

use harry_core::air::{Air, UNSET_SCALE, half_pipe, jump_scale};
use harry_core::bounce::Body;
use harry_core::physics::Speed;
use harry_core::random::Random;
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn levels() -> Option<Vec<harry_core::Level>> {
    let bytes = std::fs::read(root().join("re/work/installed/Episode 1.bin")).ok()?;
    let res = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    Some((1..=15).map(|i| hlvl::read(res.get(b"Hlvl", i).unwrap()).unwrap()).collect())
}

fn run(name: &str, f: impl Fn(&[harry_core::Level], &[i32]) -> Vec<i32>) {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff").join(format!("{name}.txt"))) else {
        eprintln!("skipped: {name}.txt not found (run re/tools/emu_air.py)");
        return;
    };
    let Some(levels) = levels() else { return };
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let (a, b) = line.split_once("->").unwrap();
        let v: Vec<i32> = a.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let want: Vec<i32> = b.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let got = f(&levels, &v);
        n += 1;
        if got != want {
            bad += 1;
            if bad <= 8 {
                eprintln!("{name}: {line}\n  port {got:?}");
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} {name} cases differ");
}

#[test]
fn half_pipe_matches() {
    run("halfpipe", |levels, v| {
        let level = &levels[v[0] as usize - 1];
        let at = (v[2] as i16, v[3] as i16);
        let mut body = Body { pos: at, at, fix: (v[4], v[5]), speed: Speed { h: v[6] as i16, v: v[7] as i16 } };
        let mut a = Air { air: v[8] as i16, slide: v[9] as i16, last_tile: (0, 0), flip: v[10] as i16, scale: 0x100 };
        let mut sounds = Vec::new();
        half_pipe(&mut body, &mut a, 32, v[1] != 0, level, &mut sounds);
        vec![
            body.speed.h.into(), body.speed.v.into(), a.air.into(), a.flip.into(), a.last_tile.0.into(), a.last_tile.1.into(),
            a.slide.into(), sounds.len() as i32,
        ]
    });
}

#[test]
fn jump_scale_matches() {
    run("jump", |levels, v| {
        let level = &levels[v[0] as usize - 1];
        let mut rng = Random::new(v[1] as u32);
        let at = (v[2] as i16, v[3] as i16);
        let mut body = Body { pos: at, at, fix: (v[4], v[5]), speed: Speed { h: v[6] as i16, v: v[7] as i16 } };
        let mut dir = v[8] as i16;
        let mut a = Air { air: v[9] as i16, slide: 0, last_tile: (0, 0), flip: v[10] as i16, scale: 0x100 };
        let mut comfort = v[11] as i16;
        let mut sounds = Vec::new();
        let fx = jump_scale(&mut body, &mut a, &mut dir, 32, true, level, &mut rng, &mut comfort, UNSET_SCALE, &mut sounds);
        let view = fx.recentre.unwrap_or((0x5555, 0x5555));
        vec![
            a.air.into(), a.scale.into(), a.scale.into(), body.speed.h.into(), body.speed.v.into(), dir.into(), body.fix.0,
            body.fix.1, body.pos.0.into(), body.pos.1.into(), body.at.0.into(), body.at.1.into(), comfort.into(),
            sounds.len() as i32, view.0.into(), view.1.into(),
        ]
    });
}
