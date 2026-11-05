//! Barrels (550) and office chairs (560): objects Harry kicks and pushes
//! about like the plants (`kinds`), with their own hit tasks. A barrel
//! catches fire when kicked hard, shot or near a flame and explodes; a
//! chair spins.

use crate::air::{AirSound, half_pipe, jump_scale};
use crate::bounce::rect_bounce;
use crate::effect::{FIRE, explode_radius, pe_distance};
use crate::kinds::{door_open, plus_scaled, scaled};
use crate::level::Level;
use crate::mailguy::mail_guy_crash;
use crate::mask::{Mask, mask_hit};
use crate::physics::{Physics, Speed, apply_moded_impulse, find_desired_direction, handle_glide, inverse_mode};
use crate::ramp::{Rider, handle_ramp_collision};
use crate::random::Random;
use crate::stairs::object_on_stairs;
use crate::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};
use crate::tiles::{KICK_RECTS, offset, sect};

/// `snd` ids.
pub mod snd {
    /// The barrel bumping into something, landing, bouncing.
    pub const BARREL_BUMP: i16 = 800;
    /// The barrel catching fire.
    pub const BARREL_FIRE: i16 = 801;
    /// The chair kicked, landing, bouncing.
    pub const CHAIR_KICK: i16 = 257;
    /// The chair bumping into something, off a half pipe.
    pub const CHAIR_BUMP: i16 = 258;
}

/// Barrel faces: whole and burning (cicn 550 / 551).
pub const BARREL: i16 = 1301;
pub const BARREL_BURNING: i16 = 1302;
/// Chair faces: 36 directions (PICT 560).
pub const CHAIR: i16 = 1801;

/// The masks the barrel's and chair's hit tasks use.
pub struct FurnitureMasks<'a> {
    /// Harry's body (cicn 299).
    pub body: &'a Mask,
    /// Plant and chair (cicn 599).
    pub plant: &'a Mask,
    /// Barrel and water cooler (cicn 598).
    pub barrel: &'a Mask,
}

fn rounded(s: Speed) -> Speed {
    // `(s >> 8)` rounded toward 0
    Speed { h: s.h / 256, v: s.v / 256 }
}

fn clear_entry(level: &mut Level, index: i16) {
    if let Some(e) = usize::try_from(index).ok().filter(|&i| i < 256).and_then(|i| level.sprites.get_mut(i)) {
        e.present = false;
    }
}

/// `SetupBarrelSprite`: layer 6, hot rect (0, 0, 24, 24), dir 1 (whole).
/// `at` is left as it was.
pub fn setup_barrel(sp: &mut Sprite) {
    sp.init();
    sp.dir = 1;
    sp.layer = 6;
    sp.task = Some(Task::Barrel);
    sp.hit_task = Some(HitTask::Barrel);
    sp.hit_tile_task = Some(HitTileTask::Barrel);
    sp.speed = Speed::default();
    sp.hot_rect = [0, 0, 24, 24];
    sp.face = BARREL;
    sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
    sp.b8b = true;
}

/// `HandleBarrelSprite`: the timer (barrels touching each other) counts
/// down. A burning barrel (dir 2…) flickers between its faces for 17
/// frames - its level entry is cleared, it will not come back - then
/// explodes (dir 19: four moving explosions, the view shakes for 32
/// frames), shows the burning face once more and is gone. It glides, rides
/// half pipes and jumps like a plant (centre at + 12). `shake` is the view
/// shake counter; `unset` the scale `JumpScale` leaves on landing in a
/// hole (r30 = the record's address in the original, see BUGS.md).
#[allow(clippy::too_many_arguments)]
pub fn handle_barrel(
    sp: &mut Sprite,
    level: &mut Level,
    physics: &Physics,
    rng: &mut Random,
    shake: &mut i16,
    unset: i16,
    events: &mut Vec<SpriteEvent>,
    sounds: &mut Vec<AirSound>,
) {
    if sp.frozen {
        return;
    }
    if sp.timer > 0 {
        sp.timer -= 1;
    }
    match sp.dir {
        2 | 4 | 6 | 8 | 10 | 12 | 14 | 16 | 18 => {
            sp.face = BARREL_BURNING;
            sp.dir += 1;
        }
        3 | 5 | 7 | 9 | 11 | 13 | 15 | 17 => {
            sp.face = BARREL;
            sp.dir += 1;
            clear_entry(level, sp.level_index);
        }
        19 => {
            *shake = 0x20;
            let speed = rounded(sp.speed);
            let (h, v) = (sp.pos.1, sp.pos.0);
            for (dh, dv, b) in [(-20, -20, 0), (-20, 0, -3), (-20, -36, -8), (0, -20, -5)] {
                events.push(SpriteEvent::MovingExplosion { h: h.wrapping_add(dh), v: v.wrapping_add(dv), speed, a: 0, b });
            }
            sp.dir += 1;
        }
        // the switch falls through into case 30
        20 | 21 | 23..=29 => {
            sp.face = BARREL_BURNING;
            sp.dir += 1;
            sp.frozen = true;
        }
        30 => sp.frozen = true,
        _ => sp.face = BARREL,
    }
    handle_glide(&mut sp.speed, sp.fix, sp.air, sp.slide, 12, level, physics);
    let mut body = sp.body();
    let mut air = sp.air_state();
    let mut s = Vec::new();
    half_pipe(&mut body, &mut air, 12, false, level, &mut s);
    // HandleHalfPipe with sound 0: none
    s.clear();
    let mut unused_comfort = 0;
    jump_scale(&mut body, &mut air, &mut sp.dir, 12, false, level, rng, &mut unused_comfort, unset, &mut s);
    sounds.extend(s);
    sp.set_body(body);
    sp.set_air_state(air);
    sp.integrate();
}

/// The barrel catches fire (dir 1 → 2) with its sound.
fn ignite(sp: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    if sp.dir == 1 {
        events.push(SpriteEvent::Sound(snd::BARREL_FIRE));
        sp.dir += 1;
    }
}

fn bump(bump_playing: bool, id: i16, events: &mut Vec<SpriteEvent>) {
    if !bump_playing {
        events.push(SpriteEvent::Sound(id));
    }
}

/// Both trade momentum (0.4 each way), then both × −0.4.
fn trade(a: &mut Speed, b: &mut Speed) {
    b.h = plus_scaled(b.h, 0.4, a.h);
    b.v = plus_scaled(b.v, 0.4, a.v);
    a.h = plus_scaled(a.h, 0.4, b.h);
    a.v = plus_scaled(a.v, 0.4, b.v);
    a.h = scaled(-0.4, a.h);
    a.v = scaled(-0.4, a.v);
    b.h = scaled(-0.4, b.h);
    b.v = scaled(-0.4, b.v);
}

fn back(s: &mut Speed) {
    s.h = scaled(-0.4, s.h);
    s.v = scaled(-0.4, s.v);
}

fn ride_ramp(sp: &mut Sprite, half: i16, ramp: &mut Sprite, physics: &Physics) {
    let mut body = sp.body();
    let hot = sp.hot_rect;
    let mut r = Rider { body: &mut body, air: &mut sp.air, cooldown: &mut sp.f94, player: false };
    // the sound given is 0: none
    handle_ramp_collision(&mut r, half, hot, ramp, physics.bounce);
    sp.set_body(body);
}

/// `HitBarrelSprite` (both on the floor). Harry's kick sends it away with
/// 13 × the kick thrust and lights it half the time (sound 801 at a
/// random pitch); walking into it he is pushed out, it takes 0.6 of his
/// speed, he keeps −0.3 - and on caffeine it lights. Two barrels touching
/// count up the timer by 8 (past 25 they light), are pushed apart and
/// bounce (× −0.4, then each adds the other's speed). Coolers, plants and
/// chairs: pushed out, with a plant or chair trading momentum. Doors (not
/// open), sliding doors, colliders: `RectBounce`; ramps carry it. Shots stop on it and
/// light it, the big shot (9450) pushes; explosions push it, and flames
/// light it; stairs take it down; a mail guy crashes into it
/// (`mail_guy_crash`); grates: `GrateBounce`. Desks: always a hit
/// (no mask), it bounces back × −0.4 as from a cooler.
#[allow(clippy::too_many_arguments)]
pub fn hit_barrel(
    sp: &mut Sprite,
    other: &mut Sprite,
    masks: &FurnitureMasks,
    physics: &Physics,
    rng: &mut Random,
    kick_contact: &mut i16,
    caffeine: bool,
    bump_playing: bool,
    events: &mut Vec<SpriteEvent>,
) {
    if sp.frozen || other.frozen || sp.air >= 1 || other.air >= 1 {
        return;
    }
    if other.task == Some(Task::Player) {
        if other.hot_rect[1] == 0 {
            let kick = offset(KICK_RECTS[other.dir as usize], other.at.0, other.at.1);
            if sect(kick, offset(sp.hot_rect, sp.at.0, sp.at.1)) {
                let thrust = ((i32::from(physics.kick) * 26) >> 1) as i16;
                apply_moded_impulse(&mut sp.speed, inverse_mode(other.dir), thrust);
                *kick_contact = 0x80;
                if rng.below(100) < 50 && sp.dir == 1 {
                    let pitch = rng.below(6000) as u32;
                    events.push(SpriteEvent::Sound3DPitched { id: snd::BARREL_FIRE, rate: pitch + 62000 });
                    sp.dir += 1;
                } else {
                    bump(bump_playing, snd::BARREL_BUMP, events);
                }
            }
        } else if mask_hit(&mut other.pos, masks.body, sp.at, masks.barrel) {
            other.fix = (i32::from(other.pos.1) << 8, i32::from(other.pos.0) << 8);
            bump(bump_playing, snd::BARREL_BUMP, events);
            sp.speed.h = scaled(0.6, other.speed.h);
            sp.speed.v = scaled(0.6, other.speed.v);
            other.speed.h = scaled(-0.3, other.speed.h);
            other.speed.v = scaled(-0.3, other.speed.v);
            if caffeine {
                ignite(sp, events);
            }
        }
    } else if other.task == Some(Task::MailGuy) {
        mail_guy_crash(sp, other, events);
    }
    if other.task == Some(Task::Barrel) {
        sp.timer = sp.timer.wrapping_add(8);
        if sp.timer > 25 {
            ignite(sp, events);
        }
        sp.pos = sp.at;
        other.pos = other.at;
        if mask_hit(&mut other.pos, masks.barrel, sp.at, masks.barrel) {
            other.fix = (i32::from(other.pos.1) << 8, i32::from(other.pos.0) << 8);
            sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
            bump(bump_playing, snd::BARREL_BUMP, events);
            back(&mut sp.speed);
            back(&mut other.speed);
            sp.speed.h = sp.speed.h.wrapping_add(other.speed.h);
            sp.speed.v = sp.speed.v.wrapping_add(other.speed.v);
            other.speed.h = other.speed.h.wrapping_add(sp.speed.h);
            other.speed.v = other.speed.v.wrapping_add(sp.speed.v);
        }
    }
    if other.task == Some(Task::Ramp) {
        ride_ramp(sp, 12, other, physics);
    }
    if other.task == Some(Task::Stairs) {
        object_on_stairs(sp, 12, other, physics.bounce);
    }
    if matches!(other.task, Some(Task::Cooler | Task::Plant | Task::Chair | Task::Desk)) {
        let other_mask = if other.task == Some(Task::Cooler) { masks.barrel } else { masks.plant };
        // a desk: always a hit, no mask test (its step back, fix − speed,
        // is overwritten by the fix from pos)
        if other.task == Some(Task::Desk) || mask_hit(&mut sp.pos, masks.barrel, other.at, other_mask) {
            sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
            bump(bump_playing, snd::BARREL_BUMP, events);
            if matches!(other.task, Some(Task::Cooler | Task::Desk)) {
                back(&mut sp.speed);
            } else {
                trade(&mut sp.speed, &mut other.speed);
            }
        }
    }
    let door = other.task == Some(Task::Door) && !door_open(other.kind);
    if door || matches!(other.task, Some(Task::Collider | Task::SlidingDoor | Task::CoWorker)) {
        let mut body = sp.body();
        rect_bounce(&mut body, 12, i32::from(physics.bounce), sp.hot_rect, other.at, other.hot_rect);
        sp.set_body(body);
    }
    if other.task == Some(Task::Grate) {
        crate::grate::grate_bounce(sp, other, 12, i32::from(physics.bounce), sp.hot_rect);
    }
    let effect = other.task == Some(Task::Effect);
    if matches!(other.task, Some(Task::PlayerShot | Task::EnemyShot)) || effect {
        if other.kind == 9610 {
            return;
        }
        if effect {
            explode_radius(sp.at, &mut sp.speed, &mut sp.blast_cooldown, 18, other);
        } else {
            other.frozen = true;
        }
        ignite(sp, events);
        if other.kind == 9450 {
            sp.speed.h = sp.speed.h.wrapping_add(other.speed.h >> 2);
            sp.speed.v = sp.speed.v.wrapping_add(other.speed.v >> 2);
        }
    }
    if effect && other.kind >= FIRE {
        // the flame: +0x6e = 0, +0x70 = 3
        other.f6e = 3;
        ignite(sp, events);
    }
}

/// `SetupChairSprite`: a random direction (0–34), layer 7, hot rect (8, 8,
/// 28, 28); `f6e` is the direction × 256, `timer` the spin.
pub fn setup_chair(sp: &mut Sprite, rng: &mut Random) {
    sp.init();
    sp.dir = rng.below(35);
    sp.f6e = i32::from(sp.dir) << 8;
    sp.timer = 0;
    sp.speed = Speed::default();
    sp.layer = 7;
    sp.task = Some(Task::Chair);
    sp.hit_task = Some(HitTask::Chair);
    sp.hit_tile_task = Some(HitTileTask::Chair);
    sp.hot_rect = [8, 8, 28, 28];
    sp.face = CHAIR;
    sp.at_from_pos();
    sp.fix_from_at();
}

/// `HandleChairSprite`: it spins by `timer` (1/256 of a direction a
/// frame, slowing × 0.9), its face shows the direction; then it moves like
/// a plant (centre at + 18).
#[allow(clippy::too_many_arguments)]
pub fn handle_chair(sp: &mut Sprite, level: &Level, physics: &Physics, rng: &mut Random, unset: i16, sounds: &mut Vec<AirSound>) {
    if sp.frozen {
        return;
    }
    sp.f6e = sp.f6e.wrapping_add(i32::from(sp.timer));
    sp.timer = (f64::from(sp.timer) * 0.9) as i32 as i16;
    while sp.f6e > 0x23ff {
        sp.f6e -= 0x2400;
    }
    while sp.f6e < 0 {
        sp.f6e += 0x2400;
    }
    sp.dir = ((sp.f6e as u32) >> 8) as i16;
    sp.dir = sp.dir.clamp(0, 35);
    sp.face = CHAIR + sp.dir;
    handle_glide(&mut sp.speed, sp.fix, sp.air, sp.slide, 18, level, physics);
    let mut body = sp.body();
    let mut air = sp.air_state();
    half_pipe(&mut body, &mut air, 18, false, level, sounds);
    let mut unused_comfort = 0;
    jump_scale(&mut body, &mut air, &mut sp.dir, 18, false, level, rng, &mut unused_comfort, unset, sounds);
    sp.set_body(body);
    sp.set_air_state(air);
    sp.integrate();
    sp.b8b = true;
}

/// The turn between two directions (0–35) the short way, as the chair
/// takes it: `b − a`, or `b + 36 − a` when that is 19 or more apart.
fn turn(a: i16, b: i16) -> i16 {
    let d = b.wrapping_sub(a);
    if i32::from(d).abs() < 19 { d } else { b.wrapping_add(36).wrapping_sub(a) }
}

/// `HitChairSprite` (both on the floor). Explosions push it. Harry's kick
/// (sound 257) sends it away with 13 × the kick thrust and spins it by how
/// much the kick turns its path (at most 9 directions × an eighth of its
/// new speed); walking into it he is pushed out, it takes 0.6 of his
/// speed, he keeps −0.3, and it spins by the angle between his path and
/// the chair × his speed / 32. Coolers, barrels, chairs: pushed out, a
/// barrel or chair trading momentum; desks: always a hit, back × −0.4.
/// Doors (not open), sliding doors, colliders: `RectBounce`; ramps carry it, stairs take it down. Shots stop on it
/// (sound 258).
#[allow(clippy::too_many_arguments)]
pub fn hit_chair(
    sp: &mut Sprite,
    other: &mut Sprite,
    masks: &FurnitureMasks,
    physics: &Physics,
    kick_contact: &mut i16,
    bump_playing: bool,
    events: &mut Vec<SpriteEvent>,
) {
    if sp.frozen || other.frozen || sp.air >= 1 || other.air >= 1 {
        return;
    }
    match other.task {
        Some(Task::Effect) => {
            if other.hot_rect != [0; 4] {
                explode_radius(sp.at, &mut sp.speed, &mut sp.blast_cooldown, 18, other);
            }
        }
        Some(Task::MailGuy) => mail_guy_crash(sp, other, events),
        Some(Task::Player) => {
            let harry = (other.at.0.wrapping_add(32), other.at.1.wrapping_add(32));
            let chair = (sp.at.0.wrapping_add(16), sp.at.1.wrapping_add(16));
            if other.hot_rect[1] == 0 {
                let kick = offset(KICK_RECTS[other.dir as usize], other.at.0, other.at.1);
                if sect(kick, offset(sp.hot_rect, sp.at.0, sp.at.1)) {
                    events.push(SpriteEvent::Sound3D(snd::CHAIR_KICK));
                    let old = sp.speed;
                    sp.speed = Speed::default();
                    let thrust = ((i32::from(physics.kick) * 26) >> 1) as i16;
                    apply_moded_impulse(&mut sp.speed, inverse_mode(other.dir), thrust);
                    *kick_contact = 0x80;
                    let path = (harry.0.wrapping_add(sp.speed.v), harry.1.wrapping_add(sp.speed.h));
                    let a = find_desired_direction(harry, chair);
                    let b = find_desired_direction(harry, path);
                    let speed = (pe_distance(sp.speed.h, sp.speed.v) as i16) >> 3;
                    let d = turn(a, b).clamp(-9, 9);
                    sp.timer = sp.timer.wrapping_sub(d.wrapping_mul(speed));
                    sp.speed.h = sp.speed.h.wrapping_add(old.h);
                    sp.speed.v = sp.speed.v.wrapping_add(old.v);
                }
            } else if mask_hit(&mut other.pos, masks.body, sp.at, masks.plant) {
                sp.dir = 1;
                other.at = other.pos;
                other.fix_from_at();
                sp.fix_from_at();
                bump(bump_playing, snd::CHAIR_BUMP, events);
                sp.speed.h = scaled(0.6, other.speed.h);
                sp.speed.v = scaled(0.6, other.speed.v);
                other.speed.h = scaled(-0.3, other.speed.h);
                other.speed.v = scaled(-0.3, other.speed.v);
                let harry = (other.at.0.wrapping_add(32), other.at.1.wrapping_add(32));
                let path = (harry.0.wrapping_add(other.speed.v), harry.1.wrapping_add(other.speed.h));
                let a = find_desired_direction(harry, chair);
                let b = find_desired_direction(harry, path);
                let speed = pe_distance(other.speed.h, other.speed.v) as i16;
                sp.timer = sp.timer.wrapping_add(turn(a, b).wrapping_mul(speed >> 5));
            }
        }
        _ => {}
    }
    if matches!(other.task, Some(Task::Cooler | Task::Barrel | Task::Chair | Task::Desk)) {
        let other_mask = if other.task == Some(Task::Chair) { masks.plant } else { masks.barrel };
        // a desk: always a hit, as for the barrel
        if other.task == Some(Task::Desk) || mask_hit(&mut sp.pos, masks.plant, other.at, other_mask) {
            sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
            bump(bump_playing, snd::CHAIR_BUMP, events);
            if matches!(other.task, Some(Task::Cooler | Task::Desk)) {
                back(&mut sp.speed);
            } else {
                trade(&mut sp.speed, &mut other.speed);
            }
        }
    }
    let door = other.task == Some(Task::Door) && !door_open(other.kind);
    if door || matches!(other.task, Some(Task::Collider | Task::SlidingDoor | Task::CoWorker)) {
        let mut body = sp.body();
        rect_bounce(&mut body, 18, i32::from(physics.bounce), sp.hot_rect, other.at, other.hot_rect);
        sp.set_body(body);
    }
    if other.task == Some(Task::Grate) {
        crate::grate::grate_bounce(sp, other, 18, i32::from(physics.bounce), sp.hot_rect);
    }
    if other.task == Some(Task::Ramp) {
        ride_ramp(sp, 18, other, physics);
    }
    if other.task == Some(Task::Stairs) {
        object_on_stairs(sp, 18, other, physics.bounce);
    }
    if matches!(other.task, Some(Task::PlayerShot | Task::EnemyShot)) {
        other.frozen = true;
        events.push(SpriteEvent::Sound3D(snd::CHAIR_BUMP));
        sp.dir = 1;
        if other.kind == 9450 {
            sp.speed.h = sp.speed.h.wrapping_add(other.speed.h >> 2);
            sp.speed.v = sp.speed.v.wrapping_add(other.speed.v >> 2);
        }
    }
}
