//! Scientists (1111 / 1211 facing right, 1121 / 1221 left, 1131 / 1231
//! up, 1141 / 1241 down; the 12xx throw harder and more often): they wait
//! for Harry to come into their arc, then throw a test tube - a homing
//! shot. Shooting one costs corporate favor.

use crate::random::Random;
use crate::robot::toward_harry;
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};

/// `snd` ids.
pub mod snd {
    pub const THROW: i16 = 300;
    /// Hit by a shot.
    pub const HIT: i16 = 4700;
}

/// 32 faces (PICT 1111): eight frames each.
const FACES: i16 = 4301;

/// The pose group (1111 …, 1211 → 1111 …).
fn base(kind: i16) -> i16 {
    if kind < 1200 { kind } else { kind - 100 }
}

/// The unit steps of `AssignScientistShotSpeed`.
const UNIT: [f64; 10] = [1.0, 0.985, 0.94, 0.866, 0.766, 0.643, 0.5, 0.342, 0.174, 0.0];

/// `SetupScientistSprite`: layer 9, hot rect (8, 8, 24, 24), waiting
/// (`f6e` 0), no face yet.
pub fn setup_scientist(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.timer = 0;
    sp.task = Some(Task::Scientist);
    sp.hit_task = Some(HitTask::Scientist);
    sp.hit_tile_task = None;
    sp.hot_rect = [8, 8, 24, 24];
    sp.face = 0;
    sp.layer = 9;
    sp.f6e = 0;
    sp.at_from_pos();
    sp.b8b = false;
}

/// `HandleScientistSprite`, only while its `at` is within the view − 64
/// … + 512. `f6e` counts the throw: 0 waits (pose 1) for Harry's
/// direction (the robots') to be within its arc (1111: below 5 or above
/// 30; 1121: 15–21; 1131: 6–12; 1141: 24–30), keeping it in `speed.h`;
/// 1–8 wind up (poses 1, 2, 3), 9 throws (pose 4, sound 300: a shot of
/// its kind from its hand, 1200 along the kept direction - 1800 for 12xx
/// -, timer 0 and dir 0, so it homes at once), 10–20 follow through
/// (poses 4–7); then, after the timer (set by a hit), it counts on and
/// starts over at random (1 in 25 a frame from 46, 12xx 1 in 15 from 16).
/// A hit scientist (layer 10) is back on 7. Face: pose + 8 × group.
/// `view` is the view's corner (v, h), `player` Harry's top-left (h, v).
pub fn handle_scientist(sp: &mut Sprite, view: (i16, i16), player: (i16, i16), rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    let (vv, vh) = (i32::from(view.0), i32::from(view.1));
    let (v, h) = (i32::from(sp.at.0), i32::from(sp.at.1));
    if vv - 64 <= v && v < vv + 512 && vh - 64 <= h && h < vh + 512 {
        let group = base(sp.kind);
        match sp.f6e {
            0 => {
                sp.dir = 1;
                let d = toward_harry(sp, player);
                let ready = match group {
                    1131 => 5 < d && d < 13,
                    1121 => 14 < d && d < 22,
                    1111 => !(5..=30).contains(&d),
                    1141 => 23 < d && d < 31,
                    _ => false,
                };
                if ready {
                    sp.f6e += 1;
                    sp.speed.h = d;
                }
            }
            1 | 2 => sp.f6e += 1,
            3..=5 => {
                sp.dir = 2;
                sp.f6e += 1;
            }
            6..=8 => {
                sp.dir = 3;
                sp.f6e += 1;
            }
            9 => {
                sp.dir = 4;
                events.push(SpriteEvent::Sound3D(snd::THROW));
                let hand = match group {
                    1131 => Some((20, 2)),
                    1121 => Some((4, 4)),
                    1111 => Some((22, 20)),
                    1141 => Some((5, 19)),
                    _ => None,
                };
                if let Some((dh, dv)) = hand {
                    let m = f64::from(if sp.kind < 1200 { 1200 } else { 1800 });
                    let mut speed = crate::physics::Speed::default();
                    let d = sp.speed.h;
                    if (0..36).contains(&d) {
                        let k = (d % 9) as usize;
                        let (along, across) = (UNIT[k] * m, UNIT[9 - k] * m);
                        let (sh, sv) = match d / 9 {
                            0 => (along, -across),
                            1 => (-across, -along),
                            2 => (-along, across),
                            _ => (across, along),
                        };
                        speed.h = sh as i32 as i16;
                        speed.v = sv as i32 as i16;
                    }
                    events.push(SpriteEvent::Shot {
                        kind: sp.kind,
                        h: sp.pos.1.wrapping_add(dh),
                        v: sp.pos.0.wrapping_add(dv),
                        speed,
                        dir: 0,
                        fix: None,
                        timer: Some(0),
                    });
                }
                sp.f6e += 1;
            }
            10 | 11 => {
                sp.dir = 4;
                sp.f6e += 1;
            }
            12..=14 => {
                sp.dir = 5;
                sp.f6e += 1;
            }
            15..=17 => {
                sp.dir = 6;
                sp.f6e += 1;
            }
            18..=20 => {
                sp.dir = 7;
                sp.f6e += 1;
            }
            _ => {
                if sp.timer == 0 {
                    let (from, chance) = if sp.kind < 1200 { (46, 25) } else { (16, 15) };
                    if sp.f6e < from || rng.below(chance) != 2 {
                        sp.f6e += 1;
                    } else {
                        sp.f6e = 0;
                    }
                } else {
                    sp.timer -= 1;
                }
            }
        }
    }
    if sp.layer == 10 {
        sp.layer = 7;
    }
    let group = base(sp.kind);
    sp.face = 0;
    if (0..8).contains(&sp.dir) {
        let row = match group {
            1111 => Some(0),
            1121 => Some(8),
            1131 => Some(16),
            1141 => Some(24),
            _ => None,
        };
        if let Some(r) = row {
            sp.face = FACES + r + sp.dir;
        }
    }
}

/// `HitScientistSprite`: a player shot is gone; sound 4700, layer 10,
/// corporate favor − 3, and it pauses 65–134 frames before its next
/// throw.
pub fn hit_scientist(sp: &mut Sprite, other: &mut Sprite, favor: &mut i16, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen || other.task != Some(Task::PlayerShot) {
        return;
    }
    other.frozen = true;
    events.push(SpriteEvent::Sound3D(snd::HIT));
    sp.layer = 10;
    *favor = favor.wrapping_sub(3);
    sp.timer = rng.below(70) + 65;
}
