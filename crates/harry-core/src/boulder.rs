//! Boulders (6001 up, 6002 down, 6003 left, 6004 right; 6011–6014 the
//! faster ones): they roll along a corridor, stop at a wall and, after a
//! while, start again from where they began. They run Harry over.

use crate::level::Level;
use crate::mailguy::{Way, blocks};
use crate::random::Random;
use crate::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};

/// The rolling sound (`snd` 402), at a random pitch.
pub const SOUND: i16 = 402;
/// 12 faces (PICT 6001): rolling up / down 1601…, sideways 1607….
const FACES: i16 = 1601;

fn way(kind: i16) -> Option<Way> {
    match kind {
        6001 | 6011 => Some(Way::Up),
        6002 | 6012 => Some(Way::Down),
        6003 | 6013 => Some(Way::Left),
        6004 | 6014 => Some(Way::Right),
        _ => None,
    }
}

fn upright(kind: i16) -> bool {
    matches!(kind, 6001 | 6002 | 6011 | 6012)
}

/// `SetupBoulderSprite`: layer 200 (on top), hot rect (16, 16, 48, 48),
/// no face yet; `state` keeps its starting coordinate (v rolling up or
/// down, else h); not started (+0x8b clear).
pub fn setup_boulder(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.f6e = 0;
    sp.task = Some(Task::Boulder);
    sp.hit_task = Some(HitTask::Boulder);
    sp.hit_tile_task = Some(HitTileTask::Boulder);
    sp.face = 0;
    sp.hot_rect = [16, 16, 48, 48];
    sp.layer = 200;
    sp.speed = Default::default();
    sp.at_from_pos();
    sp.state = if upright(sp.kind) { sp.pos.0 } else { sp.pos.1 };
    sp.timer = 0;
    sp.b8b = false;
}

/// `HandleBoulderSprite`. On its first frame it takes `timer` and `dir`
/// from its level entry (info #0, #1) and its start from info #2 (or
/// writes its own there). The timer counting down to 0 stops it (dir +
/// 1). dir 0: rolling 8 px a frame (6011–6014: 12) its way; stopped:
/// standing, dir + 1, and from 4 on a 1 in 5 chance a frame to start over
/// from its start (sound 402 at a random pitch). It moves in whole
/// pixels; timer, dir and kind go back to its entry (index unchecked; the
/// port skips a missing one); the face turns with `f6e` (0–5).
pub fn handle_boulder(sp: &mut Sprite, level: &mut Level, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    let up = upright(sp.kind);
    if !sp.b8b {
        if let Some(e) = level.entry_mut(sp.level_index) {
            sp.timer = e.info[0];
            if e.info[2] == 0 {
                e.info[2] = sp.state;
            } else {
                sp.state = e.info[2];
                if up {
                    sp.pos.0 = sp.state;
                } else {
                    sp.pos.1 = sp.state;
                }
            }
            sp.dir = e.info[1];
        }
        sp.b8b = true;
    }
    if sp.timer > 0 {
        sp.timer -= 1;
        if sp.timer == 0 {
            sp.dir += 1;
        }
    }
    if let Some(w) = way(sp.kind) {
        if sp.dir == 0 {
            let s = if sp.kind < 6010 { 8 } else { 12 };
            match w {
                Way::Up => sp.speed.v = -s,
                Way::Down => sp.speed.v = s,
                Way::Left => sp.speed.h = -s,
                Way::Right => sp.speed.h = s,
            }
        } else if sp.dir < 4 || rng.below(5) != 1 {
            sp.speed = Default::default();
            sp.dir += 1;
        } else {
            if up {
                sp.pos.0 = sp.state;
            } else {
                sp.pos.1 = sp.state;
            }
            sp.dir = 0;
            let pitch = rng.below(6000) as u32;
            events.push(SpriteEvent::Sound3DPitched { id: SOUND, at: sp.at, rate: pitch + 62000 });
        }
    }
    sp.pos = (sp.pos.0.wrapping_add(sp.speed.v), sp.pos.1.wrapping_add(sp.speed.h));
    sp.at = sp.pos;
    if let Some(e) = level.entry_mut(sp.level_index) {
        e.info[0] = sp.timer;
        e.info[1] = sp.dir;
        e.id = sp.kind;
    }
    sp.f6e += 1;
    if sp.f6e > 5 {
        sp.f6e = 0;
    }
    let f = sp.f6e as i16;
    if let Some(w) = way(sp.kind) {
        sp.face = FACES
            + match w {
                Way::Up => f,
                Way::Down => 5 - f,
                Way::Left => f + 6,
                Way::Right => 11 - f,
            };
    }
}

/// `HitBoulderTileSprite`: rolling (timer and dir 0) into a wall shape
/// that faces its way, it stops in 12 frames.
pub fn hit_boulder_tile(sp: &mut Sprite, hit: crate::tiles::TileHit) {
    if !hit.foreground || sp.timer != 0 || sp.dir != 0 {
        return;
    }
    if way(sp.kind).is_some_and(|w| blocks(w, hit.kind)) {
        sp.timer = 12;
    }
}

/// `HitBoulderSprite`: a robot or blob it rolls over is out of hit
/// points.
pub fn hit_boulder(sp: &Sprite, other: &mut Sprite) {
    if sp.frozen || other.frozen {
        return;
    }
    if matches!(other.task, Some(Task::Robot | Task::Blob)) {
        other.state = 0;
    }
}
