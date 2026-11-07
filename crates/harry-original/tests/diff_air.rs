//! Half pipes, jumps and holes against the original (`HandleHalfPipe`,
//! `JumpScale`, recorded in Unicorn by `re/tools/emu_air.py` on the real
//! levels).

mod common;

use common::{check_numbers, episode1_levels};
use harry_core::air::{Air, UNSET_SCALE, half_pipe, jump_scale};
use harry_core::bounce::Body;
use harry_core::physics::Speed;
use harry_core::random::Random;

fn run(name: &str, f: impl Fn(&[harry_core::Level], &[i32]) -> Vec<i32>) {
    let Some(levels) = episode1_levels() else { return };
    check_numbers(name, "emu_air.py", |v| f(&levels, v));
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
