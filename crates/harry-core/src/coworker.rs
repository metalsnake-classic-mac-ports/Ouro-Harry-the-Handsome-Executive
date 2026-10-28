//! Co-workers (1300–1449): people standing about the office. Harry and the
//! objects bounce off them, a shot costs corporate favor (`shots`); only
//! 1301 moves - now and then it goes through a little animation.

use crate::random::Random;
use crate::sprite::{Sprite, Task};

/// The faces: 1301's eleven animation frames (cicn 1301–1311), the other
/// kinds one each (1321, 1341, …: cicn of the kind).
const ANIMATED: i16 = 2101;
const STILL: i16 = 2301;

/// The face of a co-worker kind at animation frame `n` (`dir` / 3): the
/// original's table has 12 entries per group of 20 kinds - for 1301–1320
/// the eleven frames, for the next seven groups only the first.
fn face(kind: i16, n: i16) -> i16 {
    let group = kind.wrapping_sub(1301) / 20;
    match (group, n) {
        (0, 0..=10) => ANIMATED + n,
        (1..=7, 0) => STILL + group,
        _ => 0,
    }
}

/// `SetupCoWorkerSprite`: no hit task, layer 5, hot rect (2, 2, 40, 40).
pub fn setup_coworker(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::CoWorker);
    sp.hit_task = None;
    sp.hit_tile_task = None;
    sp.hot_rect = [2, 2, 40, 40];
    sp.face = face(sp.kind, 0);
    sp.layer = 5;
    sp.at_from_pos();
    sp.b8b = false;
}

/// `HandleCoWorkerSprite`. 1301 waits (each frame a 1 in 15 chance) and
/// then plays frames 1–12, holds frame 12 (a 1 in 60 chance a frame to go
/// on) and plays on to 32, then starts over; its face is frame / 3. A
/// co-worker hit by a shot (layer 10) is back on layer 5.
pub fn handle_coworker(sp: &mut Sprite, rng: &mut Random) {
    if sp.frozen {
        return;
    }
    if sp.kind == 1301 {
        match sp.dir {
            12 => {
                if rng.below(60) == 1 {
                    sp.dir += 1;
                }
            }
            0 => {
                if rng.below(15) == 1 {
                    sp.dir += 1;
                }
            }
            1..=11 | 13..=31 => sp.dir += 1,
            _ => sp.dir = 0,
        }
    }
    sp.face = face(sp.kind, sp.dir / 3);
    if sp.layer == 10 {
        sp.layer = 5;
    }
}
