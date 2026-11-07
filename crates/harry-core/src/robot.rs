//! Robots: the small ones (1801 drives at Harry, 1811 a turret that only
//! turns, 1821 a faster driver) and the big machine-gun robots (2001,
//! 2011, 2021). `state` is their hit points, `f7c` their mode. Within
//! 500 px of Harry they turn toward him, drive and shoot; at no hit points
//! they explode, give points and may leave something behind (their level
//! entry's info #3).

use crate::bounce::{FirstBounce, TileMasks, first_obstacle, first_wall_bounce, obstacle_bounce};
use crate::effect::explode_radius;
use crate::frame::PlayerGlobals;
use crate::kinds::door_open;
use crate::level::Level;
use crate::physics::{Physics, Speed, apply_moded_impulse, find_desired_direction, inverse_mode, wrap_dir, wrap_dir_35};
use crate::ramp::{Rider, handle_ramp_collision};
use crate::random::Random;
use crate::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};
use crate::stairs::object_on_stairs;
use crate::tiles::{KICK_RECTS, TileHit, offset, sect};

/// `snd` ids.
pub mod snd {
    /// Hit; also its landing and bouncing.
    pub const HIT: i16 = 600;
    pub const SHOT: i16 = 601;
    pub const EXPLODE: i16 = 602;
    pub const MACHINE_GUN: i16 = 606;
}

/// The small robots' 36 faces (PICT 212), the big ones' 72 (PICT 2001: 36
/// directions, then 36 firing).
const SMALL_FACES: i16 = 4201;
const BIG_FACES: i16 = 4701;

/// The first big robot kind.
pub const BIG: i16 = 2001;

/// `f7c` modes of the small robots.
mod mode {
    /// Slowing down.
    pub const BRAKE: i16 = 0;
    /// Turning toward Harry.
    pub const TURN: i16 = 1;
    /// Driving at him.
    pub const DRIVE: i16 = 2;
    pub const DEAD: i16 = 3;
    /// The turret: facing him (big robots: aiming).
    pub const AIM: i16 = 4;
    /// Kicked: waits for the timer.
    pub const KICKED: i16 = 5;
    /// Big robots: firing.
    pub const FIRE: i16 = 6;
}

/// The magnitudes of a step in each tenth of a quarter turn (`cos` of
/// 0°, 10°, … 90° × the step), as the original's tables have them.
const ACCEL: [f64; 10] = [20.0, 19.7, 18.8, 17.32, 15.32, 12.86, 10.0, 6.840000000000001, 3.4799999999999995, 0.0];
pub(crate) const SHOT_SPEED: [f64; 10] = [1500.0, 1477.5, 1410.0, 1299.0, 1149.0, 964.5, 750.0, 513.0, 261.0, 0.0];

/// `AccelerateRobot` / `AssignRobotShotSpeed`: add a step of the table in
/// direction `dir` (0 = right, counterclockwise; other values nothing).
pub(crate) fn add_polar(speed: &mut Speed, dir: i16, table: &[f64; 10]) {
    if !(0..36).contains(&dir) {
        return;
    }
    let k = (dir % 9) as usize;
    let (along, across) = (table[k], table[9 - k]);
    let (dh, dv) = match dir / 9 {
        0 => (along, -across),
        1 => (-across, -along),
        2 => (-along, across),
        _ => (across, along),
    };
    let add = |s: i16, d: f64| (f64::from(s) + d) as i32 as i16;
    speed.h = add(speed.h, dh);
    speed.v = add(speed.v, dv);
}

/// `FindDesiredDirection` (robots): from the robot (pos + 14) toward
/// Harry (`gPlayerH/V` + 32).
pub(crate) fn toward_harry(sp: &Sprite, player: (i16, i16)) -> i16 {
    let harry = (player.1.wrapping_add(32), player.0.wrapping_add(32));
    let me = (sp.pos.0.wrapping_add(14), sp.pos.1.wrapping_add(14));
    find_desired_direction(me, harry)
}

/// `RobotTooFar`: Harry 500 px or more away on either axis.
fn too_far(sp: &Sprite, player: (i16, i16)) -> bool {
    let d = |a: i16, b: i16| a.wrapping_sub(b).wrapping_abs();
    !(d(player.0, sp.at.1) < 501 && d(player.1, sp.at.0) < 501)
}

/// `ApproachMode`: one step toward `dir` the short way round.
pub(crate) fn approach(sp: &mut Sprite, dir: i16) {
    if dir == sp.dir {
        return;
    }
    let d = sp.dir.wrapping_sub(dir);
    let mut up = d < 0;
    if d.wrapping_abs() > 18 {
        up = !up;
    }
    sp.dir += if up { 1 } else { -1 };
    sp.dir = wrap_dir(sp.dir);
}

/// `SetupRobotSprite`: facing Harry, timer 25, layer 7, hot rect (4, 4,
/// 24, 24). 1801: mode turn, 3 hit points; 1811 (and 1802–1813): the
/// turret, its hit points kind − 1808 (3 for 1811), kind 1811; 1821 (and
/// up to 2000): mode turn, hit points kind − 1814 (7), kind 1821. Big
/// robots: mode aim, hot rect (4, 4, 36, 36), 8 / 12 / 15 hit points for
/// 2001 / 2011 / 2021 (3 for others).
pub fn setup_robot(sp: &mut Sprite, player: (i16, i16)) {
    sp.init();
    sp.timer = 25;
    sp.dir = toward_harry(sp, player);
    sp.task = Some(Task::Robot);
    sp.hit_task = Some(HitTask::Robot);
    sp.hit_tile_task = Some(HitTileTask::Robot);
    sp.hot_rect = [4, 4, 24, 24];
    sp.layer = 7;
    sp.speed = Speed::default();
    sp.at_from_pos();
    sp.fix_from_pos();
    let mut hp = 3;
    if sp.kind < BIG {
        sp.face = SMALL_FACES + sp.dir;
        if sp.kind < 1802 {
            sp.f7c = mode::TURN;
        } else if sp.kind < 1812 {
            hp = sp.kind - 1808;
            sp.kind = 1811;
            sp.f7c = mode::AIM;
        } else {
            hp = sp.kind - 1814;
            sp.kind = 1821;
            sp.f7c = mode::TURN;
        }
    } else {
        sp.face = BIG_FACES + sp.dir;
        hp = match sp.kind {
            2011 => 12,
            2001 => 8,
            2021 => 15,
            _ => 3,
        };
        sp.f7c = mode::AIM;
        sp.hot_rect = [4, 4, 36, 36];
    }
    sp.state = hp;
    sp.b8b = true;
}

/// What a robot's task touches besides itself.
pub struct RobotWorld<'a> {
    pub level: &'a mut Level,
    pub g: &'a mut PlayerGlobals,
    pub rng: &'a mut Random,
    /// `PTR_DAT_1006619c` / `PTR_DAT_10066198`: a robot already bounced
    /// off a wall / an obstacle - cleared by every robot's task, so in the
    /// tile pass after the tasks only the first robot to touch one bounces.
    pub done: &'a mut FirstBounce,
    /// How many machine-gun sounds (606) play: from two on, a bolt is
    /// silent (and draws no pitch).
    pub mg_playing: usize,
}

/// `RandomRobotShot`: 1 in 60 (1801) or 1 in 35 a frame, a shot of the
/// robot's kind from pos + (8, 7) with its speed and direction plus 1500
/// that way; sound 601.
fn random_shot(sp: &Sprite, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if rng.below(if sp.kind < 1802 { 60 } else { 35 }) == 1 {
        let mut speed = sp.speed;
        add_polar(&mut speed, sp.dir, &SHOT_SPEED);
        events.push(SpriteEvent::Shot { kind: sp.kind, h: sp.pos.1.wrapping_add(7), v: sp.pos.0.wrapping_add(8), speed, dir: sp.dir, fix: None, to_fix: false, timer: None, f7c: None });
        events.push(SpriteEvent::sound_3d(snd::SHOT, sp.at));
    }
}

/// `RandomMGShot`: 3 in 5 a frame, a bolt from at + 16: it starts 3
/// frames' flight out along the direction 2 to one side (the wrap uses
/// 35, so 36 and −1 stay and add nothing), then flies with the robot's
/// speed and direction (now and then one off) plus 1500; sound 606 at a
/// random pitch - only when fewer than two are playing.
fn random_mg_shot(sp: &Sprite, rng: &mut Random, mg_playing: usize, events: &mut Vec<SpriteEvent>) {
    if rng.below(100) <= 39 {
        return;
    }
    let (h, v) = (sp.at.1.wrapping_add(16), sp.at.0.wrapping_add(16));
    let dir = wrap_dir_35(sp.dir + if rng.below(100) < 50 { 2 } else { -2 });
    let mut out = Speed::default();
    add_polar(&mut out, dir, &SHOT_SPEED);
    let fix = (
        (i32::from(h) << 8).wrapping_add(i32::from(out.h) * 3),
        (i32::from(v) << 8).wrapping_add(i32::from(out.v) * 3),
    );
    let mut dir = sp.dir;
    if rng.below(100) < 10 {
        dir += 1;
    } else if rng.below(100) < 20 {
        dir -= 1;
    }
    let mut speed = sp.speed;
    add_polar(&mut speed, dir, &SHOT_SPEED);
    events.push(SpriteEvent::Shot { kind: sp.kind, h, v, speed, dir, fix: Some(fix), to_fix: true, timer: None, f7c: None });
    if mg_playing < 2 {
        let pitch = rng.below(6000) as u32;
        events.push(SpriteEvent::Sound3DPitched { id: snd::MACHINE_GUN, at: sp.at, rate: pitch + 62000 });
    }
}

/// The death: frozen, an explosion (the small ones' at pos − 20 with
/// their speed / 256, the big ones' at pos − 12 with speed >> 8), sound
/// 602 (at 2/3 volume in the original), Harry quiet for 20 frames, and what the level entry's info #3
/// names is made where it was.
fn explode(sp: &mut Sprite, w: &mut RobotWorld, events: &mut Vec<SpriteEvent>) {
    sp.frozen = true;
    let (d, speed) = if sp.kind > 2000 {
        (12, Speed { h: sp.speed.h >> 8, v: sp.speed.v >> 8 })
    } else {
        (20, Speed { h: sp.speed.h / 256, v: sp.speed.v / 256 })
    };
    events.push(SpriteEvent::MovingExplosion { h: sp.pos.1.wrapping_sub(d), v: sp.pos.0.wrapping_sub(d), speed, a: 0, b: 0 });
    events.push(SpriteEvent::Sound3D { id: snd::EXPLODE, at: sp.pos, volume: 0xab });
    w.g.chatter = 20;
    let drop = w.level.entry(sp.level_index).map_or(0, |e| e.info[3]);
    if drop != 0 {
        events.push(SpriteEvent::Generate { kind: drop, h: sp.at.1, v: sp.at.0 });
    }
}

/// Out of hit points: dying mode, the level entry cleared, the points.
fn die(sp: &mut Sprite, w: &mut RobotWorld, points: i32) {
    sp.f7c = mode::DEAD;
    if let Some(e) = w.level.entry_mut(sp.level_index) {
        e.present = false;
    }
    w.g.score = w.g.score.wrapping_add(points);
}

/// `HandleRobotSprite` up to the jump (see `handle_robot`).
fn task(sp: &mut Sprite, w: &mut RobotWorld, events: &mut Vec<SpriteEvent>) {
    let player = w.g.player_pos;
    if sp.kind > 2000 {
        let mut firing = false;
        if sp.state < 1 {
            let points = match sp.kind {
                2011 => 2500,
                2001 => 2000,
                2021 => 3000,
                _ => 0,
            };
            die(sp, w, points);
        }
        if too_far(sp, player) {
            return;
        }
        match sp.f7c {
            mode::DEAD => explode(sp, w, events),
            mode::AIM => {
                let d = toward_harry(sp, player);
                approach(sp, d);
                if sp.dir == d {
                    sp.f7c = mode::FIRE;
                    sp.f6e = i32::from(w.rng.below(70)) + 25;
                }
                sp.speed = Speed::default();
                sp.face = BIG_FACES + sp.dir;
            }
            mode::FIRE => {
                if w.rng.below(50) < 20 && sp.f6e > 20 {
                    random_mg_shot(sp, w.rng, w.mg_playing, events);
                    sp.face = BIG_FACES + sp.dir + 36;
                    firing = true;
                } else {
                    sp.face = BIG_FACES + sp.dir;
                }
                sp.f6e -= 1;
                if sp.f6e < 1 {
                    sp.f7c = mode::AIM;
                }
            }
            _ => sp.f7c = mode::AIM,
        }
        sp.integrate();
        if firing {
            // it shakes as it fires
            sp.at.1 = sp.at.1.wrapping_add(2 - w.rng.below(5));
            sp.at.0 = sp.at.0.wrapping_add(2 - w.rng.below(5));
        }
        sp.b8a = true;
        return;
    }
    if too_far(sp, player) {
        return;
    }
    if sp.state < 1 {
        die(sp, w, i32::from((sp.kind - 1791) / 10) * 1250);
    }
    let off = |d: i16, dir: i16, limit: i32| {
        [d, d + 36, d - 36].iter().all(|&x| (i32::from(x) - i32::from(dir)).abs() > limit)
    };
    match sp.f7c {
        mode::DEAD => explode(sp, w, events),
        mode::TURN => {
            let d = toward_harry(sp, player);
            if off(d, sp.dir, 0) {
                sp.dir += if sp.dir < d && d - sp.dir < 18 { 1 } else { -1 };
                sp.dir = wrap_dir(sp.dir);
                random_shot(sp, w.rng, events);
            } else {
                sp.f7c = mode::DRIVE;
            }
        }
        mode::BRAKE => {
            if i32::from(sp.speed.h).abs() < 101 && i32::from(sp.speed.v).abs() < 101 {
                sp.f7c = mode::TURN;
                sp.speed = Speed::default();
            } else {
                sp.speed.h = (f64::from(sp.speed.h) * 0.7) as i32 as i16;
                sp.speed.v = (f64::from(sp.speed.v) * 0.7) as i32 as i16;
                random_shot(sp, w.rng, events);
            }
        }
        mode::DRIVE => {
            let d = toward_harry(sp, player);
            if off(d, sp.dir, 3) {
                sp.f7c = mode::BRAKE;
            } else {
                if i32::from(sp.speed.h).abs() < 150 || i32::from(sp.speed.v).abs() < 150 {
                    add_polar(&mut sp.speed, sp.dir, &ACCEL);
                }
                random_shot(sp, w.rng, events);
            }
        }
        mode::KICKED => {
            if sp.timer == 0 {
                sp.f7c = mode::BRAKE;
            }
        }
        mode::AIM => {
            sp.dir = toward_harry(sp, player);
            random_shot(sp, w.rng, events);
        }
        _ => {}
    }
    if sp.kind == 1811 {
        sp.speed = Speed::default();
    } else {
        sp.integrate();
    }
    sp.b8a = true;
    sp.face = SMALL_FACES + sp.dir;
}

/// `HandleRobotSprite`. The timer counts down; the robot clears the
/// frame's wall flags. Out of hit points: the dying mode, its level entry
/// cleared, points (small: (kind − 1791) / 10 × 1250; big: 2000 / 2500 /
/// 3000) - the small ones only within reach of Harry. Within 500 px of
/// Harry (see the modes in `task`) it moves; in any case it rides half
/// pipes and jumps (centre + 12, sound 600). `unset` is the scale
/// `JumpScale` leaves on landing in a hole (the record's address, see
/// BUGS.md).
pub fn handle_robot(
    sp: &mut Sprite,
    w: &mut RobotWorld,
    unset: i16,
    events: &mut Vec<SpriteEvent>,
    sounds: &mut Vec<crate::air::AirSound>,
) {
    if sp.frozen {
        return;
    }
    if sp.timer > 0 {
        sp.timer -= 1;
    }
    *w.done = FirstBounce::default();
    task(sp, w, events);
    let mut body = sp.body();
    let mut air = sp.air_state();
    let mut s = Vec::new();
    crate::air::half_pipe(&mut body, &mut air, 12, false, w.level, &mut s);
    // HandleHalfPipe with sound 0: none
    let mut unused_comfort = 0;
    crate::air::jump_scale(&mut body, &mut air, &mut sp.dir, 12, false, w.level, w.rng, &mut unused_comfort, unset, sounds);
    sp.set_body(body);
    sp.set_air_state(air);
}

/// Bounce back: the move undone, × −1.4, a driving robot brakes.
fn recoil(sp: &mut Sprite, undo: bool) {
    if undo {
        sp.fix.0 = sp.fix.0.wrapping_sub(i32::from(sp.speed.h));
        sp.fix.1 = sp.fix.1.wrapping_sub(i32::from(sp.speed.v));
    }
    sp.speed.h = (-1.4 * f64::from(sp.speed.h)) as i32 as i16;
    sp.speed.v = (-1.4 * f64::from(sp.speed.v)) as i32 as i16;
    if sp.f7c == mode::DRIVE {
        sp.f7c = mode::BRAKE;
    }
}

/// `HitRobotSprite`. A barrel: it lights (sound 801) and the robot
/// bounces back (× −1.4, without undoing its move). An explosion (when the
/// timer is out): pushed (centre + 12), layer 10, and 3 in 10 cost a hit
/// point. Ramps carry it, stairs take it down. Plants, coolers, desks, colliders,
/// co-workers, chairs, doors (not open), sliding doors, grates:
/// its move undone, × −1.4. Harry's kick: sound 600, a hit point, flies
/// off with 13 × the kick thrust, kick contact 128, kicked mode for 10
/// frames; walking into it: it bounces back, on caffeine losing a point.
#[allow(clippy::too_many_arguments)]
pub fn hit_robot(
    sp: &mut Sprite,
    other: &mut Sprite,
    physics: &Physics,
    rng: &mut Random,
    kick_contact: &mut i16,
    caffeine: bool,
    events: &mut Vec<SpriteEvent>,
) {
    if sp.frozen || other.frozen {
        return;
    }
    let door = other.task == Some(Task::Door) && !door_open(other.kind);
    match other.task {
        Some(Task::Barrel) => {
            if other.dir == 1 {
                events.push(SpriteEvent::sound_3d(crate::furniture::snd::BARREL_FIRE, other.at));
                other.dir += 1;
            }
            recoil(sp, false);
        }
        Some(Task::Effect) => {
            if sp.timer == 0 && other.hot_rect != [0; 4] {
                explode_radius(sp.at, &mut sp.speed, &mut sp.blast_cooldown, 12, other);
                sp.layer = 10;
                if rng.below(100) < 30 {
                    sp.state -= 1;
                }
            }
        }
        Some(Task::Ramp) => {
            let mut body = sp.body();
            let hot = sp.hot_rect;
            let mut r = Rider { body: &mut body, air: &mut sp.air, cooldown: &mut sp.ramp_cooldown, player: false };
            if handle_ramp_collision(&mut r, 12, hot, other, physics.bounce) {
                events.push(SpriteEvent::Sound(snd::HIT));
            }
            sp.set_body(body);
        }
        Some(Task::Stairs) => object_on_stairs(sp, 12, other, physics.bounce),
        Some(Task::Plant | Task::Cooler | Task::Desk | Task::Collider | Task::CoWorker | Task::Chair | Task::SlidingDoor | Task::Grate) => {
            recoil(sp, true)
        }
        _ if door => recoil(sp, true),
        Some(Task::Player) => {
            if other.hot_rect[1] == 0 {
                let kick = offset(KICK_RECTS[other.dir as usize], other.at.0, other.at.1);
                if sect(kick, offset(sp.hot_rect, sp.at.0, sp.at.1)) {
                    events.push(SpriteEvent::sound_3d(snd::HIT, sp.centre()));
                    sp.state -= 1;
                    sp.layer = 10;
                    let thrust = ((i32::from(physics.kick) * 26) >> 1) as i16;
                    apply_moded_impulse(&mut sp.speed, inverse_mode(other.dir), thrust);
                    *kick_contact = 0x80;
                    sp.f7c = mode::KICKED;
                    sp.timer = 10;
                }
            } else {
                recoil(sp, true);
                if caffeine {
                    sp.state -= 1;
                    sp.layer = 10;
                }
            }
        }
        _ => {}
    }
}

/// `HitRobotTileSprite`: walls (`WallBounce`, centre + 12, bounce 256)
/// and, on the floor, background obstacles (`BGTileBounce`, bounce 254),
/// with the robot mask (face 4251: cicn 598 with the faces' mirrored
/// bits) - each only for the first robot of the
/// frame to touch one (`RobotWorld`'s flags); a wall it bounced off puts
/// it into braking.
pub fn hit_robot_tile(sp: &mut Sprite, hit: TileHit, m: &TileMasks, done: &mut FirstBounce) {
    if hit.foreground {
        if first_wall_bounce(sp, hit, (12, 12), 0x100, m.mask, m.walls, done) {
            sp.f7c = mode::BRAKE;
        }
    } else if first_obstacle(sp, hit, done) {
        obstacle_bounce(sp, hit.tile, (12, 12), 0xfe, m);
    }
}

/// A robot losing a hit point to a shot (`HitPlayerShotSprite`,
/// `HitEnemyShotSprite`): layer 10, the point, `f6e` 0, sound 600, an
/// explosion of kind −1 (no hot rect) at pos − 20 (`big`: the big ones
/// at pos − 12) while it has points left.
pub fn shot_hit(robot: &mut Sprite, big: bool, events: &mut Vec<SpriteEvent>) {
    robot.layer = 10;
    robot.state -= 1;
    robot.f6e = 0;
    events.push(SpriteEvent::Sound(snd::HIT));
    if robot.state > 0 {
        let d = if big { 12 } else { 20 };
        events.push(SpriteEvent::Explosion { h: robot.pos.1.wrapping_sub(d), v: robot.pos.0.wrapping_sub(d), a: -1, b: 0 });
    }
}
