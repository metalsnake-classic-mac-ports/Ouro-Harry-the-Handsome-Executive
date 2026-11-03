//! Grates: 3900 (across) and 3905 (upright) break after 20 hits; the
//! fences 3910 and 3915 go out after 4 and flicker back about 7 seconds
//! later. `state` counts the hits left, then the fences' off time.

use crate::random::Random;
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};
use crate::tiles::{KICK_RECTS, offset, sect};

/// `snd` ids.
pub mod snd {
    /// Hit (`STPlay3DSoundPitched` from a shot, the kick).
    pub const HIT: i16 = 4702;
    pub const BREAK: i16 = 602;
    /// A fence going out.
    pub const OFF: i16 = 1200;
}

/// The first fence kind; below it the grates that break.
pub const FENCE: i16 = 3910;

/// Lies across (3900, 3910) rather than upright (3905, 3915).
pub fn across(kind: i16) -> bool {
    matches!(kind, 3900 | 3910)
}

/// The faces (cicn 3900, 3905).
fn face(kind: i16) -> i16 {
    if across(kind) { 3401 } else { 3402 }
}

/// `SetupGrateSprite`: layer 8; hot rect (0, −8, 6, 72) across, (−8, 0,
/// 72, 6) upright; 20 hits for a grate, 4 for a fence.
pub fn setup_grate(sp: &mut Sprite) {
    sp.init();
    sp.speed = Default::default();
    sp.dir = 0;
    sp.task = Some(Task::Grate);
    sp.hit_task = Some(HitTask::Grate);
    sp.hit_tile_task = None;
    sp.layer = 8;
    sp.hot_rect = if across(sp.kind) { [0, -8, 6, 72] } else { [-8, 0, 72, 6] };
    if matches!(sp.kind, 3900 | 3905 | 3910 | 3915) {
        sp.face = face(sp.kind);
    }
    sp.state = if sp.kind < FENCE { 20 } else { 4 };
    sp.b8b = false;
    sp.at_from_pos();
}

/// `HandleGrateSprite`. Out of hits, a grate breaks: three explosions
/// along it (the last frame 4), sound 602, 600 points. A fence goes out
/// (sound 1200, `state` −100) and counts on down: invisible from −102,
/// flickering −101 … −130 and −201 … −230 (odd frames shown), back at
/// −231 with 4 hits. A hit grate (layer 9, after its drawing) is back on
/// layer 8. It moves by its speed (whole pixels).
pub fn handle_grate(sp: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    if sp.state < 1 {
        let (h, v) = (sp.at.1, sp.at.0);
        let blasts = if across(sp.kind) {
            [(h - 16, v - 29), (h + 16, v - 29), (h, v - 29)]
        } else {
            [(h - 29, v - 16), (h - 29, v + 16), (h - 29, v)]
        };
        if sp.kind < FENCE {
            sp.frozen = true;
            for (k, (h, v)) in blasts.into_iter().enumerate() {
                events.push(SpriteEvent::Explosion { h, v, a: 0, b: if k == 2 { 4 } else { 0 } });
            }
            events.push(SpriteEvent::Sound(snd::BREAK));
            events.push(SpriteEvent::Score(600));
        } else {
            if sp.state > -100 {
                events.push(SpriteEvent::Sound(snd::OFF));
                sp.state = -100;
            }
            sp.state -= 1;
            let shown = |sp: &mut Sprite| {
                if matches!(sp.kind, 3910 | 3915) {
                    sp.face = face(sp.kind);
                }
            };
            for range in [-130..=-100, -230..=-201] {
                if range.contains(&sp.state) {
                    if sp.state & 1 == 0 {
                        sp.face = 0;
                    } else {
                        shown(sp);
                    }
                }
            }
            if sp.state < -230 {
                shown(sp);
                sp.state = 4;
            }
        }
    }
    if sp.layer == 9 {
        sp.layer = 8;
        sp.b8a = true;
    }
    sp.at = (sp.at.0.wrapping_add(sp.speed.v), sp.at.1.wrapping_add(sp.speed.h));
    sp.pos = sp.at;
}

/// `HitGrateSprite`: an explosion (still in its first frames, timer not
/// 0) costs it a hit (layer 10); Harry standing in a fence that is
/// flickering back keeps it flickering (−226 or −227).
pub fn hit_grate(sp: &mut Sprite, other: &Sprite, rng: &mut Random) {
    if sp.frozen || other.frozen {
        return;
    }
    if other.task == Some(Task::Effect) && other.timer != 0 {
        if sp.state > 0 {
            sp.state -= 1;
            sp.layer = 10;
        }
    } else if other.task == Some(Task::Player) && sp.state < -200 {
        sp.state = if rng.below(2) == 1 { -226 } else { -227 };
    }
}

/// `VBounce` / `HBounce` (`GrateBounce`: across → V): an object with
/// centre at + `half` and hot rect `hot` touching the grate is put back on
/// its side (the grate's middle line decides), its speed across reversed
/// and both × `bounce` / 256 if it was moving into it; `fix` follows a
/// moved `at`.
pub fn grate_bounce(sp: &mut Sprite, grate: &Sprite, half: i16, bounce: i32, hot: crate::bounce::Rect) {
    let g = offset(grate.hot_rect, grate.at.0, grate.at.1);
    let me = offset(hot, sp.at.0, sp.at.1);
    if !sect(me, g) {
        return;
    }
    let scale = |s: i16| ((bounce * i32::from(s)) >> 8) as i16;
    let back = |s: i16| ((-(bounce * i32::from(s))) >> 8) as i16;
    if across(grate.kind) {
        let old = sp.at.0;
        let mid = grate.at.0.wrapping_add(grate.hot_rect[0]).wrapping_add(((i32::from(grate.hot_rect[2]) - i32::from(grate.hot_rect[0])) >> 1) as i16);
        let before = sp.at.0.wrapping_add(half) < mid;
        if before && sp.speed.v > 0 || !before && sp.speed.v < 0 {
            sp.speed.h = scale(sp.speed.h);
            sp.speed.v = back(sp.speed.v);
        }
        sp.at.0 = if before { g[0].wrapping_sub(hot[2]) } else { g[2].wrapping_sub(hot[0]) };
        if old != sp.at.0 {
            sp.fix.1 = i32::from(sp.at.0) << 8;
        }
    } else {
        let old = sp.at.1;
        let mid = grate.at.1.wrapping_add(grate.hot_rect[1]).wrapping_add(((i32::from(grate.hot_rect[3]) - i32::from(grate.hot_rect[1])) >> 1) as i16);
        let before = sp.at.1.wrapping_add(half) < mid;
        if before && sp.speed.h > 0 || !before && sp.speed.h < 0 {
            sp.speed.h = back(sp.speed.h);
            sp.speed.v = scale(sp.speed.v);
        }
        sp.at.1 = if before { g[1].wrapping_sub(hot[3]) } else { g[3].wrapping_sub(hot[1]) };
        if old != sp.at.1 {
            sp.fix.0 = i32::from(sp.at.1) << 8;
        }
    }
}

/// Harry and a grate that is not out (`HitPlayerSprite`; his centre at +
/// 32, his normal hot rect `hot`): across, below the grate's top + 3
/// (by `pos`) he is kept below its bottom, else above its top - with the
/// speed across reversed, both × `bounce` / 256 - and upright the same
/// sideways. His kick (kick rect on the grate's `hot_rect2`): kick
/// contact 256, a hit, sound 4702 (returns whether it plays).
pub fn touch_grate(
    body: &mut crate::bounce::Body,
    dir: i16,
    kicking: bool,
    hot: crate::bounce::Rect,
    grate: &mut Sprite,
    bounce: i16,
    kick_contact: &mut i16,
) -> bool {
    if grate.state <= -100 {
        return false;
    }
    let b = i32::from(bounce);
    let scale = |s: i16| ((b * i32::from(s)) >> 8) as i16;
    let i = i32::from;
    let r = grate.hot_rect;
    if across(grate.kind) {
        if i(r[0]) + 3 + i(grate.pos.0) < i(body.at.0) + 32 {
            let edge = i(r[2]) + i(grate.at.0) - i(hot[0]);
            if i(body.at.0) < edge {
                body.at.0 = edge as i16;
                body.speed.v = scale(body.speed.v).wrapping_neg();
                body.speed.h = scale(body.speed.h);
                body.fix.1 = i(body.at.0) << 8;
            }
        } else {
            let edge = i(r[0]) + i(grate.at.0) - i(hot[2]);
            if edge < i(body.at.0) {
                body.at.0 = edge as i16;
                body.speed.v = scale(body.speed.v).wrapping_neg();
                body.speed.h = scale(body.speed.h);
                body.fix.1 = i(body.at.0) << 8;
            }
        }
    } else if matches!(grate.kind, 3905 | 3915) {
        if i(r[1]) + 3 + i(grate.pos.1) < i(body.at.1) + 32 {
            let edge = i(r[3]) + i(grate.at.1) - i(hot[1]);
            if i(body.at.1) < edge {
                body.at.1 = edge as i16;
                body.speed.v = scale(body.speed.v);
                body.speed.h = scale(body.speed.h).wrapping_neg();
                body.fix.0 = i(body.at.1) << 8;
            }
        } else {
            let edge = i(r[1]) + i(grate.at.1) - i(hot[3]);
            if edge < i(body.at.1) {
                body.at.1 = edge as i16;
                body.speed.v = scale(body.speed.v);
                body.speed.h = scale(body.speed.h).wrapping_neg();
                body.fix.0 = i(body.at.1) << 8;
            }
        }
    }
    if kicking {
        let kick = offset(KICK_RECTS[dir as usize], body.at.0, body.at.1);
        if sect(kick, grate.hot_rect2) {
            *kick_contact = 0x100;
            grate.state -= 1;
            return true;
        }
    }
    false
}

/// A shot hitting a grate with hits left (`HitPlayerShotSprite`): a hit,
/// layer 10, sound 4702 rising as it weakens (40000 + `SATRand(12000)` +
/// (20 − hits) × 1800, fences (4 − hits) × 8000).
pub fn player_shot_hit(grate: &mut Sprite, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    grate.state -= 1;
    grate.b8a = true;
    grate.layer = 10;
    let r = i32::from(rng.below(12000));
    let rise = if grate.kind < 3910 { (20 - i32::from(grate.state)) * 1800 } else { (4 - i32::from(grate.state)) * 8000 };
    events.push(SpriteEvent::Sound3DPitched { id: snd::HIT, rate: (r + rise + 40000) as u32 });
}
