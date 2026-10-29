//! Blobs: 1701 hops about at random, 1711 lunges (mostly at Harry), 1721
//! creeps after him. `state` is their hit points, `f6e` the mode: 2 sets
//! off, 0 slides and slows, 4 rests (turning on the spot).

use crate::bounce::wall_bounce;
use crate::effect::explode_radius;
use crate::level::Level;
use crate::mask::{Mask, mask_hit};
use crate::physics::{Physics, Speed, apply_moded_impulse, enforce_max_speed, inverse_mode};
use crate::random::Random;
use crate::robot::toward_harry;
use crate::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};
use crate::tiles::{KICK_RECTS, TileHit, offset, sect};

/// `snd` ids.
pub mod snd {
    /// Hit by a shot; also its half-pipe and jump sound.
    pub const HIT: i16 = 604;
    /// The last hit.
    pub const DIE: i16 = 603;
}

/// 24 faces (PICT 1701): 16 turning, then 8 dying.
const FACES: i16 = 1401;

/// `f6e` modes.
mod mode {
    pub const SLIDE: i32 = 0;
    pub const SET_OFF: i32 = 2;
    pub const REST: i32 = 4;
}

const UNIT: [f64; 10] = [1.0, 0.985, 0.94, 0.866, 0.766, 0.643, 0.5, 0.342, 0.174, 0.0];

/// `AccelerateBlob`: `m` added along `dir` (other directions nothing).
fn accelerate(sp: &mut Sprite, m: i16) {
    if !(0..36).contains(&sp.dir) {
        return;
    }
    let m = f64::from(m);
    let k = (sp.dir % 9) as usize;
    let (along, across) = (UNIT[k] * m, UNIT[9 - k] * m);
    let (dh, dv) = match sp.dir / 9 {
        0 => (along, -across),
        1 => (-across, -along),
        2 => (-along, across),
        _ => (across, along),
    };
    sp.speed.h = (f64::from(sp.speed.h) + dh) as i32 as i16;
    sp.speed.v = (f64::from(sp.speed.v) + dv) as i32 as i16;
}

/// `SetupBlobSprite`: a random direction (0–34) and wait (0–89), layer 7,
/// hot rect (4, 4, 24, 24), resting; 5 hit points (1711: 7; other kinds
/// none). A record that was on layer 10 gets +0x8a set first. The face is
/// 1401 + dir - for dir 24 and up the original reads past its 24 faces
/// (BUGS.md); the port gives none until the first frame sets one.
pub fn setup_blob(sp: &mut Sprite, rng: &mut Random) {
    if sp.layer == 10 {
        sp.b8a = true;
    }
    sp.init();
    sp.dir = rng.below(35);
    sp.task = Some(Task::Blob);
    sp.hit_task = Some(HitTask::Blob);
    sp.hit_tile_task = Some(HitTileTask::Blob);
    sp.hot_rect = [4, 4, 24, 24];
    sp.layer = 7;
    sp.speed = Speed::default();
    sp.at_from_pos();
    sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
    sp.f6e = mode::REST;
    sp.timer = rng.below(90);
    match sp.kind {
        1711 => sp.state = 7,
        1701 | 1721 => sp.state = 5,
        _ => {}
    }
    sp.face = if sp.dir < 24 { FACES + sp.dir } else { 0 };
    sp.b8b = true;
}

/// `HandleBlobSprite`. It clears the frame's blob wall flags (as the
/// robots'). Out of hit points it dies over 32 frames (faces 1416…), then
/// gives 500 / 1200 / 2500 points and leaves its level entry's info #3.
/// Otherwise, by mode: setting off - 1701 any way with 3200; 1711 at
/// Harry 2 in 3 times, else any way, with 4500; 1721 for 15 frames 350 a
/// frame, at Harry 93 in 100 times -, then sliding; sliding: × 0.7 until
/// both speeds are below 101, then resting (1701 30–89 frames, 1711
/// 15–54, 1721 5–64), 1711 and 1721 setting off again at random (1 in 20
/// / 1 in 34 a frame); resting: the timer counts down, the blob turning
/// faster in the last 40 and 20 frames, then sets off. At most 2000 on
/// each axis; the face shows dir / 2 of 16. Half pipes (centre + 16,
/// sound 604) and, in the air, jumps. `player` is Harry's top-left (h, v).
#[allow(clippy::too_many_arguments)]
pub fn handle_blob(
    sp: &mut Sprite,
    level: &Level,
    player: (i16, i16),
    drop: i16,
    rng: &mut Random,
    wall_done: &mut bool,
    obstacle_done: &mut bool,
    events: &mut Vec<SpriteEvent>,
    sounds: &mut Vec<crate::air::AirSound>,
) {
    if sp.frozen {
        return;
    }
    *obstacle_done = false;
    *wall_done = false;
    if sp.state < 1 {
        if sp.timer >= 0 {
            sp.timer = -999;
        }
        sp.timer -= 1;
        sp.dir = -1000 - sp.timer;
        sp.face = FACES + sp.dir / 4 + 15;
        if sp.dir > 31 {
            sp.frozen = true;
            let points = match sp.kind {
                1711 => 1200,
                1701 => 500,
                1721 => 2500,
                _ => 0,
            };
            events.push(SpriteEvent::Score(points));
            if (0..256).contains(&sp.level_index) && drop != 0 {
                events.push(SpriteEvent::Generate { kind: drop, h: sp.at.1, v: sp.at.0 });
            }
        }
    } else {
        let (rest, restart): ((i16, i16), Option<i16>) = match sp.kind {
            1711 => ((40, 15), Some(950)),
            1701 => ((60, 30), None),
            1721 => ((60, 5), Some(970)),
            _ => ((0, 0), None),
        };
        if matches!(sp.kind, 1701 | 1711 | 1721) {
            match sp.f6e {
                mode::SET_OFF => match sp.kind {
                    1711 => {
                        sp.dir = if rng.below(100) < 66 { toward_harry(sp, player) } else { rng.below(35) };
                        accelerate(sp, 4500);
                        sp.f6e = mode::SLIDE;
                    }
                    1701 => {
                        sp.dir = rng.below(35);
                        accelerate(sp, 3200);
                        sp.f6e = mode::SLIDE;
                    }
                    _ => {
                        if sp.timer == 0 {
                            sp.timer = 15;
                        }
                        sp.timer -= 1;
                        sp.dir = if rng.below(100) < 93 { toward_harry(sp, player) } else { rng.below(35) };
                        accelerate(sp, 350);
                        if sp.timer == 1 {
                            sp.f6e = mode::SLIDE;
                        }
                    }
                },
                mode::SLIDE => {
                    if i32::from(sp.speed.h).abs() < 101 && i32::from(sp.speed.v).abs() < 101 {
                        sp.f6e = mode::REST;
                        sp.speed = Speed::default();
                        sp.timer = rng.below(rest.0) + rest.1;
                    } else {
                        sp.speed.h = (f64::from(sp.speed.h) * 0.7) as i32 as i16;
                        sp.speed.v = (f64::from(sp.speed.v) * 0.7) as i32 as i16;
                        if restart.is_some_and(|r| rng.below(1000) > r) {
                            sp.f6e = mode::SET_OFF;
                        }
                    }
                }
                mode::REST => {
                    if sp.timer < 1 {
                        sp.f6e = mode::SET_OFF;
                    } else {
                        sp.timer -= 1;
                        if sp.timer < 40 {
                            sp.dir += 1;
                        }
                        if sp.timer < 20 {
                            sp.dir += 1;
                        }
                    }
                }
                _ => {}
            }
        }
        enforce_max_speed(&mut sp.speed, 2000);
        sp.integrate();
        let d = sp.dir / 2;
        sp.face = FACES + d - (d / 16) * 16;
    }
    let mut body = sp.body();
    let mut air = sp.air_state();
    crate::air::half_pipe(&mut body, &mut air, 16, false, level, sounds);
    if air.air > 0 {
        let mut unused_comfort = 0;
        crate::air::jump_scale(&mut body, &mut air, &mut sp.dir, 16, false, level, rng, &mut unused_comfort, 0, sounds);
    }
    sp.set_body(body);
    sp.set_air_state(air);
}

/// Bounce back: the move undone, × −1.1, sliding.
fn recoil(sp: &mut Sprite) {
    sp.fix.0 = sp.fix.0.wrapping_sub(i32::from(sp.speed.h));
    sp.fix.1 = sp.fix.1.wrapping_sub(i32::from(sp.speed.v));
    sp.speed.h = (f64::from(sp.speed.h) * -1.1) as i32 as i16;
    sp.speed.v = (f64::from(sp.speed.v) * -1.1) as i32 as i16;
    sp.f6e = mode::SLIDE;
}

/// `HitBlobSprite`. An explosion pushes it (centre + 16) and costs a hit
/// point (layer 10). Doors (open ones too), plants, barrels, coolers,
/// grates, boulders, sliding doors, chairs, co-workers, colliders: it
/// bounces back. Harry's kick (kick rect on its hot rect): sound 600, a
/// hit point, layer 10, half the robots' kick thrust away from him, kick
/// contact 64.
pub fn hit_blob(sp: &mut Sprite, other: &Sprite, physics: &Physics, kick_contact: &mut i16, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen {
        return;
    }
    match other.task {
        Some(Task::Effect) => {
            if other.hot_rect != [0; 4] {
                explode_radius(sp.at, &mut sp.speed, &mut sp.blast_cooldown, 16, other);
                sp.state -= 1;
                sp.layer = 10;
            }
        }
        Some(
            Task::Door
            | Task::Plant
            | Task::Barrel
            | Task::Cooler
            | Task::Grate
            | Task::Boulder
            | Task::SlidingDoor
            | Task::Chair
            | Task::CoWorker
            | Task::Collider,
        ) => recoil(sp),
        Some(Task::Player) if other.hot_rect[1] == 0 => {
            let kick = offset(KICK_RECTS[other.dir as usize], other.at.0, other.at.1);
            if sect(kick, offset(sp.hot_rect, sp.at.0, sp.at.1)) {
                events.push(SpriteEvent::Sound3D(crate::robot::snd::HIT));
                sp.state -= 1;
                sp.layer = 10;
                let thrust = ((i32::from(physics.kick) * 26) >> 2) as i16;
                apply_moded_impulse(&mut sp.speed, inverse_mode(other.dir), thrust);
                *kick_contact = 0x40;
            }
        }
        _ => {}
    }
}

/// `HitBlobTileSprite`: walls (`WallBounce`, centre + 16, bounce 255,
/// mask cicn 598; a bounce makes it slide) and, on the floor, obstacles
/// (it bounces back) - the first blob of the frame only, each.
#[allow(clippy::too_many_arguments)]
pub fn hit_blob_tile(
    sp: &mut Sprite,
    hit: TileHit,
    mask: &Mask,
    walls: &[Mask],
    wall_done: &mut bool,
    obstacle_done: &mut bool,
) {
    if hit.foreground {
        if *wall_done {
            return;
        }
        let tile = hit.tile;
        let hot = sp.hot_rect;
        let mut body = sp.body();
        let wall = usize::try_from(hit.kind).ok().and_then(|k| walls.get(k));
        let r = wall_bounce(&mut body, hit.kind, tile, (16, 16), 0xff, hot, true, &mut |b| match wall {
            Some(w) => mask_hit(&mut b.pos, mask, tile, w),
            None => false,
        });
        sp.set_body(body);
        if r as i8 != 0 {
            *wall_done = true;
            sp.f6e = mode::SLIDE;
        }
    } else if sp.air == 0 && hit.kind == 1 && !*obstacle_done {
        *obstacle_done = true;
        recoil(sp);
    }
}

/// A player shot hitting a blob with hit points left
/// (`HitPlayerShotSprite`): layer 10, a point, `f6e` 0, sound 604 (and
/// 603 for the last); the shot is gone.
pub fn shot_hit(blob: &mut Sprite, events: &mut Vec<SpriteEvent>) -> bool {
    if blob.state <= 0 {
        return false;
    }
    blob.layer = 10;
    blob.state -= 1;
    blob.f6e = 0;
    events.push(SpriteEvent::Sound(snd::HIT));
    if blob.state == 0 {
        events.push(SpriteEvent::Sound(snd::DIE));
    }
    true
}

/// The blob's draw proc: a hit blob (layer 10) is drawn lit and goes back
/// to layer 7.
pub fn drawn(sp: &mut Sprite) {
    if sp.layer == 10 {
        sp.layer = 7;
        sp.b8a = true;
    }
}

