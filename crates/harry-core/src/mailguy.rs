//! The mail guy (1601 up, 1611 down, 1621 right, 1631 left): a mail cart
//! running along a corridor at 15 px a frame. At a wall it stops, waits,
//! and some time later turns round. It runs Harry over; plants, barrels
//! and chairs blow it up (their hit tasks, `mail_guy_crash`). Its state
//! lives in its level entry, so it carries on where it was when it comes
//! back.

use crate::level::Level;
use crate::physics::Speed;
use crate::random::Random;
use crate::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};
use crate::tiles::TileHit;

/// The cart's sound as it sets off (`snd` 400).
pub const SOUND: i16 = 400;
/// Faces 3701–3704 (PICT 1601, 1611, 1621, 1631).
const FACES: i16 = 3701;
/// The faces' bounds, the hot rect: upright for 1601 / 1611, across for
/// 1621 / 1631.
const BOUNDS: [[i16; 4]; 2] = [[0, 0, 64, 34], [0, 0, 34, 64]];

fn face(kind: i16) -> i16 {
    FACES + kind.wrapping_sub(1601) / 10
}

/// `SetupMailGuySprite`: layer 9, standing, its hot rect its face's bounds,
/// not started yet (+0x8b clear).
pub fn setup_mail_guy(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::MailGuy);
    sp.hit_task = Some(HitTask::MailGuy);
    sp.hit_tile_task = Some(HitTileTask::MailGuy);
    sp.face = face(sp.kind);
    sp.hot_rect = BOUNDS[usize::from(matches!(sp.kind, 1621 | 1631))];
    sp.layer = 9;
    sp.speed = Speed::default();
    sp.at_from_pos();
    sp.b8b = false;
}

fn entry_mut(level: &mut Level, index: i16) -> Option<&mut crate::level::LevelSprite> {
    usize::try_from(index).ok().and_then(|i| level.sprites.get_mut(i))
}

/// `HandleMailGuySprite`. On its first frame it takes `timer` and `dir`
/// from its level entry's info #0 / #1. `timer` counts toward 0 (from
/// above it then stops the cart: dir + 1). `dir` 0: running its way (only
/// that speed is set); otherwise stopped, dir + 1 a frame, and from 41 on
/// a 1 in 40 chance each frame to turn round: the opposite kind, dir 0,
/// timer −9 (it cannot stop again at once), sound 400 (half as loud for
/// 1631) - with the old speed set again for this frame. Moves by its speed
/// (whole pixels, `pos` and `at`) and writes timer, dir and kind back to
/// its entry (the index unchecked; the port skips a missing one).
pub fn handle_mail_guy(sp: &mut Sprite, level: &mut Level, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    if !sp.b8b {
        if let Some(e) = entry_mut(level, sp.level_index) {
            sp.timer = e.info[0];
            sp.dir = e.info[1];
        }
        sp.b8b = true;
    }
    if sp.timer >= 1 {
        sp.timer -= 1;
        if sp.timer == 0 {
            sp.dir += 1;
        }
    } else if sp.timer < 0 {
        sp.timer += 1;
    }
    // (kind, the speed it runs at, the kind it turns into)
    let (run, back): (fn(&mut Speed), i16) = match sp.kind {
        1601 => (|s| s.v = -15, 1611),
        1611 => (|s| s.v = 15, 1601),
        1621 => (|s| s.h = 15, 1631),
        1631 => (|s| s.h = -15, 1621),
        _ => (|_| {}, 0),
    };
    if back != 0 {
        if sp.dir == 0 {
            run(&mut sp.speed);
        } else if sp.dir < 41 || rng.below(40) != 1 {
            sp.speed = Speed::default();
            sp.dir += 1;
        } else {
            let volume = if sp.kind == 1631 { 0x80 } else { 0x100 };
            sp.kind = back;
            sp.dir = 0;
            // the turn sets the old way's speed once more
            run(&mut sp.speed);
            sp.timer = -9;
            events.push(SpriteEvent::DynamicSound { id: SOUND, volume, speed: sp.speed });
        }
    }
    sp.pos = (sp.pos.0.wrapping_add(sp.speed.v), sp.pos.1.wrapping_add(sp.speed.h));
    sp.at = sp.pos;
    sp.face = face(sp.kind);
    if let Some(e) = entry_mut(level, sp.level_index) {
        e.info[0] = sp.timer;
        e.info[1] = sp.dir;
        e.id = sp.kind;
    }
}

/// `HitMailGuyTileSprite`: running (timer and dir 0) into a wall shape
/// that faces its way, it stops in 6 frames.
pub fn hit_mail_guy_tile(sp: &mut Sprite, hit: TileHit) {
    if !hit.foreground || sp.timer != 0 || sp.dir != 0 {
        return;
    }
    let way = match sp.kind {
        1601 => Way::Up,
        1611 => Way::Down,
        1621 => Way::Right,
        1631 => Way::Left,
        _ => return,
    };
    if blocks(way, hit.kind) {
        sp.timer = 6;
    }
}

/// The way a mail cart or a boulder runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Up,
    Down,
    Right,
    Left,
}

/// The wall shapes (foreground kinds) that stop something running that
/// way.
pub fn blocks(way: Way, wall: i16) -> bool {
    let walls: &[i16] = match way {
        Way::Right => &[2, 6, 7, 8, 9, 14, 15, 19, 22, 27, 30],
        Way::Down => &[3, 4, 7, 9, 10, 12, 15, 16, 23, 24, 31],
        Way::Up => &[1, 5, 6, 8, 11, 13, 14, 18, 21, 26, 29],
        Way::Left => &[0, 4, 5, 10, 11, 12, 13, 17, 20, 25, 28],
    };
    walls.contains(&wall)
}

/// `HitMailGuySprite`: shots stop on it; a robot it meets is out of hit
/// points (blobs too - with their kind).
pub fn hit_mail_guy(sp: &Sprite, other: &mut Sprite) {
    if sp.frozen || other.frozen {
        return;
    }
    if other.task == Some(Task::Robot) {
        other.state = 0;
    }
    if matches!(other.task, Some(Task::EnemyShot | Task::PlayerShot)) {
        other.frozen = true;
    }
}

/// A plant, barrel or chair (`sp`) meeting the mail guy, in their hit
/// tasks: both are gone (frozen, their level entries cleared), sounds 602
/// and 401, three explosions - kind 1 with the cart's speed, kind 0 with
/// half of it, and one at the object − 20 with its speed / 256 (frame −4)
/// - and 5000 points.
pub fn mail_guy_crash(sp: &mut Sprite, mail: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    sp.frozen = true;
    mail.frozen = true;
    events.push(SpriteEvent::Sound(602));
    events.push(SpriteEvent::Sound(401));
    let (h, v) = (mail.at.1, mail.at.0);
    events.push(SpriteEvent::MovingExplosion { h, v, speed: mail.speed, a: 1, b: 4 });
    let half = |s: i16| (0.5 * f64::from(s)) as i32 as i16;
    let speed = Speed { h: half(mail.speed.h), v: half(mail.speed.v) };
    events.push(SpriteEvent::MovingExplosion { h, v, speed, a: 0, b: 0 });
    let own = Speed { h: sp.speed.h / 256, v: sp.speed.v / 256 };
    events.push(SpriteEvent::MovingExplosion { h: sp.at.1.wrapping_sub(20), v: sp.at.0.wrapping_sub(20), speed: own, a: 0, b: -4 });
    for i in [sp.level_index, mail.level_index] {
        if (0..256).contains(&i) {
            events.push(SpriteEvent::Forget(i));
        }
    }
    events.push(SpriteEvent::Score(5000));
}
