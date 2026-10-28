//! Stairs (9900–9909): a 32 × 32 step down. They do nothing themselves;
//! a sprite meeting them from the top side starts down them (`air` 200,
//! the flip of the stairs' kind, see `air`), from any other side it
//! bounces off.

use crate::bounce::{Body, Rect, rect_bounce};
use crate::sprite::{HitTask, Sprite, Task};

/// The faces of 9901–9904 (cicn 9901–9904).
pub const FACES: i16 = 4501;

/// `SetupStairsSprite`: layer 1, hot rect (0, 0, 32, 32). The face goes
/// by kind: 9901 / 9902 / 9903 / 9904 their own; door kinds 922 / 923
/// take 9903's and 932 / 933 9904's (the setup is never run for them);
/// other kinds none.
pub fn setup_stairs(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Stairs);
    sp.hit_task = Some(HitTask::Stairs);
    sp.hit_tile_task = None;
    sp.layer = 1;
    sp.hot_rect = [0, 0, 32, 32];
    match sp.kind {
        9901 => sp.face = FACES,
        9902 => sp.face = FACES + 1,
        9903 | 922 | 923 => sp.face = FACES + 2,
        9904 | 932 | 933 => sp.face = FACES + 3,
        _ => {}
    }
    sp.at_from_pos();
    sp.b8b = true;
}

/// `HandleStairsCollision(sprite, half, hot, stairs)`: the sprite's centre
/// (at + `half`) against the stairs' `at`. 9901 (down): moving down with
/// the centre above at.v + 22 - it starts down them when below at.v − 4
/// (`air` 200, speed h 0, `flip` the kind); 9902 (up): moving up with the
/// centre below at.v + 10 - down them when above at.v + 36; 9903 (right)
/// and 9904 (left) the same across. Otherwise `RectBounce` with `hot`.
pub fn stairs_collision(body: &mut Body, air: &mut i16, flip: &mut i16, half: i16, hot: Rect, stairs: &Sprite, bounce: i16) {
    let ch = i32::from(body.at.1.wrapping_add(half));
    let cv = i32::from(body.at.0.wrapping_add(half));
    let (sv, sh) = (i32::from(stairs.at.0), i32::from(stairs.at.1));
    let mut down = |body: &mut Body, across: bool| {
        *air = 200;
        if across {
            body.speed.v = 0;
        } else {
            body.speed.h = 0;
        }
        *flip = stairs.kind;
    };
    let entering = match stairs.kind {
        9901 => cv < sv + 22 && body.speed.v > 0,
        9902 => sv + 10 < cv && body.speed.v < 0,
        9903 => ch < sh + 22 && body.speed.h > 0,
        9904 => sh + 10 < ch && body.speed.h < 0,
        _ => return,
    };
    if entering {
        match stairs.kind {
            9901 if sv - 4 < cv => down(body, false),
            9902 if cv < sv + 36 => down(body, false),
            9903 if sh - 4 < ch => down(body, true),
            9904 if ch < sh + 36 => down(body, true),
            _ => {}
        }
    } else {
        rect_bounce(body, half, i32::from(bounce), hot, stairs.at, stairs.hot_rect);
    }
}

/// `HandleStairsCollision` for an object with its own hot rect (plant,
/// barrel, chair).
pub fn object_on_stairs(sp: &mut Sprite, half: i16, stairs: &Sprite, bounce: i16) {
    let mut body = sp.body();
    stairs_collision(&mut body, &mut sp.air, &mut sp.flip, half, sp.hot_rect, stairs, bounce);
    sp.set_body(body);
}
