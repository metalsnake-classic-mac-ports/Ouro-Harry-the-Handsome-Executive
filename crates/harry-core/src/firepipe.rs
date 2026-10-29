//! Fire pipes (4201 up, 4202 down, 4203 right, 4204 left): now and then a
//! flame shoots out (an effect, `effect`), which hurts Harry and lights
//! barrels.

use crate::random::Random;
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};

/// The flame's sound (`snd` 700).
pub const SOUND: i16 = 700;

/// The pipes' faces (cicn 4201–4204), given to `AddIdleSprite`.
pub fn face(kind: i16) -> Option<i16> {
    matches!(kind, 4201..=4204).then_some(6041 + kind - 4201)
}

/// `SetupFirePipeSprite`: layer 0, no hit task of its own (an empty one),
/// a random start of its wait (`f6e`, 0–99).
pub fn setup_fire_pipe(sp: &mut Sprite, rng: &mut Random) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::FirePipe);
    sp.hit_task = Some(HitTask::FirePipe);
    sp.hit_tile_task = None;
    sp.layer = 0;
    sp.f6e = i32::from(rng.below(100));
    sp.at_from_pos();
    sp.b8b = true;
}

/// `HandleFirePipeSprite`: `f6e` counts up; past 90 each frame has a 1
/// in 40 chance of a flame - made with the pipe's kind at its mouth
/// (4201: at + (−3, −49) (h, v); 4202: (−4, 16); 4203: (16, −2); 4204:
/// (−49, −2); other kinds would take garbage), the kind of the level
/// entry's info #1 (0 without an entry) and `f7c` the pipe's kind; the
/// count starts over and sound 700 plays.
pub fn handle_fire_pipe(sp: &mut Sprite, info1: Option<i16>, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    sp.f6e = sp.f6e.wrapping_add(1);
    if sp.f6e <= 90 || rng.below(40) != 1 {
        return;
    }
    let (dh, dv) = match sp.kind {
        4203 => (16, -2),
        4201 => (-3, -49),
        4202 => (-4, 16),
        4204 => (-49, -2),
        _ => (0, 0),
    };
    events.push(SpriteEvent::Flame {
        pipe: sp.kind,
        h: sp.at.1.wrapping_add(dh),
        v: sp.at.0.wrapping_add(dv),
        kind: info1.unwrap_or(0),
    });
    sp.f6e = 0;
    events.push(SpriteEvent::Sound3D(SOUND));
}
