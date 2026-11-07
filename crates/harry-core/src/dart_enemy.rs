//! Dart enemies (1011 / 1021 / 1031 / 1041, facing right, left, up,
//! down): an office worker in the wall who throws darts - or, with info
//! #0 = 1, paper balls - while near the view (`SetupDartEnemySprite`,
//! `HandleDartEnemySprite`, `ThrowDart`, `HitDartEnemySprite`).

use crate::frame::PlayerGlobals;
use crate::level::Level;
use crate::physics::Speed;
use crate::random::Random;
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};

/// `snd` ids.
pub mod snd {
    pub const THROW: i16 = 300;
    pub const HIT: i16 = 4700;
}

/// The 28 faces (7 per direction), HrSp 2501….
pub const FACES: i16 = 2501;

/// The paper ball thrown with info #0 = 1.
pub const PAPER: i16 = 1051;

fn face(sp: &Sprite) -> i16 {
    FACES + sp.dir + (sp.kind - 1011) / 10 * 7
}

/// `SetupDartEnemySprite`: hot rect (8, 8, 24, 24), layer 9.
pub fn setup_dart_enemy(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.timer = 0;
    sp.task = Some(Task::DartEnemy);
    sp.hit_task = Some(HitTask::DartEnemy);
    sp.hit_tile_task = None;
    sp.hot_rect = [8, 8, 24, 24];
    sp.face = face(sp);
    sp.layer = 9;
    sp.f6e = 0;
    sp.at_from_pos();
    sp.b8b = false;
}

/// `GetDartEnemyProjOffset`: where the dart leaves the hand in throw frame
/// `dir` (1–6; 0: (−1, −1)), (h, v) from `pos`. The original leaves one
/// of the two unset for other frames and kinds; the port gives 0.
pub fn dart_offset(kind: i16, dir: i16) -> (i16, i16) {
    if dir == 0 {
        return (-1, -1);
    }
    let (mut h, mut v) = (0, 0);
    match kind {
        1011 => {
            v = 19;
            match dir {
                1 => (h, v) = (22, 21),
                2 => h = 20,
                3 => h = 18,
                4 => h = 14,
                5 => h = 13,
                6 => h = 23,
                _ => {}
            }
        }
        1021 => {
            v = 4;
            match dir {
                1 => (h, v) = (5, 0),
                2 => h = 1,
                3 => h = 6,
                4 => h = 10,
                5 => h = 11,
                6 => h = 1,
                _ => {}
            }
        }
        1031 => {
            h = 20;
            match dir {
                1 => (h, v) = (22, 2),
                2 => v = 3,
                3 => v = 6,
                4 => v = 10,
                5 => v = 11,
                6 => v = 1,
                _ => {}
            }
        }
        1041 => {
            h = 4;
            match dir {
                1 => (h, v) = (2, 22),
                2 => v = 21,
                3 => v = 16,
                4 => v = 12,
                5 => v = 11,
                6 => v = 21,
                _ => {}
            }
        }
        _ => {}
    }
    (h, v)
}

/// `ThrowDart`: a dart (kind + 7) from the hand; with a level entry its
/// info #0 decides - 0 a dart, 1 a paper ball thrown forward (1100–1699)
/// with some spread (−250…249), anything else nothing.
fn throw(sp: &Sprite, level: &Level, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    let (oh, ov) = dart_offset(sp.kind, sp.dir);
    let (h, v) = (sp.pos.1.wrapping_add(oh), sp.pos.0.wrapping_add(ov));
    let dart = SpriteEvent::Spawn { kind: sp.kind + 7, h, v, speed: None };
    let Some(e) = level.entry(sp.level_index) else {
        events.push(dart);
        return;
    };
    match e.info[0] {
        0 => events.push(dart),
        1 => {
            let speed = match sp.kind {
                1031 => {
                    let h = rng.below(500) - 250;
                    Some(Speed { h, v: -(rng.below(600) + 1100) })
                }
                1021 => {
                    let h = -(rng.below(600) + 1100);
                    Some(Speed { h, v: rng.below(500) - 250 })
                }
                1011 => {
                    let h = rng.below(600) + 1100;
                    Some(Speed { h, v: rng.below(500) - 250 })
                }
                1041 => {
                    let h = rng.below(500) - 250;
                    Some(Speed { h, v: rng.below(600) + 1100 })
                }
                _ => None,
            };
            events.push(SpriteEvent::Spawn { kind: PAPER, h, v, speed });
        }
        _ => {}
    }
}

/// `HandleDartEnemySprite`: only while `at` is within the view widened by
/// 64 px up and left (view = its top-left (v, h), 448 + 64 px): `f6e`
/// counts the frames of a throw - the arm up over frames 1–13, at 20 the
/// throw (sound 300), at 22 back; then, after a hit's pause (timer)
/// has run out and 45 frames have passed, each frame a 1 in 25 chance to
/// start again. A hit enemy (layer 10) goes back to layer 7.
pub fn handle_dart_enemy(sp: &mut Sprite, view: (i16, i16), level: &Level, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    let (vv, vh) = (i32::from(view.0), i32::from(view.1));
    // SetRect takes shorts; PtInRect: top/left inside, bottom/right not
    let r = [(vv - 64) as i16, (vh - 64) as i16, (vv + 512) as i16, (vh + 512) as i16];
    if r[0] <= sp.at.0 && sp.at.0 < r[2] && r[1] <= sp.at.1 && sp.at.1 < r[3] {
        sp.f6e = sp.f6e.wrapping_add(1);
        match sp.f6e {
            1 => sp.dir = 1,
            4 => sp.dir = 2,
            7 => sp.dir = 3,
            10 => sp.dir = 4,
            13 => sp.dir = 5,
            20 => {
                sp.dir = 6;
                events.push(SpriteEvent::Sound3D(snd::THROW));
                throw(sp, level, rng, events);
            }
            22 => sp.dir = 0,
            _ => {
                if sp.timer == 0 {
                    if sp.f6e > 45 && rng.below(25) == 2 {
                        sp.f6e = 0;
                    }
                } else {
                    sp.timer -= 1;
                }
            }
        }
        sp.face = face(sp);
    }
    if sp.layer == 10 {
        sp.layer = 7;
    }
}

/// `HitDartEnemySprite`: a player shot stops on it (sound 4700); it
/// flinches (layer 10 for a frame), pauses 65–134 frames and Harry loses
/// 3 corporate favor.
pub fn hit_dart_enemy(sp: &mut Sprite, other: &mut Sprite, g: &mut PlayerGlobals, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen || other.task != Some(Task::PlayerShot) {
        return;
    }
    other.frozen = true;
    events.push(SpriteEvent::Sound3D(snd::HIT));
    sp.layer = 10;
    g.favor = g.favor.wrapping_sub(3);
    sp.timer = rng.below(70) + 65;
}
