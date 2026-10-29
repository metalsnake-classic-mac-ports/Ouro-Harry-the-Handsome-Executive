//! Colliders: the office machines and fixtures Harry bumps into
//! (`SetupColliderSprite`, `HandleColliderSprite`, `HitColliderSprite`).
//! One sprite kind with a branch per object id:
//!
//! - 2301–2304 fans blowing up, down, left, right (`HandleRepulsions`);
//! - 3100 the phone (rings, answered with a balloon: 3101);
//! - 9199 / 9200 the copier (9200 saves the game);
//! - 9300 the magnet (pulls an armed Harry down);
//! - 9401 / 9411 / 9421 / 9431 wall guns shooting up, down, right, left
//!   while their switch is on;
//! - 9600 the vending machine (kicked: drops a can, 9610);
//! - 9800 the gate that opens for item 11199;
//! - 1019 / 1029 / 1039 / 1049 and 2001: only a hot rect (the background
//!   props come from `AddBGProp`, so these are never made here).
//!
//! What Harry does when he touches them is in `HitPlayerSprite`
//! (`touch`).

use crate::frame::PlayerGlobals;
use crate::kinds::{BalloonText, ShowBalloon};
use crate::level::Level;
use crate::physics::Speed;
use crate::random::Random;
use crate::words::Words;
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};

/// `snd` ids.
pub mod snd {
    pub const PHONE: i16 = 301;
    pub const GUN: i16 = 250;
    pub const MAGNET: i16 = 602;
    pub const GATE: i16 = 700;
    pub const VENDING_EMPTY: i16 = 1301;
    pub const VENDING_DROP: i16 = 1302;
    pub const FAN: i16 = 1303;
}

/// HrSp ids of the faces (`InitColliderSprite`).
pub mod face {
    pub const GATE: i16 = 1901;
    pub const VENDING: i16 = 1902;
    pub const VENDING_KICKED: i16 = 1903;
    pub const PHONE_RING: i16 = 1904;
    pub const PHONE: i16 = 1905;
    pub const COPIER: i16 = 1906;
    pub const MAGNET: i16 = 1907;
    /// The wall guns 9401, 9411, 9421, 9431.
    pub const GUN_UP: i16 = 1908;
    /// The fan's four frames.
    pub const FAN: i16 = 1921;
}

/// The vending machine's can and the wall guns' shot (enemy shots).
pub const CAN: i16 = 9610;
pub const GUN_SHOT: i16 = 9450;

/// `f6e` of a fan before its first frame near Harry.
const FAN_UNSET: i32 = -69;

fn is_fan(kind: i16) -> bool {
    (2301..=2304).contains(&kind)
}

/// `SetRect(left, top, right, bottom)` as a hot rect.
fn set_rect(l: i16, t: i16, r: i16, b: i16) -> [i16; 4] {
    [t, l, b, r]
}

/// `SetupColliderSprite`. Layer 5; the phone and the wall guns start with
/// a random delay.
pub fn setup_collider(sp: &mut Sprite, rng: &mut Random) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Collider);
    sp.hit_task = Some(HitTask::Collider);
    sp.hit_tile_task = None;
    sp.timer = 0;
    sp.layer = 5;
    sp.state = 5;
    sp.hot_rect = [0; 4];
    sp.b8b = false;
    sp.f78 = -1;
    match sp.kind {
        1019 => sp.hot_rect = set_rect(8, 0, 12, 32),
        1029 => sp.hot_rect = set_rect(24, 0, 20, 32),
        1039 => sp.hot_rect = set_rect(0, 24, 12, 20),
        1049 => sp.hot_rect = set_rect(0, 8, 32, 12),
        2001 => sp.hot_rect = set_rect(2, 2, 14, 14),
        2301..=2304 => {
            sp.hot_rect = set_rect(4, 4, 28, 28);
            sp.b8b = true;
            sp.face = face::FAN;
            sp.f6e = FAN_UNSET;
            sp.f78 = -1;
        }
        3100 => {
            sp.face = face::PHONE;
            sp.b8b = false;
            sp.hot_rect = set_rect(-18, -18, 34, 34);
            sp.timer = -100 - rng.below(100);
        }
        9199 | 9200 => {
            sp.face = face::COPIER;
            sp.b8b = true;
            sp.hot_rect = set_rect(12, 0, 40, 30);
        }
        9300 => {
            sp.face = face::MAGNET;
            sp.hot_rect = set_rect(1, 1, 22, 22);
        }
        9401 | 9411 | 9421 | 9431 => {
            sp.face = face::GUN_UP + (sp.kind - 9401) / 10;
            sp.b8b = false;
            sp.hot_rect = set_rect(2, 2, 14, 14);
            sp.timer = rng.below(30);
        }
        9600 => {
            sp.hot_rect = set_rect(0, 4, 32, 30);
            sp.face = face::VENDING;
            sp.b8b = true;
        }
        9800 => {
            sp.hot_rect = set_rect(8, 0, 56, 64);
            sp.face = face::GATE;
            sp.b8b = true;
        }
        _ => {}
    }
    sp.at_from_pos();
}

/// What else `HandleColliderSprite` reads and changes.
pub struct ColliderWorld<'a> {
    pub level: &'a mut Level,
    /// Harry's position, ammo, caffeine and comfort.
    pub g: &'a mut PlayerGlobals,
    /// Harry's speed (the magnet pulls him down).
    pub harry_speed: &'a mut Speed,
    /// Frames until the magnets pull again (`_DAT_1006c4ac`, one for all).
    pub magnet: &'a mut i16,
    /// Is the fan sound playing?
    pub fan_sound_playing: bool,
    pub rng: &'a mut Random,
}

fn entry(level: &Level, index: i16) -> Option<&crate::level::LevelSprite> {
    usize::try_from(index).ok().and_then(|i| level.sprites.get(i))
}

fn entry_mut(level: &mut Level, index: i16) -> Option<&mut crate::level::LevelSprite> {
    usize::try_from(index).ok().and_then(|i| level.sprites.get_mut(i))
}

/// The vending machine's level entry: info #0 kicks until the next can
/// (2–4), info #1 cans left (−1 empty); unset ones get 2–4.
fn vending_defaults(level: &mut Level, index: i16, rng: &mut Random) {
    if let Some(e) = entry_mut(level, index) {
        if e.info[0] == 0 {
            e.info[0] = rng.below(3) + 2;
        }
        if e.info[1] == 0 {
            e.info[1] = rng.below(3) + 2;
        }
    }
}

/// `HandleColliderSprite`. Returns whether a fan blows this frame: then
/// the caller runs `repel` with it on every sprite in the list whose task
/// `is_repelled` (the original's `HandleRepulsions`, in list order).
pub fn handle_collider(sp: &mut Sprite, w: &mut ColliderWorld, events: &mut Vec<SpriteEvent>) -> bool {
    if sp.frozen {
        return false;
    }
    let mut blows = false;
    if !is_fan(sp.kind) {
        match sp.kind {
            3100 => phone(sp, w.rng, events),
            3101 => sp.face = face::PHONE,
            9200 => {
                if sp.timer > 0 {
                    sp.timer -= 1;
                }
            }
            9300 => magnet(sp, w, events),
            9401..=9449 => {
                if !(0..256).contains(&sp.level_index) {
                    return false;
                }
                gun(sp, w, events);
            }
            _ => {}
        }
    } else {
        let (ph, pv) = w.g.player_pos;
        let near = |a: i16, b: i16| (i32::from(a) - i32::from(b)).abs() < 448;
        if near(sp.at.1, ph) && near(sp.at.0, pv) {
            if sp.f6e == FAN_UNSET && (0..256).contains(&sp.level_index) {
                let e = &w.level.sprites[sp.level_index as usize];
                sp.f6e = if e.info[0] == 0 { 125 } else { i32::from(e.info[0]) };
                sp.timer = if e.info[1] == 0 { 256 } else { e.info[1] };
            }
            if !w.fan_sound_playing {
                events.push(SpriteEvent::Sound3D(snd::FAN));
                // the channel of the sound; the port has none
                sp.f78 = 0;
            }
            sp.dir += 1;
            if sp.dir > 3 {
                sp.dir = 0;
            }
            sp.face = face::FAN + sp.dir;
            blows = true;
        }
    }
    if sp.dir == 0 && sp.kind == 9600 {
        if (0..256).contains(&sp.level_index) {
            vending_defaults(w.level, sp.level_index, w.rng);
        }
        sp.dir = 1;
    }
    if sp.kind == 9800 && sp.timer > 0 {
        gate(sp, events);
    } else if sp.kind == 9600 {
        vending(sp, w, events);
    }
    blows
}

/// The phone: quiet for 100–299 frames (timer < 0), then rings 1–3 times
/// (90 frames each, timer counting down; the bell face flickers in the
/// last 30 frames of each ring).
fn phone(sp: &mut Sprite, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.timer < 0 {
        sp.timer += 1;
        if sp.timer == 0 {
            sp.timer = (rng.below(3) + 1) * 90;
        }
    } else if sp.timer > 0 {
        if matches!(sp.timer, 90 | 180 | 270) {
            events.push(SpriteEvent::Sound3D(snd::PHONE));
        }
        let t = sp.timer;
        if !((61..=89).contains(&t) || (151..=179).contains(&t) || (241..=269).contains(&t)) {
            sp.face = face::PHONE;
        } else if t & 2 != 0 {
            sp.face = if sp.face == face::PHONE_RING { face::PHONE } else { face::PHONE_RING };
        }
        sp.timer -= 1;
        if sp.timer == 0 {
            sp.timer = -100 - rng.below(200);
        }
    }
}

/// The magnet: Harry less than 96 px to the side of it and above it,
/// carrying any ammo, is pulled down (speed v 2500) every third frame and
/// loses 5 comfort (unless on caffeine).
fn magnet(sp: &Sprite, w: &mut ColliderWorld, events: &mut Vec<SpriteEvent>) {
    let g = &mut *w.g;
    let (ph, pv) = (i32::from(g.player_pos.0), i32::from(g.player_pos.1));
    let dh = (i32::from(sp.at.1) + 12 - (ph + 32)).abs();
    let armed = g.ammo.staple != 0 || g.ammo.badass != 0 || g.ammo.shrapnel != 0;
    if dh < 96 && pv + 32 < i32::from(sp.at.0) + 12 && armed {
        if *w.magnet == 0 {
            events.push(SpriteEvent::Sound3D(snd::MAGNET));
            w.harry_speed.v = 2500;
            *w.magnet = 2;
            if g.caffeine == 0 {
                g.comfort = g.comfort.wrapping_sub(5);
            }
        } else if *w.magnet > 0 {
            *w.magnet -= 1;
        }
    }
}

/// A wall gun: its entry's info #0 names the switch's entry (negated:
/// fires while the switch's info #1 is set, else while it is 0); it also
/// fires while its own info #1 is set. A shot every 60 frames. Ids other
/// than the four directions fire from registers the original never set
/// (none are in Episode 1); the port fires them from the gun's position,
/// standing still.
fn gun(sp: &mut Sprite, w: &mut ColliderWorld, events: &mut Vec<SpriteEvent>) {
    let level = &*w.level;
    let link = level.sprites[sp.level_index as usize].info[0];
    let switch = entry(level, link.wrapping_abs()).map_or(0, |e| e.info[1]);
    if sp.timer > 0 {
        sp.timer -= 1;
    }
    let on = if link < 0 { switch != 0 } else { switch == 0 };
    if (on || level.sprites[sp.level_index as usize].info[1] != 0) && sp.timer == 0 {
        let (dh, dv, speed) = match sp.kind {
            9401 => (2, -12, Speed { v: -1500, h: 0 }),
            9411 => (2, 8, Speed { v: 1500, h: 0 }),
            9421 => (12, 2, Speed { v: 0, h: 1500 }),
            9431 => (-12, 2, Speed { v: 0, h: -1500 }),
            _ => (0, 0, Speed::default()),
        };
        events.push(SpriteEvent::Sound3D(snd::GUN));
        events.push(SpriteEvent::Spawn {
            kind: GUN_SHOT,
            h: sp.at.1.wrapping_add(dh),
            v: sp.at.0.wrapping_add(dv),
            speed: Some(speed),
        });
        sp.timer = 60;
    }
}

/// The gate after item 11199 was used on it (timer > 0): no more hits,
/// rises 1 px a frame for 62 frames, then is gone.
fn gate(sp: &mut Sprite, events: &mut Vec<SpriteEvent>) {
    sp.hit_task = None;
    if sp.timer == 3 {
        events.push(SpriteEvent::Sound(snd::GATE));
    }
    sp.at.0 = sp.at.0.wrapping_sub(1);
    sp.timer += 1;
    if sp.timer > 64 {
        sp.frozen = true;
    }
}

/// The vending machine. A kick sets the timer to 3 (`HitPlayerSprite`);
/// it then counts down to 0 with the kicked face. On the kick's first
/// frame one kick is counted (info #0); the last one drops a can if there
/// are any left: 25 frames later a can (9610) flies out, speed v
/// 1500–2299, h −1000…999. (The kick frame reads the entry without
/// checking the index; a vending machine always has one.)
fn vending(sp: &mut Sprite, w: &mut ColliderWorld, events: &mut Vec<SpriteEvent>) {
    if sp.timer < 1 {
        if sp.timer < 0 {
            sp.timer += 1;
            if sp.timer == 0 {
                let v = w.rng.below(800) + 1500;
                let h = w.rng.below(2000) - 1000;
                events.push(SpriteEvent::Spawn {
                    kind: CAN,
                    h: sp.at.1.wrapping_add(12),
                    v: sp.at.0.wrapping_add(28),
                    speed: Some(Speed { v, h }),
                });
            }
        } else {
            sp.face = face::VENDING;
        }
        return;
    }
    if sp.timer == 3 {
        vending_kick(sp, w, events);
    }
    sp.face = face::VENDING_KICKED;
    sp.timer -= 1;
}

fn vending_kick(sp: &mut Sprite, w: &mut ColliderWorld, events: &mut Vec<SpriteEvent>) {
    let index = sp.level_index;
    let Some(e) = entry_mut(w.level, index) else { return };
    if e.info[0] > 0 {
        e.info[0] -= 1;
    }
    if e.info[0] != 0 || e.info[1] < 1 {
        events.push(SpriteEvent::Sound3D(snd::VENDING_EMPTY));
        return;
    }
    events.push(SpriteEvent::Sound3D(snd::VENDING_DROP));
    if e.info[0] == 0 {
        e.info[0] = w.rng.below(3) + 2;
    }
    sp.timer = -25;
    e.info[1] -= 1;
    if e.info[1] == 0 {
        e.info[1] = -1;
    }
}

/// The tasks a fan blows (`HandleRepulsions`): Harry, shots, plants,
/// barrels, chairs, robots, ghosts - and blobs and reversors, which come
/// with their kinds.
pub fn is_repelled(task: Task) -> bool {
    matches!(task, Task::Player | Task::PlayerShot | Task::EnemyShot | Task::Plant | Task::Barrel | Task::Chair | Task::Robot | Task::Ghost)
}

/// `SpriteCenter` (PPC 0x1001e458): the middle of the hot rect at `at`,
/// (v, h).
pub fn centre(sp: &Sprite) -> (i16, i16) {
    let mid = |a: i16, b: i16| ((i32::from(a) + i32::from(b)) >> 1) as i16;
    (
        sp.at.0.wrapping_add(mid(sp.hot_rect[0], sp.hot_rect[2])),
        sp.at.1.wrapping_add(mid(sp.hot_rect[1], sp.hot_rect[3])),
    )
}

/// One sprite (its centre `c`, (v, h)) in a fan's wind: in front of the
/// fan (at + 16), nearer along the wind than across it and nearer than
/// the reach (`f6e`), it is pushed by (reach − distance) × strength
/// (`timer`) / 256. 2301 blows up, 2302 down, 2303 left, 2304 right.
pub fn repel(fan: &Sprite, c: (i16, i16), speed: &mut Speed) {
    let (fv, fh) = (fan.at.0.wrapping_add(16), fan.at.1.wrapping_add(16));
    let (cv, ch) = c;
    let abs = |x: i16, y: i16| {
        let d = i32::from(x) - i32::from(y);
        (if d < 1 { -d } else { d }) as i16
    };
    let push = |dist: i16| (((fan.f6e - i32::from(dist)).wrapping_mul(i32::from(fan.timer)) as u32) >> 8) as i16;
    let (across, along, ahead) = match fan.kind {
        2301 => (abs(ch, fh), fv.wrapping_sub(cv), cv < fv),
        2302 => (abs(ch, fh), cv.wrapping_sub(fv), fv < cv),
        2303 => (abs(cv, fv), fh.wrapping_sub(ch), ch < fh),
        2304 => (abs(cv, fv), ch.wrapping_sub(fh), fh < ch),
        _ => return,
    };
    if !ahead || across >= along || i32::from(along) >= fan.f6e {
        return;
    }
    match fan.kind {
        2301 => speed.v = speed.v.wrapping_sub(push(along)),
        2302 => speed.v = speed.v.wrapping_add(push(along)),
        2303 => speed.h = speed.h.wrapping_sub(push(along)),
        _ => speed.h = speed.h.wrapping_add(push(along)),
    }
}

/// `CopierSave`: the copier's save dialog (frontend); true if saved.
pub type CopierSave<'a> = dyn FnMut() -> bool + 'a;

/// `HitColliderSprite`: Harry at the copier (9200, ready) saves the game -
/// its entry becomes 9199 (used) first, so the saved level has it used;
/// cancelled, the copier stays 9200 but waits 300 frames (and its entry
/// stays 9199: after leaving the level it is used up - BUGS.md). Harry
/// at the phone while the bell face shows answers it: a balloon with
/// phone words (mode 5), once.
#[allow(clippy::too_many_arguments)]
pub fn hit_collider(
    sp: &mut Sprite,
    other: &Sprite,
    level: &mut Level,
    words: &mut Words,
    rng: &mut Random,
    save: &mut CopierSave,
    show: &mut ShowBalloon,
) {
    if sp.frozen || other.frozen || other.task != Some(Task::Player) {
        return;
    }
    if sp.kind == 9200 && sp.timer == 0 {
        let index = sp.level_index;
        if let Some(e) = entry_mut(level, index) {
            e.id = 9199;
        }
        if save() {
            if let Some(e) = entry_mut(level, index) {
                e.id = 9199;
            }
            sp.kind = 9199;
        } else {
            if let Some(e) = entry_mut(level, index) {
                e.present = true;
            }
            sp.timer = 300;
        }
    } else if sp.kind == 3100 && sp.face == face::PHONE_RING {
        let text = BalloonText::Phonewords(words.phonewords(rng));
        if show(sp, text, 5) {
            sp.kind = 3101;
        }
    }
}
