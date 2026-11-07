//! Ramps (9701–9704): wedges on the floor that sprites ride up and over
//! (`SetupRampSprite`, `HandleRampSprite`, `HandleRampCollision`). 9701
//! and 9702 lie along v, 9703 and 9704 along h; a ramp whose level entry
//! has info #1 set can be pushed around.

use crate::bounce::{Body, Half, Rect, bg_tile_bounce2, rect_bounce_fake, wall_bounce};
use crate::level::Level;
use crate::mask::{Mask, mask_hit};
use crate::tiles::TileHit;
use crate::physics::{Physics, handle_glide};
use crate::sprite::{HitTask, HitTileTask, Sprite, Task};

/// `snd` id: a sprite crashing into a ramp's high end and taking off.
pub const LAUNCH_SOUND: i16 = 209;

/// The faces (`InitRampSprite`): still 4101–4104, pushable 4105–4108.
pub const FACES: i16 = 4101;
pub const MOVING_FACES: i16 = 4105;

/// Hot rect of each kind (the setup's, set again by every routine).
pub fn hot_rect(kind: i16) -> Option<Rect> {
    match kind {
        9701 => Some([8, 0, 60, 24]),
        9702 => Some([0, 0, 52, 24]),
        9703 => Some([0, 8, 24, 60]),
        9704 => Some([0, 0, 24, 52]),
        _ => None,
    }
}

/// `SetupRampSprite`: layer 1, fix from `pos`, speed 0.
pub fn setup_ramp(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Ramp);
    sp.hit_task = Some(HitTask::Ramp);
    sp.hit_tile_task = Some(HitTileTask::Ramp);
    if let Some(r) = hot_rect(sp.kind) {
        sp.face = FACES + (sp.kind - 9701);
        sp.hot_rect = r;
    }
    sp.at_from_pos();
    sp.fix_from_at();
    sp.layer = 1;
    sp.speed = Default::default();
    sp.b8b = true;
}

/// `HandleRampSprite`: a pushable ramp (entry info #1 ≠ 0) moves by its
/// speed and shows the pushable face; every ramp then glides
/// (`HandleGlide`, centre + 20).
pub fn handle_ramp(sp: &mut Sprite, level: &Level, physics: &Physics) {
    if sp.frozen {
        return;
    }
    let pushable = level.entry(sp.level_index).is_some_and(|e| e.info[1] != 0);
    if pushable {
        sp.integrate();
        if let Some(r) = hot_rect(sp.kind) {
            sp.face = MOVING_FACES + (sp.kind - 9701);
            sp.hot_rect = r;
        }
    }
    handle_glide(&mut sp.speed, sp.fix, sp.air, sp.slide, 20, level, physics);
}

/// The sprite riding a ramp: its body (`pos`, `fix`, `speed`), jump state,
/// the ramp cooldown (+0x94, counted down by `EraseSprites`), and whether
/// it is Harry (he climbs with 30, others with 15).
pub struct Rider<'a> {
    pub body: &'a mut Body,
    pub air: &'a mut i16,
    pub cooldown: &'a mut i16,
    pub player: bool,
}

fn halve(x: i16) -> i16 {
    x / 2
}

/// `-(bounce × speed >> 9)`: half the level's bounce, reversed.
fn rebound(bounce: i16, s: i16) -> i16 {
    ((i32::from(bounce) * i32::from(s)) >> 9).wrapping_neg() as i16
}

/// `HandleRampCollision` (PPC 0x10021170): a sprite (centre at fix + `half`,
/// hot rect `hot`) touching a ramp. Not within 3 frames of the last bounce.
/// Coming at the ramp across its length (more than 8 px past the middle
/// line, moving toward it) it bounces back (half the level's bounce) and
/// pushes the ramp with half its speed. Otherwise, at the low end and on
/// it, it is drawn toward the middle line (1 px, speed across halved) and
/// pushed up the slope (speed +30 / +15 along it); at the high end it
/// stops on the edge - or, faster than 450 against it, takes off (`air`
/// 1, sound 209); hitting the far end's back it bounces. Returns whether
/// the sound plays. The four kinds differ in small ways: 9703 moves only
/// `fix` back at its back end, 9704 only `pos`.
pub fn handle_ramp_collision(r: &mut Rider, half: i16, hot: Rect, ramp: &mut Sprite, bounce: i16) -> bool {
    if *r.cooldown >= 1 {
        return false;
    }
    let b = &mut *r.body;
    let ch = ((b.fix.0 >> 8) as i16).wrapping_add(half);
    let cv = ((b.fix.1 >> 8) as i16).wrapping_add(half);
    b.pos = ((b.fix.1 >> 8) as i16, (b.fix.0 >> 8) as i16);
    ramp.pos = ((ramp.fix.1 >> 8) as i16, (ramp.fix.0 >> 8) as i16);
    let rh = &ramp.hot_rect;
    let (rl, rr) = (rh[1].wrapping_add(ramp.pos.1), rh[3].wrapping_add(ramp.pos.1));
    let (rt, rb) = (rh[0].wrapping_add(ramp.pos.0), rh[2].wrapping_add(ramp.pos.0));
    let (ml, mr) = (b.pos.1.wrapping_add(hot[1]), b.pos.1.wrapping_add(hot[3]));
    let (mt, mb) = (b.pos.0.wrapping_add(hot[0]), b.pos.0.wrapping_add(hot[2]));
    let i = i32::from;
    let climb: i16 = if r.player { 30 } else { 15 };
    let mid_v = (i(rt) + i(rb)) >> 1;
    let mid_h = (i(rl) + i(rr)) >> 1;
    // coming at it across: (centre, its speed, the middle line)
    let across = |c: i16, s: i16, mid: i32| (i(c) < mid && s > 0 && i(c) < mid - 8) || (mid <= i(c) && s < 0 && mid + 8 <= i(c));
    // drawn 1 px toward the middle line, the speed across halved
    let toward = |pos: &mut i16, fix: &mut i32, s: &mut i16, c: i16, mid: i32| {
        if i(c) < mid {
            *pos = pos.wrapping_add(1);
        } else if mid < i(c) {
            *pos = pos.wrapping_sub(1);
        } else {
            return;
        }
        *fix = i(*pos) << 8;
        *s = halve(*s);
    };
    let mut sound = false;
    let mut launch = |air: &mut i16| {
        sound = true;
        *air = 1;
    };
    match ramp.kind {
        9703 | 9704 => {
            let mut hit = across(cv, b.speed.v, mid_v);
            if hit && i(rr) - 13 < i(ml) {
                hit = false;
            }
            if !hit {
                if ramp.kind == 9703 {
                    if i(rl) + 15 < i(mr) {
                        toward(&mut b.pos.0, &mut b.fix.1, &mut b.speed.v, cv, mid_v);
                        b.speed.h = b.speed.h.wrapping_add(climb);
                    } else if b.speed.h < 0 {
                        if b.speed.h < -450 {
                            launch(r.air);
                        } else {
                            b.speed.h = 0;
                            b.pos.1 = b.pos.1.wrapping_add(rl.wrapping_add(16).wrapping_sub(mr));
                            b.fix.0 = i(b.pos.1) << 8;
                        }
                    } else if i(mr) <= i(rl) + 6 {
                        b.fix.0 = b.fix.0.wrapping_add((i(rl) - i(mr) + 1).wrapping_mul(-0x100));
                        b.speed.h = rebound(bounce, b.speed.h);
                        *r.cooldown = 3;
                    }
                } else if i(ml) < i(rr) - 15 {
                    toward(&mut b.pos.0, &mut b.fix.1, &mut b.speed.v, cv, mid_v);
                    b.speed.h = b.speed.h.wrapping_sub(climb);
                } else if b.speed.h < 1 {
                    if i(rr) - 6 <= i(ml) {
                        b.pos.1 = b.pos.1.wrapping_add(ml.wrapping_sub(rr)).wrapping_add(1);
                        b.speed.h = rebound(bounce, b.speed.h);
                        *r.cooldown = 3;
                    }
                } else if b.speed.h < 451 {
                    b.speed.h = 0;
                    b.pos.1 = b.pos.1.wrapping_sub(ml.wrapping_sub(rr.wrapping_sub(16)));
                    b.fix.0 = i(b.pos.1) << 8;
                } else {
                    launch(r.air);
                }
            }
            if hit {
                if b.speed.v >= 1 {
                    b.pos.0 = b.pos.0.wrapping_sub(mb.wrapping_sub(rt).wrapping_add(1));
                } else if b.speed.v < 0 {
                    b.pos.0 = b.pos.0.wrapping_add(rb.wrapping_sub(mt)).wrapping_add(1);
                }
                b.fix.1 = i(b.pos.0) << 8;
                ramp.speed.v = ramp.speed.v.wrapping_add(b.speed.v >> 1);
                b.speed.v = rebound(bounce, b.speed.v);
                *r.cooldown = 3;
            }
        }
        9701 | 9702 => {
            let mut hit = across(ch, b.speed.h, mid_h);
            if hit && (if ramp.kind == 9701 { i(rb) - 13 < i(mt) } else { i(mb) < i(rt) + 13 }) {
                hit = false;
            }
            if !hit {
                if ramp.kind == 9701 {
                    if i(rt) + 15 < i(mb) {
                        toward(&mut b.pos.1, &mut b.fix.0, &mut b.speed.h, ch, mid_h);
                        b.speed.v = b.speed.v.wrapping_add(climb);
                    } else if b.speed.v < 0 {
                        if b.speed.v < -450 {
                            launch(r.air);
                        } else {
                            b.speed.v = 0;
                            b.pos.0 = b.pos.0.wrapping_add(rt.wrapping_add(16).wrapping_sub(mb));
                            b.fix.1 = i(b.pos.0) << 8;
                        }
                    } else if i(mb) <= i(rt) + 6 {
                        b.pos.0 = b.pos.0.wrapping_sub(rt.wrapping_sub(mb).wrapping_add(1));
                        b.fix.1 = i(b.pos.0) << 8;
                        b.speed.v = rebound(bounce, b.speed.v);
                        *r.cooldown = 3;
                    }
                } else if i(mt) < i(rb) - 15 {
                    toward(&mut b.pos.1, &mut b.fix.0, &mut b.speed.h, ch, mid_h);
                    b.speed.v = b.speed.v.wrapping_sub(climb);
                } else if b.speed.v < 1 {
                    if i(rb) - 6 <= i(mt) {
                        b.pos.0 = b.pos.0.wrapping_add(rb.wrapping_sub(mt)).wrapping_add(1);
                        b.fix.1 = i(b.pos.0) << 8;
                        b.speed.v = rebound(bounce, b.speed.v);
                        *r.cooldown = 3;
                    }
                } else if b.speed.v < 451 {
                    b.speed.v = 0;
                    b.pos.0 = b.pos.0.wrapping_sub(mt.wrapping_sub(rb.wrapping_sub(16)));
                    b.fix.1 = i(b.pos.0) << 8;
                } else {
                    launch(r.air);
                }
            }
            if hit {
                if b.speed.h >= 1 {
                    b.pos.1 = b.pos.1.wrapping_sub(mr.wrapping_sub(rl).wrapping_add(1));
                } else if b.speed.h < 0 {
                    b.pos.1 = b.pos.1.wrapping_add(rr.wrapping_sub(ml)).wrapping_add(1);
                }
                b.fix.0 = i(b.pos.1) << 8;
                ramp.speed.h = ramp.speed.h.wrapping_add(b.speed.h >> 1);
                b.speed.h = rebound(bounce, b.speed.h);
                *r.cooldown = 3;
            }
        }
        _ => {}
    }
    sound
}

/// The offset to a ramp's centre, (v, h) (the routines' local points).
pub fn half(kind: i16) -> Option<Half> {
    match kind {
        9701 => Some((34, 12)),
        9702 => Some((26, 12)),
        9703 => Some((12, 34)),
        9704 => Some((12, 26)),
        _ => None,
    }
}

/// `HitRampSprite`: a ramp meeting another ramp bounces off it with
/// `RectBounceFake2` (the level's bounce) - but the original sets this
/// ramp's hot rect to the *other* ramp's kind's and takes the obstacle
/// rect from the other's centre instead of its `at` (BUGS.md). Other
/// kinds have the centre (0, 0) (a constant in the data) and keep their
/// hot rect.
pub fn hit_ramp(sp: &mut Sprite, other: &Sprite, bounce: i16) {
    if sp.frozen || other.frozen {
        return;
    }
    let own = half(sp.kind).unwrap_or((0, 0));
    if let Some(rect) = hot_rect(sp.kind) {
        sp.hot_rect = rect;
    }
    if other.task != Some(Task::Ramp) {
        return;
    }
    let mut point = other.at;
    if let (Some(h), Some(r)) = (half(other.kind), hot_rect(other.kind)) {
        point = (point.0.wrapping_add(h.0), point.1.wrapping_add(h.1));
        sp.hot_rect = r;
    }
    let mut body = sp.body();
    rect_bounce_fake(&mut body, point, other.hot_rect, own, i32::from(bounce), sp.hot_rect);
    sp.set_body(body);
}

/// The masks pushable ramps bounce with.
pub struct RampMasks<'a> {
    /// The four ramps' masks (cicn 571–574 for 9701–9704).
    pub ramps: [&'a Mask; 4],
    /// The 32 wall shapes (PICT 160).
    pub walls: &'a [Mask],
    /// The background obstacle (cicn 290).
    pub obstacle: &'a Mask,
}

/// `HitRampTileSprite`: walls (`WallBounce2`, only with a face) and
/// background obstacles (`BGTileBounce2`), with the ramp's mask, centre
/// and the level's bounce, always as the frame's first contact.
pub fn hit_ramp_tile(sp: &mut Sprite, hit: TileHit, level: &Level, masks: &RampMasks, bounce: i16) {
    let (Some(h), Some(rect)) = (half(sp.kind), hot_rect(sp.kind)) else { return };
    sp.hot_rect = rect;
    let own = masks.ramps[(sp.kind - 9701) as usize];
    let tile = hit.tile;
    let mut body = sp.body();
    if hit.foreground {
        if hit.kind < 56 && sp.face != 0 {
            let wall = usize::try_from(hit.kind).ok().and_then(|k| masks.walls.get(k));
            wall_bounce(&mut body, hit.kind, tile, h, bounce, sp.hot_rect, true, &mut |b: &mut Body| match wall {
                Some(w) => mask_hit(&mut b.pos, own, tile, w),
                None => false,
            });
        }
    } else if hit.kind == 1 {
        let fg = |x: i32, y: i32| level.fg_kind(level.fg_tile(x, y));
        bg_tile_bounce2(&mut body, tile, h, bounce, sp.hot_rect, true, &mut |b: &mut Body| {
            mask_hit(&mut b.pos, own, tile, masks.obstacle)
        }, &fg);
    }
    sp.set_body(body);
}
