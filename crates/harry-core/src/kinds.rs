//! Sprite kinds: bonuses, items, exits, doors, water coolers, plants,
//! messages. Each has a setup (run when
//! the sprite is made), a task (each frame) and a hit task (SAT's
//! collision pass); what Harry does when he touches them is in
//! `HitPlayerSprite`.

use crate::air::{AirSound, PLANT_UNSET_SCALE, half_pipe, jump_scale};
use crate::bounce::{bg_tile_bounce, rect_bounce, wall_bounce};
use crate::level::Level;
use crate::physics::{Physics, apply_moded_impulse, handle_glide, inverse_mode};
use crate::ramp::{Rider, handle_ramp_collision};
use crate::mailguy::mail_guy_crash;
use crate::random::Random;
use crate::stairs::object_on_stairs;
use crate::words::Words;
use crate::mask::{Mask, mask_hit};
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};
use crate::tiles::{KICK_RECTS, TileHit, offset, sect};

/// `SetupBonusSprite` (700–799: the bonus objects Harry collects).
pub fn setup_bonus(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Bonus);
    sp.hit_task = Some(HitTask::Bonus);
    sp.hit_tile_task = None;
    sp.layer = 1;
    sp.hot_rect = [-6, -6, 22, 22];
    match sp.kind {
        700 | 701 => sp.face = 1501,
        702 => {
            sp.face = 1503;
            sp.hot_rect = [-6, -6, 36, 36];
        }
        703 => sp.face = 1504,
        711..=713 => {
            sp.hot_rect = [-10, -10, 26, 26];
            sp.face = 1505 + (sp.kind - 711);
        }
        _ => {}
    }
    sp.at_from_pos();
    sp.fix_from_at();
    sp.b8b = true;
}

/// `HandleBonusSprite`: 700 and 701 blink between two faces.
pub fn handle_bonus(sp: &mut Sprite) {
    if sp.frozen || !matches!(sp.kind, 700 | 701) {
        return;
    }
    if sp.dir == 0 {
        sp.dir = 1;
        sp.face = 1502;
    } else {
        sp.dir = 0;
        sp.face = 1501;
    }
}

/// `SetupItemSprite` (11001–19999: items to pick up; the object id is the
/// inventory id).
pub fn setup_item(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Item);
    sp.hit_task = Some(HitTask::Item);
    sp.hit_tile_task = None;
    sp.hot_rect = [-6, -6, 20, 20];
    sp.face = match sp.kind {
        11699 => 0,
        11398 => 3651,
        11397 => 3652,
        // faces of 11199, 11299, … 12099 (none for 11699 and 12099); ids
        // outside read past the original's table
        k => match (k - 11199) / 100 {
            i @ (0..=4 | 6..=8) => 3601 + i,
            _ => 0,
        },
    };
    sp.layer = 7;
    sp.f6e = 0;
    sp.at_from_pos();
    sp.b8b = true;
}

/// `HandleItemSprite`: the mystery vial (11699) bubbles through 8 faces,
/// two frames each.
pub fn handle_item(sp: &mut Sprite) {
    if sp.frozen || sp.kind != 11699 {
        return;
    }
    sp.dir += 1;
    if sp.dir > 15 {
        sp.dir = 0;
    }
    sp.face = 3661 + sp.dir / 2;
}

/// `SetupExitSprite` (9990–9999). 9991–9994 are the four directions,
/// each with its face and a hot rect beside it.
pub fn setup_exit(sp: &mut Sprite) {
    sp.init();
    sp.timer = 0;
    sp.dir = 0;
    sp.task = Some(Task::Exit);
    sp.hit_task = Some(HitTask::Exit);
    sp.hit_tile_task = None;
    (sp.face, sp.hot_rect) = match sp.kind {
        9991 => (3201, [96, 0, 100, 64]),
        9992 => (3202, [-40, 0, -32, 64]),
        9993 => (3203, [0, 40, 64, 100]),
        9994 => (3204, [0, -32, 64, 0]),
        _ => (0, [0, 0, 32, 32]),
    };
    sp.layer = 1;
    sp.at_from_pos();
    sp.b8b = false;
}

/// The level counter and the one before (`*_DAT_100659cc`,
/// `_DAT_10068568`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LevelCounter {
    pub current: i16,
    pub previous: i16,
}

/// `HitExitSprite`: Harry reaches the exit - the next level is the exit's
/// info #0, or the following one when that is 0. Works once (timer 69,
/// never counted down).
pub fn hit_exit(sp: &mut Sprite, other: &Sprite, level: &Level, counter: &mut LevelCounter) {
    if sp.frozen || other.frozen || other.task != Some(Task::Player) || sp.timer != 0 {
        return;
    }
    counter.previous = counter.current;
    let dest = level.sprites.get(sp.level_index as usize).map_or(0, |s| s.info[0]);
    counter.current = if dest == 0 { counter.current.wrapping_add(1) } else { dest };
    sp.timer = 69;
}

/// Door faces: 36 frames from HrSp 2601, nine per door kind (opening).
const DOOR_FACES: i16 = 2601;

/// The four door kinds (hinged at different sides): 901–903, 911–913,
/// 921–923, 931–933; …1 closed, …2 open, …3 locked (a key opens it). Index
/// 0–3 of the kind, or `None`.
pub fn door_group(kind: i16) -> Option<i16> {
    match kind {
        901..=903 => Some(0),
        911..=913 => Some(1),
        921..=923 => Some(2),
        931..=933 => Some(3),
        _ => None,
    }
}

/// An open door (902, 912, 922, 932).
pub fn door_open(kind: i16) -> bool {
    matches!(kind, 902 | 912 | 922 | 932)
}

/// `SetupDoorSprite`. An open door starts at frame 16 (fully open).
pub fn setup_door(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Door);
    sp.hit_task = Some(HitTask::Door);
    sp.hit_tile_task = None;
    sp.layer = 5;
    if door_open(sp.kind) {
        sp.dir = 16;
    }
    if let Some(g) = door_group(sp.kind) {
        sp.hot_rect = [[34, 15, 37, 46], [15, 16, 18, 47], [3, 33, 34, 36], [1, 4, 32, 7]][g as usize];
        sp.face = if door_open(sp.kind) { DOOR_FACES + 9 * g + 8 } else { DOOR_FACES + 9 * g + sp.dir };
    }
    sp.at_from_pos();
    sp.b8b = true;
}

/// `HandleDoorSprite`: a door Harry kicked open (dir 1–16) swings one
/// step per frame (a face every two) and is open from its first step on -
/// also in the level's sprite list, so it stays open.
pub fn handle_door(sp: &mut Sprite, level: &mut Level) {
    if sp.frozen {
        return;
    }
    if sp.hit_task.is_none() {
        sp.hot_rect = [0; 4];
    }
    if let Some(g) = door_group(sp.kind) {
        sp.face = DOOR_FACES + 9 * g + sp.dir / 2;
    }
    if 0 < sp.dir && sp.dir < 17 {
        sp.dir += 1;
        if let Some(g) = door_group(sp.kind).filter(|_| !door_open(sp.kind)) {
            sp.kind = 902 + 10 * g;
        }
        if let Some(e) = usize::try_from(sp.level_index).ok().and_then(|i| level.sprites.get_mut(i)) {
            e.id = sp.kind;
        }
    }
}

/// `HitDoorSprite`: a closed door stops shots.
pub fn hit_door(sp: &Sprite, other: &mut Sprite) {
    if !sp.frozen
        && !other.frozen
        && matches!(other.task, Some(Task::EnemyShot | Task::PlayerShot))
        && !door_open(sp.kind)
    {
        other.frozen = true;
    }
}

/// `SetupCoolerSprite` (500, 501): a water cooler, face 2001, solid to
/// Harry (mask HrSp 2002), destroyed by four shots.
pub fn setup_cooler(sp: &mut Sprite) {
    sp.init();
    sp.dir = 1;
    sp.layer = 8;
    sp.task = Some(Task::Cooler);
    sp.hit_task = Some(HitTask::Cooler);
    sp.hit_tile_task = None;
    sp.speed = Default::default();
    sp.hot_rect = [0, 0, 24, 24];
    sp.face = 2001;
    sp.at_from_pos();
    sp.fix_from_at();
    sp.b8b = true;
}

/// `HandleCoolerSprite`: `dir` counts the hits; at 4 it explodes (the
/// explosion kind is the level entry's info #1, default 5) and is gone from
/// the level, at 10 the sprite is removed.
pub fn handle_cooler(sp: &mut Sprite, level: &mut Level, events: &mut Vec<SpriteEvent>) {
    if sp.frozen {
        return;
    }
    let entry = usize::try_from(sp.level_index).ok().filter(|&i| i < 256);
    match sp.dir {
        10 => sp.frozen = true,
        4 => {
            let kind = entry.map(|i| level.sprites[i].info[1]).filter(|&k| k != 0).unwrap_or(5);
            events.push(SpriteEvent::Explosion { h: sp.pos.1.wrapping_sub(20), v: sp.pos.0.wrapping_sub(20), a: kind, b: 0 });
            sp.dir += 1;
            if let Some(i) = entry {
                level.sprites[i].present = false;
            }
        }
        5..=9 => sp.dir += 1,
        _ => sp.face = 2001,
    }
}

/// The masks the coolers need.
pub struct CoolerMasks<'a> {
    /// Harry's body (cicn 299).
    pub body: &'a Mask,
    /// The cooler (HrSp 2002, mirrored like all face masks).
    pub cooler: &'a Mask,
}

/// `HitCoolerSprite`. Harry (on the floor): kicking it rings it (3-D
/// sound 501, kick contact 192) when the kick rect of his direction
/// touches it; walking into it moves him out by the mask test, with sound
/// 501, his speed × −0.6 and kick contact 256. Shots (frozen) and
/// effects (not kind 9610) knock it: sound 502 for the first three, 500
/// for the fourth.
pub fn hit_cooler(
    sp: &mut Sprite,
    other: &mut Sprite,
    masks: &CoolerMasks,
    kick_contact: &mut i16,
    events: &mut Vec<SpriteEvent>,
) {
    if sp.frozen || other.frozen || other.air >= 1 {
        return;
    }
    if other.task == Some(Task::Player) {
        if other.hot_rect[1] == 0 {
            let kick = offset(KICK_RECTS[other.dir as usize], other.at.0, other.at.1);
            if sect(kick, offset(sp.hot_rect, sp.at.0, sp.at.1)) {
                events.push(SpriteEvent::Sound3D(501));
                *kick_contact = 0xc0;
            }
        } else {
            other.pos = other.at;
            if mask_hit(&mut other.pos, masks.body, sp.at, masks.cooler) {
                other.at = other.pos;
                other.fix_from_at();
                events.push(SpriteEvent::Sound(501));
                let damp = |s: i16| (-0.6 * f64::from(s)) as i32 as i16;
                other.speed.h = damp(other.speed.h);
                other.speed.v = damp(other.speed.v);
                *kick_contact = 0x100;
            }
        }
    }
    if matches!(other.task, Some(Task::PlayerShot | Task::EnemyShot | Task::Effect)) && other.kind != 9610 {
        if other.task != Some(Task::Effect) {
            other.frozen = true;
        }
        if sp.dir < 3 {
            events.push(SpriteEvent::Sound(502));
            sp.dir += 1;
        } else if sp.dir == 3 {
            events.push(SpriteEvent::Sound(500));
            sp.dir += 1;
        }
    }
}

/// `SetupPlantSprite` (601): a potted plant Harry can kick and push
/// around (layer 8, hot rect (8, 8, 28, 28), face 3901).
pub fn setup_plant(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.speed = Default::default();
    sp.layer = 8;
    sp.task = Some(Task::Plant);
    sp.hit_task = Some(HitTask::Plant);
    sp.hit_tile_task = Some(crate::sprite::HitTileTask::Plant);
    sp.hot_rect = [8, 8, 28, 28];
    sp.face = 3901;
    sp.at_from_pos();
    sp.fix_from_at();
}

/// `HandlePlantSprite`: a hit plant (dir 1) shakes through faces 3902,
/// 3903, 3904 (two frames each, then back to 3901); it glides with the
/// floor's friction, rides half pipes and jumps like other objects (centre
/// at + 18). `plant_flag` is `PTR_DAT_10066118`, cleared by every plant.
pub fn handle_plant(
    sp: &mut Sprite,
    level: &Level,
    physics: &Physics,
    rng: &mut Random,
    plant_flag: &mut bool,
    sounds: &mut Vec<AirSound>,
) {
    if sp.frozen {
        return;
    }
    *plant_flag = false;
    match sp.dir {
        1 | 2 => (sp.face, sp.dir) = (3902, sp.dir + 1),
        3 | 4 => (sp.face, sp.dir) = (3903, sp.dir + 1),
        5 => (sp.face, sp.dir) = (3904, 6),
        6 => (sp.face, sp.dir) = (3904, 0),
        _ => sp.face = 3901,
    }
    handle_glide(&mut sp.speed, sp.fix, sp.air, sp.slide, 18, level, physics);
    let mut body = sp.body();
    let mut air = sp.air_state();
    half_pipe(&mut body, &mut air, 18, false, level, sounds);
    let mut unused_comfort = 0;
    jump_scale(&mut body, &mut air, &mut sp.dir, 18, false, level, rng, &mut unused_comfort, PLANT_UNSET_SCALE, sounds);
    sp.set_body(body);
    sp.set_air_state(air);
    sp.integrate();
    sp.b8b = true;
}

/// The masks objects bounce with.
pub struct ObjectMasks<'a> {
    /// The object's own mask: the plant's, cicn 599 (the barrel's: 598).
    pub plant: &'a Mask,
    /// The 32 wall shapes (PICT 160).
    pub walls: &'a [Mask],
    /// The background obstacle (cicn 290).
    pub obstacle: &'a Mask,
}

/// `HitPlantTileSprite`: walls (`WallBounce`) and, on the floor,
/// background obstacles (`BGTileBounce`), with the plant's mask and the
/// level's bounce, always as the frame's first contact.
pub fn hit_plant_tile(sp: &mut Sprite, hit: TileHit, level: &Level, masks: &ObjectMasks, physics: &Physics) {
    hit_object_tile(sp, hit, level, masks, 18, physics);
}

/// The tile routine pushable objects share (plant, chair, barrel): as
/// `hit_plant_tile` with `masks.plant` as the object's mask and its centre
/// at + `half`.
pub fn hit_object_tile(sp: &mut Sprite, hit: TileHit, level: &Level, masks: &ObjectMasks, half: i16, physics: &Physics) {
    let mut body = sp.body();
    let tile = hit.tile;
    if hit.foreground {
        let wall = usize::try_from(hit.kind).ok().and_then(|k| masks.walls.get(k));
        wall_bounce(&mut body, hit.kind, tile, (half, half), physics.bounce, sp.hot_rect, true, &mut |b| match wall {
            Some(w) => mask_hit(&mut b.pos, masks.plant, tile, w),
            None => false,
        });
    } else if sp.air == 0 && hit.kind == 1 {
        let fg = |x: i32, y: i32| level.fg_kind(level.fg_tile(x, y));
        bg_tile_bounce(&mut body, tile, (half, half), physics.bounce, sp.hot_rect, true, &mut |b| {
            mask_hit(&mut b.pos, masks.plant, tile, masks.obstacle)
        }, &fg);
    }
    sp.set_body(body);
}

/// The masks a plant's hit task uses.
pub struct PlantHitMasks<'a> {
    /// Harry's body (cicn 299).
    pub body: &'a Mask,
    /// Plant and chair (cicn 599).
    pub plant: &'a Mask,
    /// Barrel and water cooler (cicn 598).
    pub barrel: &'a Mask,
}

/// `(int)(f * s)` in double, then to short.
pub(crate) fn scaled(f: f64, s: i16) -> i16 {
    (f * f64::from(s)) as i32 as i16
}

/// `(int)(a + f * b)` in double, then to short.
pub(crate) fn plus_scaled(a: i16, f: f64, b: i16) -> i16 {
    (f64::from(a) + f * f64::from(b)) as i32 as i16
}

/// `HitPlantSprite` (both on the floor). Harry kicking it (his kick rect
/// touching it) sends it flying away from him with 13 × the kick thrust;
/// walking into it, the mask test pushes him out, the plant takes 0.6 of
/// his speed and he keeps −0.3. Against another plant, a cooler, a barrel
/// or a chair they are pushed apart (jittered when on the same pixel); a
/// plant, barrel or chair trades momentum (0.4 each way, then both ×
/// −0.4), a cooler stays and the plant bounces back × −0.4; a robot
/// (plant mask against plant mask) gives it half its speed. Doors (not
/// open), sliding doors, colliders and co-workers: `RectBounce`; grates:
/// `GrateBounce`; ramps carry it (sound
/// 1101 when it takes off), stairs take it down. Shots stop on it and make it shake.
/// `bump_playing`: is the plant's bump sound (1100) playing?
#[allow(clippy::too_many_arguments)]
pub fn hit_plant(
    sp: &mut Sprite,
    other: &mut Sprite,
    masks: &PlantHitMasks,
    physics: &Physics,
    rng: &mut Random,
    kick_contact: &mut i16,
    bump_playing: bool,
    events: &mut Vec<SpriteEvent>,
) {
    if sp.frozen || other.frozen || sp.air >= 1 || other.air >= 1 {
        return;
    }
    match other.task {
        // an explosion with a hot rect pushes it and knocks it about
        Some(Task::Effect) => {
            if other.hot_rect != [0; 4] {
                crate::effect::explode_radius(sp.at, &mut sp.speed, &mut sp.blast_cooldown, 18, other);
                sp.state = sp.state.wrapping_sub(1);
                sp.layer = 10;
            }
        }
        Some(Task::MailGuy) => mail_guy_crash(sp, other, events),
        Some(Task::Player) => {
            if other.hot_rect[1] == 0 {
                let kick = offset(KICK_RECTS[other.dir as usize], other.at.0, other.at.1);
                if sect(kick, offset(sp.hot_rect, sp.at.0, sp.at.1)) {
                    sp.dir = 1;
                    events.push(SpriteEvent::Sound3D(1101));
                    let thrust = ((i32::from(physics.kick) * 26) >> 1) as i16;
                    apply_moded_impulse(&mut sp.speed, inverse_mode(other.dir), thrust);
                    *kick_contact = 0x80;
                }
            } else if mask_hit(&mut other.pos, masks.body, sp.at, masks.plant) {
                sp.dir = 1;
                other.at = other.pos;
                other.fix_from_at();
                sp.fix_from_at();
                if !bump_playing {
                    events.push(SpriteEvent::Sound(1100));
                }
                sp.speed.h = scaled(0.6, other.speed.h);
                sp.speed.v = scaled(0.6, other.speed.v);
                other.speed.h = scaled(-0.3, other.speed.h);
                other.speed.v = scaled(-0.3, other.speed.v);
            }
        }
        Some(Task::Plant | Task::Cooler | Task::Barrel | Task::Chair | Task::Robot) => {
            if other.fix.0 >> 8 == sp.fix.0 >> 8 && other.fix.1 >> 8 == sp.fix.1 >> 8 {
                sp.fix.0 -= i32::from(rng.below(0x300)) + 0x100;
                sp.fix.1 -= i32::from(rng.below(0x300)) + 0x100;
                other.fix.0 += i32::from(rng.below(0x300)) + 0x100;
                other.fix.1 += i32::from(rng.below(0x300)) + 0x100;
            }
            sp.pos = ((sp.fix.1 >> 8) as i16, (sp.fix.0 >> 8) as i16);
            sp.at = sp.pos;
            other.pos = ((other.fix.1 >> 8) as i16, (other.fix.0 >> 8) as i16);
            other.at = other.pos;
            let other_mask = if matches!(other.task, Some(Task::Cooler | Task::Barrel)) { masks.barrel } else { masks.plant };
            if mask_hit(&mut sp.pos, masks.plant, other.at, other_mask) {
                sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
                if !bump_playing {
                    events.push(SpriteEvent::Sound(1100));
                }
                if other.task == Some(Task::Robot) {
                    sp.speed.h = sp.speed.h.wrapping_add(other.speed.h >> 1);
                    sp.speed.v = sp.speed.v.wrapping_add(other.speed.v >> 1);
                } else if other.task != Some(Task::Cooler) {
                    other.speed.h = plus_scaled(other.speed.h, 0.4, sp.speed.h);
                    other.speed.v = plus_scaled(other.speed.v, 0.4, sp.speed.v);
                    sp.speed.h = plus_scaled(sp.speed.h, 0.4, other.speed.h);
                    sp.speed.v = plus_scaled(sp.speed.v, 0.4, other.speed.v);
                    sp.speed.h = scaled(-0.4, sp.speed.h);
                    sp.speed.v = scaled(-0.4, sp.speed.v);
                    other.speed.h = scaled(-0.4, other.speed.h);
                    other.speed.v = scaled(-0.4, other.speed.v);
                } else {
                    sp.speed.h = scaled(-0.4, sp.speed.h);
                    sp.speed.v = scaled(-0.4, sp.speed.v);
                }
            }
        }
        Some(Task::Grate) => crate::grate::grate_bounce(sp, other, 18, i32::from(physics.bounce), sp.hot_rect),
        _ => {}
    }
    let door = other.task == Some(Task::Door) && !door_open(other.kind);
    if door || matches!(other.task, Some(Task::Collider | Task::SlidingDoor | Task::CoWorker)) {
        let mut body = sp.body();
        rect_bounce(&mut body, 18, i32::from(physics.bounce), sp.hot_rect, other.at, other.hot_rect);
        sp.set_body(body);
    }
    if other.task == Some(Task::Ramp) {
        let mut body = sp.body();
        let hot = sp.hot_rect;
        let mut r = Rider { body: &mut body, air: &mut sp.air, cooldown: &mut sp.f94, player: false };
        if handle_ramp_collision(&mut r, 18, hot, other, physics.bounce) {
            events.push(SpriteEvent::Sound(1101));
        }
        sp.set_body(body);
    }
    if other.task == Some(Task::Stairs) {
        object_on_stairs(sp, 18, other, physics.bounce);
    }
    if matches!(other.task, Some(Task::PlayerShot | Task::EnemyShot)) && other.kind != 9610 {
        other.frozen = true;
        events.push(SpriteEvent::Sound3D(1101));
        sp.dir = 1;
        if other.kind == 9450 {
            sp.speed.h = sp.speed.h.wrapping_add(other.speed.h >> 2);
            sp.speed.v = sp.speed.v.wrapping_add(other.speed.v >> 2);
        }
    }
}

/// The text of a speech balloon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BalloonText {
    /// Text n of the world (`Hwld` + 0x3c9 + n · 256, Pascal strings).
    World(i16),
    /// A sentence of the buzzword generator (`GenerateBuzzwords`, Mac
    /// Roman), made just before the balloon is tried.
    Buzzwords(Vec<u8>),
    /// A sentence of the phone word generator (`GeneratePhonewords`).
    Phonewords(Vec<u8>),
    /// One of the application's own texts (the key cheats' balloons): 0–51
    /// the random ones, −1 the B-E-N one.
    Message(i16),
}

/// `ShowBalloon(sprite, text, mode)`: shows the balloon over the sprite
/// (modal: the game waits for a key) if it fits on the screen and tells
/// whether it did. Part of the frontend; when shown and mode ≠ 5 it also
/// freezes the sprite, which the callers here do themselves (and
/// `game::Game` does its random number and view nudge).
pub type ShowBalloon<'a> = dyn FnMut(&Sprite, BalloonText, i16) -> bool + 'a;

/// Message faces: a blinking memo, HrSp 3801–3805.
const MEMO_FACES: i16 = 3801;

/// `SetupMessageSprite` (21000 and up): a memo on the floor or, with info
/// #1 set, an invisible spot; layer 9, no hot rect until its first frame.
pub fn setup_message(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Message);
    sp.hit_task = Some(HitTask::Message);
    sp.hit_tile_task = None;
    sp.face = MEMO_FACES;
    sp.layer = 9;
    sp.at_from_pos();
    sp.b8b = false;
    sp.f6e = 0;
}

fn balloon(sp: &Sprite, level: &Level, words: &mut Words, rng: &mut Random) -> (BalloonText, i16) {
    let text = usize::try_from(sp.level_index).ok().and_then(|i| level.sprites.get(i)).map_or(0, |e| e.info[0]);
    let mode = sp.f6e as i16;
    (if sp.f6e == 3 { BalloonText::Buzzwords(words.buzzwords(rng)) } else { BalloonText::World(text) }, mode)
}

/// `HandleMessageSprite`: the mode (`f6e`) comes from the level entry's
/// info #1 each frame - 0 a memo (hot rect (−8, −8, 32, 32), its face
/// blinking up and down over 30 frames), else an invisible spot (hot rect
/// (−32, −32, 64, 64)). A balloon that did not fit is tried again each
/// frame; once shown the message is done.
pub fn handle_message(sp: &mut Sprite, level: &Level, words: &mut Words, rng: &mut Random, show: &mut ShowBalloon) {
    if sp.frozen {
        return;
    }
    sp.dir += 1;
    sp.kind = 21000;
    if let Some(e) = usize::try_from(sp.level_index).ok().filter(|&i| i < 256).map(|i| &level.sprites[i]) {
        sp.f6e = i32::from(e.info[1]);
    }
    if sp.f6e == 0 {
        sp.hot_rect = [-8, -8, 32, 32];
        if sp.dir < 15 {
            sp.face = MEMO_FACES + sp.dir / 3;
        } else if sp.dir < 30 {
            sp.face = MEMO_FACES + (29 - sp.dir) / 3;
        } else {
            sp.dir = 0;
        }
    } else {
        sp.hot_rect = [-32, -32, 64, 64];
        sp.face = 0;
    }
    if sp.balloon_pending {
        let (text, mode) = balloon(sp, level, words, rng);
        if show(sp, text, mode) {
            sp.frozen = true;
            sp.balloon_pending = false;
            sp.hit_task = None;
        }
    }
}

/// `HitMessageSprite`: Harry (not kicking) walks onto it - its balloon.
/// If it does not fit and the balloon preference (prefs +8) is above 2,
/// the message keeps trying (`balloon_pending`).
pub fn hit_message(
    sp: &mut Sprite,
    other: &Sprite,
    level: &Level,
    retry: bool,
    words: &mut Words,
    rng: &mut Random,
    show: &mut ShowBalloon,
) {
    if sp.frozen || other.frozen || other.task != Some(Task::Player) || other.hot_rect[1] == 0 {
        return;
    }
    let (text, mode) = balloon(sp, level, words, rng);
    if show(sp, text, mode) {
        if mode != 5 {
            sp.frozen = true;
        }
    } else if retry {
        sp.balloon_pending = true;
    }
}
