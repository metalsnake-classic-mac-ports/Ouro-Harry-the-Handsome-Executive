//! Reversors (1851 / 1861 rolling across, 1852 / 1862 up and down; the
//! 186x faster): spinning balls that bounce off everything and send
//! shots back.

use crate::bounce::{FirstBounce, TileMasks, first_obstacle, first_wall_bounce, obstacle_bounce};
use crate::ramp::{Rider, handle_ramp_collision};
use crate::level::Level;
use crate::physics::{Physics, wrap_dir};
use crate::random::Random;
use crate::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};
use crate::stairs::object_on_stairs;
use crate::tiles::TileHit;

/// A shot sent back (`snd` 605).
pub const SOUND: i16 = 605;

/// `SetupReversorSprite`: layer 7, hot rect (4, 4, 24, 24), rolling at
/// 1000 (1851 right, 1852 down) or 1500 (1861, 1862); the small robots'
/// faces.
pub fn setup_reversor(sp: &mut Sprite) {
    sp.init();
    sp.timer = 25;
    sp.dir = 0;
    sp.task = Some(Task::Reversor);
    sp.hit_task = Some(HitTask::Reversor);
    sp.hit_tile_task = Some(HitTileTask::Reversor);
    sp.hot_rect = [4, 4, 24, 24];
    sp.layer = 7;
    sp.speed = Default::default();
    sp.at_from_pos();
    sp.fix_from_pos();
    match sp.kind {
        1861 => sp.speed.h = 1500,
        1852 => sp.speed.v = 1000,
        1851 => sp.speed.h = 1000,
        1862 => sp.speed.v = 1500,
        _ => {}
    }
    sp.state = 3;
    sp.face = 4201 + sp.dir;
    sp.b8b = true;
}

/// `HandleReversorSprite`: it spins (dir + 1, face), clears the frame's
/// reversor wall flags (as the robots, `PTR_DAT_10066188` / `…84`),
/// moves (24.8), rides half pipes and jumps (centre + 12, sound 600).
pub fn handle_reversor(
    sp: &mut Sprite,
    level: &Level,
    rng: &mut Random,
    done: &mut FirstBounce,
    sounds: &mut Vec<crate::air::AirSound>,
) {
    if sp.frozen {
        return;
    }
    sp.dir += 1;
    sp.dir = wrap_dir(sp.dir);
    *done = FirstBounce::default();
    sp.face = 4201 + sp.dir;
    sp.integrate();
    let mut body = sp.body();
    let mut air = sp.air_state();
    let mut s = Vec::new();
    crate::air::half_pipe(&mut body, &mut air, 12, false, level, &mut s);
    let mut unused_comfort = 0;
    crate::air::jump_scale(&mut body, &mut air, &mut sp.dir, 12, false, level, rng, &mut unused_comfort, 0, sounds);
    sp.set_body(body);
    sp.set_air_state(air);
}

/// `HitReversorSprite`. A barrel: it lights (sound 801, the barrel's dir
/// goes up by one whatever it was) and the reversor turns back. Ramps carry it,
/// stairs take it down. Plants, coolers, desks, grates, chairs, doors (not
/// open), reversors, co-workers, colliders, sliding doors: move
/// undone, turned back. A shot: its move undone and sent back (sound
/// 605), and the reversor turns a quarter (its larger speed component
/// goes to the other axis).
pub fn hit_reversor(sp: &mut Sprite, other: &mut Sprite, physics: &Physics, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen {
        return;
    }
    let door = other.task == Some(Task::Door) && !crate::kinds::door_open(other.kind);
    match other.task {
        Some(Task::Barrel) => {
            events.push(SpriteEvent::sound_3d(crate::furniture::snd::BARREL_FIRE, other.at));
            other.dir += 1;
            sp.speed.h = sp.speed.h.wrapping_neg();
            sp.speed.v = sp.speed.v.wrapping_neg();
        }
        Some(Task::Ramp) => {
            let mut body = sp.body();
            let hot = sp.hot_rect;
            let mut r = Rider { body: &mut body, air: &mut sp.air, cooldown: &mut sp.ramp_cooldown, player: false };
            if handle_ramp_collision(&mut r, 12, hot, other, physics.bounce) {
                events.push(SpriteEvent::Sound(crate::robot::snd::HIT));
            }
            sp.set_body(body);
        }
        Some(Task::Stairs) => object_on_stairs(sp, 12, other, physics.bounce),
        Some(
            Task::Plant | Task::Cooler | Task::Desk | Task::Grate | Task::Chair | Task::Reversor | Task::CoWorker | Task::Collider
            | Task::SlidingDoor,
        ) => turn_back(sp),
        _ if door => turn_back(sp),
        Some(Task::PlayerShot | Task::EnemyShot) => {
            other.fix.0 = other.fix.0.wrapping_sub(i32::from(other.speed.h));
            other.fix.1 = other.fix.1.wrapping_sub(i32::from(other.speed.v));
            other.speed.h = other.speed.h.wrapping_neg();
            other.speed.v = other.speed.v.wrapping_neg();
            events.push(SpriteEvent::sound_3d(SOUND, sp.centre()));
            if i32::from(sp.speed.v).abs() < i32::from(sp.speed.h).abs() {
                sp.speed.v = sp.speed.h;
                sp.speed.h = 0;
            } else {
                sp.speed.h = sp.speed.v;
                sp.speed.v = 0;
            }
        }
        _ => {}
    }
}

fn turn_back(sp: &mut Sprite) {
    sp.fix.0 = sp.fix.0.wrapping_sub(i32::from(sp.speed.h));
    sp.fix.1 = sp.fix.1.wrapping_sub(i32::from(sp.speed.v));
    sp.speed.h = sp.speed.h.wrapping_neg();
    sp.speed.v = sp.speed.v.wrapping_neg();
}

/// `HitReversorTileSprite`: walls (`WallBounce`, centre + 16, bounce 256;
/// a bounce clears `f6e`) and, on the floor, obstacles (`BGTileBounce`,
/// 256) with the robot mask - only the first reversor of the frame each.
pub fn hit_reversor_tile(sp: &mut Sprite, hit: TileHit, m: &TileMasks, done: &mut FirstBounce) {
    if hit.foreground {
        if first_wall_bounce(sp, hit, (16, 16), 0x100, m.mask, m.walls, done) {
            sp.f6e = 0;
        }
    } else if first_obstacle(sp, hit, done) {
        obstacle_bounce(sp, hit.tile, (16, 16), 0x100, m);
    }
}
