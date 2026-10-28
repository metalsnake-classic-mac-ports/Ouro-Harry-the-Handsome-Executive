//! Enemy shots (`SetupEnemyShotSprite`, `HandleEnemyShotSprite`,
//! `HitEnemyShotSprite`, `HitEnemyShotTileSprite`): everything thrown or
//! fired at Harry. The kind is the shooter's projectile id:
//!
//! - 1018 / 1028 / 1038 / 1048 darts flying right, left, up, down (dart
//!   enemies 1011–1041 throw kind + 7), 1051 a tumbling paper ball;
//! - 1101–1299 homing shots (scientists): they steer at Harry once, a
//!   few frames after the start, and burst after 100 frames or on a wall
//!   or another shot;
//! - 2001–2999 a plain bolt, 2281 / 2291 spinning ones (the boss);
//! - 9450 the wall guns' shot, 9610 the vending machine's can (it rolls
//!   out, slows down and becomes the soda can item 11899);
//! - anything else (robots, turrets): flies with a 7-frame animation.

use crate::bounce::{Body, bg_tile_bounce, rect_bounce, wall_bounce};
use crate::level::Level;
use crate::mask::{Mask, mask_hit};
use crate::physics::{Speed, apply_moded_impulse, enforce_max_speed, find_desired_direction};
use crate::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};
use crate::tiles::TileHit;

/// `snd` ids.
pub mod snd {
    /// A shot hitting a collider or a sliding door.
    pub const HIT: i16 = 4701;
    /// A homing shot bursting.
    pub const BURST: i16 = 4703;
}

/// HrSp ids of the faces (`InitEnemyShotSprite`).
pub mod face {
    /// Darts 1018, 1028, 1038, 1048.
    pub const DART: i16 = 3001;
    /// The 7-frame animation of kinds below 1900, and of 1900 and up.
    pub const SHOT: i16 = 3021;
    pub const SHOT_1900: i16 = 3121;
    /// The paper ball 1051 (8 frames).
    pub const PAPER: i16 = 3041;
    /// The can 9610 (8 frames).
    pub const CAN: i16 = 3061;
    /// Homing shots (8 frames) and their burst (8 frames).
    pub const HOMING: i16 = 3081;
    pub const BURST: i16 = 3101;
    /// 2281 and 2291 (8 frames each).
    pub const SPIN_2281: i16 = 5001;
    pub const SPIN_2291: i16 = 4981;
    pub const GUN: i16 = 3141;
    /// 2001–2999.
    pub const BOLT: i16 = 4801;
}

/// The soda can item a can becomes.
pub const SODA_CAN: i16 = 11899;

fn homing(kind: i16) -> bool {
    (1101..=1299).contains(&kind)
}

/// `SetupEnemyShotSprite`: layer 11, hot rect (0, 0, 8, 8), position and
/// fix from `pos`. Darts get their speed (3328) here; kinds without a
/// branch of their own start with timer 8 and `f7c` 12 (the frame homing
/// shots steer).
pub fn setup_enemy_shot(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.face = 0;
    sp.layer = 11;
    sp.timer = 0;
    sp.f7c = 0;
    sp.task = Some(Task::EnemyShot);
    sp.hit_task = Some(HitTask::EnemyShot);
    sp.hit_tile_task = Some(HitTileTask::EnemyShot);
    sp.f78 = 0;
    sp.at_from_pos();
    sp.fix_from_at();
    sp.hot_rect = [0, 0, 8, 8];
    match sp.kind {
        2281 => sp.hot_rect = [0, 0, 12, 12],
        1038 => sp.speed = Speed { v: -0xd00, h: 0 },
        1028 => sp.speed = Speed { v: 0, h: -0xd00 },
        1018 => sp.speed = Speed { v: 0, h: 0xd00 },
        1048 => sp.speed = Speed { v: 0xd00, h: 0 },
        9450 => {
            sp.hot_rect = [0, 0, 16, 16];
            sp.face = face::GUN;
        }
        2291 => sp.hot_rect = [0, 0, 16, 16],
        9610 => sp.timer = 45,
        _ => {
            sp.timer = 8;
            sp.f7c = 12;
        }
    }
    sp.b8b = false;
}

/// The next frame of a 15-step spin (dir 0–14, face dir / 2).
fn spin(sp: &mut Sprite) {
    sp.dir += 1;
    if sp.dir > 14 {
        sp.dir = 0;
    }
}

/// `HandleEnemyShotSprite`. A shot slower than 300 on both axes for more
/// than 35 frames (`f78` counts up while slow, down while fast) is gone,
/// as is one that leaves the 4096-px level. `harry` is the centre of
/// Harry's hot rect (`SpriteCenter`), for the homing shots.
pub fn handle_enemy_shot(sp: &mut Sprite, harry: (i16, i16), events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    if sp.f7c > 0 {
        sp.f7c -= 1;
    }
    let slow = |x: i16| i32::from(x).abs() < 300;
    if slow(sp.speed.h) && slow(sp.speed.v) {
        sp.f78 = sp.f78.wrapping_add(1);
        if sp.f78 > 35 {
            sp.frozen = true;
        }
    } else if sp.f78 > 0 {
        sp.f78 -= 1;
    }
    let kind = sp.kind;
    match kind {
        1018 | 1028 | 1038 | 1048 => {
            sp.integrate();
            sp.face = face::DART + (kind - 1018) / 10;
        }
        1051 => {
            spin(sp);
            sp.integrate();
            sp.face = face::PAPER + sp.dir / 2;
        }
        2291 => {
            spin(sp);
            sp.integrate();
            sp.face = face::SPIN_2291 + sp.dir / 2;
        }
        2281 => {
            spin(sp);
            sp.integrate();
            sp.face = face::SPIN_2281 + sp.dir / 2;
        }
        2001..=2999 => {
            if sp.timer > 0 {
                sp.timer -= 1;
            }
            sp.face = face::BOLT;
            sp.integrate();
        }
        9450 => {
            sp.integrate();
            sp.face = face::GUN;
        }
        9610 => can(sp, events),
        k if !homing(k) => {
            sp.timer = sp.timer.wrapping_sub(1);
            sp.dir += 1;
            if sp.dir > 6 {
                sp.dir = 0;
            }
            sp.face = if k < 1900 { face::SHOT } else { face::SHOT_1900 } + sp.dir;
            sp.integrate();
        }
        _ if sp.timer == 0 => homing_flight(sp, harry),
        _ => burst(sp, events),
    }
    if sp.at.1 > 0x1000 || sp.at.1 < 0 || sp.at.0 < 0 || sp.at.0 > 0x1000 {
        sp.frozen = true;
    }
}

/// The can: spins (8 faces) for 45 frames, slowing (× 235 / 256) in the
/// last 20; then it is gone and a soda can item lies at at − 4.
fn can(sp: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    sp.timer = sp.timer.wrapping_sub(1);
    sp.dir += 1;
    if sp.dir > 7 {
        sp.dir = 0;
    }
    sp.face = face::CAN + sp.dir;
    sp.integrate();
    if sp.timer < 20 {
        let damp = |x: i16| ((i32::from(x) * 235) >> 8) as i16;
        sp.speed = Speed { v: damp(sp.speed.v), h: damp(sp.speed.h) };
        if sp.timer == 0 {
            sp.frozen = true;
            events.push(SpriteEvent::Spawn {
                kind: SODA_CAN,
                h: sp.at.1.wrapping_sub(4),
                v: sp.at.0.wrapping_sub(4),
                speed: None,
            });
        }
    }
}

/// A homing shot in flight: 8 faces; on the frame `f7c` reaches 1 it
/// turns toward Harry (impulse 50 in his direction, speed at most 1200)
/// and counts that frame in `f6e` - past 100 it bursts. (`f7c` reaches 1
/// once, so the shot steers once and `f6e` stays small.)
fn homing_flight(sp: &mut Sprite, harry: (i16, i16)) {
    sp.dir += 1;
    if sp.dir >= 8 || sp.dir < 0 {
        sp.dir = 0;
    }
    sp.face = face::HOMING + sp.dir;
    sp.fix.0 = sp.fix.0.wrapping_add(i32::from(sp.speed.h));
    sp.fix.1 = sp.fix.1.wrapping_add(i32::from(sp.speed.v));
    if sp.f7c == 1 {
        let me = crate::collider::centre(sp);
        let dir = find_desired_direction(harry, me);
        apply_moded_impulse(&mut sp.speed, dir, 50);
        enforce_max_speed(&mut sp.speed, 1200);
        sp.f6e = sp.f6e.wrapping_add(1);
        if sp.f6e > 100 {
            sp.timer = -16;
        }
    }
    let (h, v) = ((sp.fix.0 >> 8) as i16, (sp.fix.1 >> 8) as i16);
    sp.pos = (v, h);
    sp.at = (v, h);
}

/// A homing shot bursting (timer −16 … −3): no more hits, the burst
/// faces, sound 4703 and at − 4 on the first frame; gone at −2. (A
/// positive timer - the setup's 8 when the shooter leaves it - shows
/// faces past the burst's 8.)
fn burst(sp: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    sp.hit_task = None;
    if sp.timer == -16 {
        sp.at.1 = sp.at.1.wrapping_sub(4);
        sp.at.0 = sp.at.0.wrapping_sub(4);
        events.push(SpriteEvent::Sound3D(snd::BURST));
    }
    sp.face = face::BURST + (8 - (-i32::from(sp.timer)) / 2) as i16;
    sp.timer = sp.timer.wrapping_add(1);
    if sp.timer == -2 {
        sp.frozen = true;
    }
}

/// The masks enemy shots bounce with.
pub struct ShotMasks<'a> {
    /// The shots' mask (cicn 1299).
    pub shot: &'a Mask,
    /// The 32 wall shapes (PICT 160).
    pub walls: &'a [Mask],
    /// The background obstacle (cicn 290).
    pub obstacle: &'a Mask,
}

/// `HitEnemyShotTileSprite`. A wall (`WallBounce` with the shot's mask,
/// half 4, bounce 255; foreground kinds above 55 always count): a homing
/// shot in flight bursts, the can bounces, anything else is gone - with
/// no sound (the original's sound test asks for a kind below 2001 and
/// above 3000). On the floor only the can bounces off obstacles
/// (`BGTileBounce`, bounce 240).
pub fn hit_enemy_shot_tile(sp: &mut Sprite, hit: TileHit, level: &Level, masks: &ShotMasks) {
    let mut body = sp.body();
    let tile = hit.tile;
    if hit.foreground {
        let touched = if hit.kind < 56 {
            let wall = usize::try_from(hit.kind).ok().and_then(|k| masks.walls.get(k));
            wall_bounce(&mut body, hit.kind, tile, (4, 4), 0xff, sp.hot_rect, true, &mut |b: &mut Body| match wall {
                Some(w) => mask_hit(&mut b.pos, masks.shot, tile, w),
                None => false,
            }) != 0
        } else {
            true
        };
        sp.set_body(body);
        if touched {
            if !homing(sp.kind) || sp.timer != 0 {
                if sp.kind != 9610 {
                    sp.frozen = true;
                }
            } else {
                sp.timer = -16;
            }
        }
    } else if sp.kind == 9610 && hit.kind == 1 {
        let fg = |x: i32, y: i32| level.fg_kind(level.fg_tile(x, y));
        bg_tile_bounce(&mut body, tile, (4, 4), 0xf0, sp.hot_rect, true, &mut |b: &mut Body| {
            mask_hit(&mut b.pos, masks.shot, tile, masks.obstacle)
        }, &fg);
        sp.set_body(body);
    }
}

/// `HitEnemyShotSprite`. The can bounces off colliders, coolers,
/// plants, barrels and chairs (and grates) like `RectBounce` with half 4 and
/// bounce 255. Other shots stop at colliders and sliding doors (sound 4701) - the wall
/// guns' shot passes through the guns - and a homing shot meeting
/// another enemy or player shot bursts, taking a plain shot with it or
/// bursting a homing one too. (Robots and grates come with their kinds.)
pub fn hit_enemy_shot(sp: &mut Sprite, other: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen {
        return;
    }
    if sp.kind == 9610 {
        if matches!(other.task, Some(Task::Collider | Task::Cooler | Task::Plant | Task::Barrel | Task::Chair)) {
            let mut body = sp.body();
            rect_bounce(&mut body, 4, 0xff, sp.hot_rect, other.at, other.hot_rect);
            sp.set_body(body);
        }
    } else if matches!(other.task, Some(Task::Collider | Task::SlidingDoor)) {
        if !(other.task == Some(Task::Collider) && (9401..=9499).contains(&other.kind) && sp.kind == 9450) {
            events.push(SpriteEvent::Sound3D(snd::HIT));
            sp.frozen = true;
        }
    } else if matches!(other.task, Some(Task::EnemyShot | Task::PlayerShot)) && homing(sp.kind) {
        sp.timer = -16;
        if homing(other.kind) {
            other.timer = -16;
        } else {
            other.frozen = true;
        }
    }
}

