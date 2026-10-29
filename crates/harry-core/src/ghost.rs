//! Ghosts (5001, 5011): they drift toward Harry through walls. Only
//! Harry's shots hurt them: hit, a ghost vanishes, blinks and comes back
//! somewhere else in view; out of hit points it bursts.

use crate::physics::Speed;
use crate::random::Random;
use crate::robot::{approach, toward_harry};
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};

/// `snd` ids.
pub mod snd {
    /// Now and then (1 in 1000 a frame).
    pub const MOAN: i16 = 850;
    pub const DIE: i16 = 851;
    pub const HIT: i16 = 852;
    /// Harry touched by a drifting ghost.
    pub const TOUCH: i16 = 224;
}

/// 36 faces (PICT 224).
const FACES: i16 = 3301;

/// `f6e` modes.
mod mode {
    /// Hit: waiting, blinking, moving.
    pub const HIT: i32 = 1;
    pub const DRIFT: i32 = 2;
    pub const DEAD: i32 = 3;
}

/// The unit steps of `AccelerateGhost` for each tenth of a quarter turn.
const UNIT: [f64; 10] = [1.0, 0.985, 0.94, 0.866, 0.766, 0.643, 0.5, 0.342, 0.174, 0.0];

/// `AccelerateGhost`: the speed set to 384 (5011 and up: 768) along `dir`.
fn accelerate(sp: &mut Sprite) {
    let m = f64::from(if sp.kind > 5001 { 0x300 } else { 0x180 });
    sp.speed = Speed::default();
    if !(0..36).contains(&sp.dir) {
        return;
    }
    let k = (sp.dir % 9) as usize;
    let (along, across) = (UNIT[k] * m, UNIT[9 - k] * m);
    let (dh, dv) = match sp.dir / 9 {
        0 => (along, -across),
        1 => (-across, -along),
        2 => (-along, across),
        _ => (across, along),
    };
    sp.speed.h = dh as i32 as i16;
    sp.speed.v = dv as i32 as i16;
}

/// `SetupGhostSprite`: facing Harry, layer 20, hot rect (0, 0, 40, 40),
/// drifting, 3 hit points (5011 and up: 5). `at` is left as it was.
pub fn setup_ghost(sp: &mut Sprite, player: (i16, i16)) {
    sp.init();
    sp.dir = toward_harry(sp, player);
    sp.task = Some(Task::Ghost);
    sp.hit_task = Some(HitTask::Ghost);
    sp.hit_tile_task = None;
    sp.hot_rect = [0, 0, 40, 40];
    sp.layer = 20;
    sp.speed = Speed::default();
    sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
    sp.f6e = mode::DRIFT;
    sp.state = if sp.kind == 5001 { 3 } else { 5 };
    sp.face = FACES + sp.dir;
    sp.timer = 0;
    sp.b8b = true;
}

/// `HandleGhostSprite`. It turns toward Harry one step a frame. Drifting:
/// 384 / 768 along its direction. Hit: it stands; the timer (60) counts
/// down, the face blinking (odd frames none); at 30 it vanishes and comes
/// back at a random place in the view (`SATRand(408)` both ways) whose
/// 40 × 40 rect does not touch Harry's surroundings (his top-left − 32
/// to + 96), blinking again until 0, then drifts. Dead: an explosion (5001:
/// kind 10, else 1; frame 4, timer 0), 2000 / 4000 points, gone. 1 in
/// 1000 a frame it moans. `view` is the view's corner (v, h), `player`
/// Harry's top-left (h, v).
pub fn handle_ghost(sp: &mut Sprite, view: (i16, i16), player: (i16, i16), rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    let d = toward_harry(sp, player);
    approach(sp, d);
    sp.b8a = true;
    sp.face = FACES + sp.dir;
    if sp.state < 0 {
        sp.f6e = mode::DEAD;
    }
    match sp.f6e {
        mode::DRIFT => accelerate(sp),
        mode::HIT => {
            sp.speed = Speed::default();
            sp.timer -= 1;
            if sp.timer == 30 {
                sp.face = 0;
                let (ph, pv) = (i32::from(player.0), i32::from(player.1));
                let around = [pv - 32, ph - 32, pv + 96, ph + 96];
                loop {
                    let h = view.1.wrapping_add(rng.below(0x198));
                    let v = view.0.wrapping_add(rng.below(0x198));
                    sp.pos = (v, h);
                    let me = [i32::from(v), i32::from(h), i32::from(v) + 40, i32::from(h) + 40];
                    let touch = me[0].max(around[0]) < me[2].min(around[2]) && me[1].max(around[1]) < me[3].min(around[3]);
                    if !touch {
                        break;
                    }
                }
                sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
            } else if sp.timer < 1 {
                sp.f6e = mode::DRIFT;
            } else if sp.timer & 1 == 1 {
                sp.face = 0;
            }
        }
        mode::DEAD => {
            let kind = if sp.kind == 5001 { 10 } else { 1 };
            events.push(SpriteEvent::ExplosionAt { h: sp.at.1.wrapping_sub(12), v: sp.at.0.wrapping_sub(12), a: kind, b: 4, timer: 0 });
            events.push(SpriteEvent::Score(if sp.kind > 5001 { 4000 } else { 2000 }));
            sp.frozen = true;
        }
        _ => {}
    }
    if rng.below(1000) == 69 {
        events.push(SpriteEvent::Sound(snd::MOAN));
    }
    sp.integrate();
}

/// `HitGhostSprite`: a player shot (when the ghost is not already hit,
/// timer 0) - with hit points left: one less, hit for 60 frames, sound
/// 852; else dead (its level entry cleared, 3000 points, sound 851). The
/// shot is gone either way.
pub fn hit_ghost(sp: &mut Sprite, other: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen || other.task != Some(Task::PlayerShot) || sp.timer != 0 {
        return;
    }
    if sp.state < 1 {
        sp.f6e = mode::DEAD;
        events.push(SpriteEvent::Sound(snd::DIE));
        if (0..256).contains(&sp.level_index) {
            events.push(SpriteEvent::Forget(sp.level_index));
        }
        events.push(SpriteEvent::Score(3000));
    } else {
        sp.f6e = mode::HIT;
        events.push(SpriteEvent::Sound(snd::HIT));
        sp.state -= 1;
        sp.timer = 60;
    }
    other.frozen = true;
}
