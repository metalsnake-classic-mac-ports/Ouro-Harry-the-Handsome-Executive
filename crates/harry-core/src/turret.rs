//! Turrets on walls (1901 / 1911 / 1921 facing up, …2 down-right, …3
//! left, …4 right): they swing toward Harry within their arc and shoot
//! when they face him.

use crate::robot::{SHOT_SPEED, add_polar, toward_harry};
use crate::random::Random;
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};

/// 32 faces (PICT 1901).
const FACES: i16 = 4601;

/// The arc a turret swings in (directions it takes), its start, and the
/// face of a direction.
fn mount(kind: i16) -> Option<u8> {
    match kind {
        1901 | 1911 | 1921 => Some(0),
        1902 | 1912 | 1922 => Some(1),
        1903 | 1913 | 1923 => Some(2),
        1904 | 1914 | 1924 => Some(3),
        _ => None,
    }
}

fn in_arc(m: u8, d: i16) -> bool {
    match m {
        0 => (5..13).contains(&d),
        1 => (23..31).contains(&d),
        2 => (14..22).contains(&d),
        _ => !(4..=31).contains(&d),
    }
}

fn face(m: u8, dir: i16) -> i16 {
    FACES
        + match m {
            0 => dir - 5,
            1 => dir - 15,
            2 => dir + 2,
            _ if dir < 4 => dir + 28,
            _ => dir - 8,
        }
}

/// `SetupTurretSprite`: timer 25, its arc's start direction (10, 30, 18,
/// 0), layer 7, hot rect (4, 4, 20, 20), no face until its first frame;
/// 25 hit points (1911–1919: 40, 1920 and up: 50).
pub fn setup_turret(sp: &mut Sprite) {
    sp.init();
    sp.timer = 25;
    if let Some(m) = mount(sp.kind) {
        sp.dir = [10, 30, 18, 0][usize::from(m)];
    }
    sp.task = Some(Task::Turret);
    sp.hit_task = Some(HitTask::Turret);
    sp.hit_tile_task = None;
    sp.hot_rect = [4, 4, 20, 20];
    sp.layer = 7;
    sp.speed = Default::default();
    sp.at_from_pos();
    sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
    sp.f6e = 1;
    sp.state = if sp.kind < 1910 { 25 } else if sp.kind < 1920 { 40 } else { 50 };
    sp.face = 0;
    sp.b8b = true;
}

/// `HandleTurretSprite`. The timer counts down. It takes Harry's
/// direction when that is within its arc and, facing him, shoots now and
/// then (`RandomTurretShot`: 1 in 35 for 1911–1919, else 1 in 60; a shot
/// of its kind from pos + (7, 8), 1500 along its direction, sound 601).
/// Out of hit points: frozen, an explosion at pos − 20, sound 602, its
/// level entry's info #3 made where it stood, 1000 / 1500 / 2000 points
/// (its entry stays: it comes back). `player` is Harry's top-left (h, v).
pub fn handle_turret(sp: &mut Sprite, player: (i16, i16), drop: i16, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    if sp.timer > 0 {
        sp.timer -= 1;
    }
    let d = toward_harry(sp, player);
    let m = mount(sp.kind);
    if m.is_some_and(|m| in_arc(m, d)) {
        sp.dir = d;
    }
    if sp.dir == d {
        let quick = (1910..1920).contains(&sp.kind);
        if rng.below(if quick { 35 } else { 60 }) == 1 {
            let mut speed = sp.speed;
            add_polar(&mut speed, sp.dir, &SHOT_SPEED);
            events.push(SpriteEvent::Shot { kind: sp.kind, h: sp.pos.1.wrapping_add(7), v: sp.pos.0.wrapping_add(8), speed, dir: sp.dir, fix: None, timer: None, f7c: None });
            events.push(SpriteEvent::Sound3D(crate::robot::snd::SHOT));
        }
    }
    if sp.state < 1 {
        sp.frozen = true;
        let speed = crate::physics::Speed { h: sp.speed.h / 256, v: sp.speed.v / 256 };
        events.push(SpriteEvent::MovingExplosion { h: sp.pos.1.wrapping_sub(20), v: sp.pos.0.wrapping_sub(20), speed, a: 0, b: 0 });
        events.push(SpriteEvent::Sound3D(crate::robot::snd::EXPLODE));
        if drop != 0 {
            events.push(SpriteEvent::Generate { kind: drop, h: sp.at.1, v: sp.at.0 });
        }
        events.push(SpriteEvent::Score(if sp.kind < 1910 { 1000 } else if sp.kind < 1920 { 1500 } else { 2000 }));
    }
    if let Some(m) = m {
        sp.face = face(m, sp.dir);
    }
}

/// `HitTurretSprite`: a player shot costs 5 hit points (sound 600) and
/// is gone; an explosion (when the timer is out) one, layer 10.
pub fn hit_turret(sp: &mut Sprite, other: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen {
        return;
    }
    if other.task == Some(Task::PlayerShot) {
        events.push(SpriteEvent::Sound3D(crate::robot::snd::HIT));
        sp.state -= 5;
        other.frozen = true;
    } else if other.task == Some(Task::Effect) && sp.timer == 0 && other.hot_rect != [0; 4] {
        sp.state -= 1;
        sp.layer = 10;
    }
}
