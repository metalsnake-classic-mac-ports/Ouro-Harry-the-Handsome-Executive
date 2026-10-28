//! The player's shots: the weapon Harry holds (`HandleHeldWeaponSprite`,
//! `FindHeldWeaponPos`), the shots it makes (`MakeNewShot`,
//! `AssignShotSpeed`) and the shot sprites (`SetupPlayerShotSprite`,
//! `HandlePlayerShotSprite`). Firing itself is in `keys`: it sets
//! `gShotKind` and `gRecoil` = 3; the frame counts the recoil down and the
//! held weapon shoots while it is above 1 - once per firing.

use crate::bounce::{Body, Rect, wall_bounce};
use crate::frame::PlayerGlobals;
use crate::inventory::{current_item, item};
use crate::mask::{Mask, mask_hit};
use crate::physics::Speed;
use crate::random::Random;
use crate::sprite::{Sprite, SpriteEvent, Task};
use crate::tiles::{TileHit, fg_rect, offset, sect};

/// A shot's speed (h, v) per direction (`AssignShotSpeed`; 2200 · cos /
/// sin of the direction, truncated - captured from the original).
pub const SHOT_SPEEDS: [(i16, i16); 36] = [
    (2200, 0), (2167, -382), (2068, -752), (1905, -1100), (1685, -1414), (1414, -1685),
    (1100, -1905), (752, -2068), (382, -2167), (0, -2200), (-382, -2167), (-752, -2068),
    (-1100, -1905), (-1414, -1685), (-1685, -1414), (-1905, -1100), (-2068, -752), (-2167, -382),
    (-2200, 0), (-2167, 382), (-2068, 752), (-1905, 1100), (-1685, 1414), (-1414, 1685),
    (-1100, 1905), (-752, 2068), (-382, 2167), (0, 2200), (382, 2167), (752, 2068),
    (1100, 1905), (1414, 1685), (1685, 1414), (1905, 1100), (2068, 752), (2167, 382),
];

/// Where Harry holds the weapon (h, v) relative to his face's top-left,
/// per direction (`FindHeldWeaponPos`).
pub const WEAPON_OFFSETS: [(i16, i16); 36] = [
    (39, 40), (41, 38), (42, 37), (43, 35), (43, 33), (43, 31), (43, 29), (42, 28), (41, 25),
    (40, 24), (39, 22), (37, 21), (35, 20), (33, 20), (31, 20), (30, 20), (27, 20), (24, 21),
    (24, 23), (22, 24), (21, 26), (20, 27), (20, 30), (20, 32), (20, 33), (20, 36), (21, 38),
    (22, 39), (24, 41), (26, 42), (27, 43), (30, 43), (31, 43), (34, 43), (35, 43), (37, 42),
];

/// What the held weapon needs to know about Harry.
#[derive(Clone, Copy, Debug)]
pub struct Holder {
    /// `atV`, `atH`.
    pub at: (i16, i16),
    pub scale0: i16,
    pub scale1: i16,
    /// The bounds (top, left, bottom, right) of his current face.
    pub face_bounds: Rect,
}

/// The held weapon sprite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HeldWeapon {
    /// (v, h).
    pub pos: (i16, i16),
    pub at: (i16, i16),
    /// Face: `dir` (+ 36 while the recoil lasts); `None` when Harry holds
    /// no gun.
    pub face: Option<i16>,
    /// Which gun is drawn (`PTR_DAT_100660dc`): 0 staple gun, 1
    /// B.A.D.A.S.S., 2 shrapnel gun; kept when he holds none.
    pub gun: u8,
    /// +0x8a: set while the use key charges.
    pub charging: bool,
    /// +0x8b.
    pub flag8b: bool,
    pub frozen: bool,
}

/// A shot sprite to create (`SATNewSprite` with `SetupPlayerShotSprite`,
/// then this speed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewShot {
    /// 0 staple / shrapnel pellet, 1 B.A.D.A.S.S., 4 soda can, 10 the
    /// power of the swivel (made by `keys`).
    pub kind: i16,
    pub h: i16,
    pub v: i16,
    pub speed: Speed,
}

/// Sounds and explosions of the weapon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponEvent {
    Sound(i16),
    /// `STPlay3DSound` at the shot's position.
    Sound3D(i16),
    /// `EasyExplosion(h, v, 0, 0, a, b)`.
    Explosion { h: i16, v: i16, a: i16, b: i16 },
}

/// `FindScaledPosAndBounds` (position part): the top-left of a face of
/// `bounds` drawn at `at` with the scale, centred on the unscaled face.
pub fn scaled_pos(at: (i16, i16), bounds: Rect, scale_h: i16, scale_v: i16) -> (i16, i16) {
    let shrink = |len: i16, scale: i16| {
        let scaled = ((i32::from(scale) * i32::from(len)) >> 8) as i16;
        ((i32::from(len) - i32::from(scaled)) >> 1) as i16
    };
    let w = bounds[3].wrapping_sub(bounds[1]);
    let h = bounds[2].wrapping_sub(bounds[0]);
    (at.0.wrapping_add(shrink(h, scale_v)), at.1.wrapping_add(shrink(w, scale_h)))
}

/// `FindHeldWeaponPos`: the weapon's position (v, h) for Harry facing
/// `dir`.
pub fn held_weapon_pos(holder: &Holder, dir: i16) -> (i16, i16) {
    let base = if holder.scale0 == 0x100 {
        holder.at
    } else {
        // the original passes scale0 for both axes here
        scaled_pos(holder.at, holder.face_bounds, holder.scale0, holder.scale0)
    };
    let (oh, ov) = WEAPON_OFFSETS[dir as usize];
    let h = i32::from(base.1) + ((i32::from(holder.scale0) * i32::from(oh)) >> 8);
    let v = i32::from(base.0) + ((i32::from(holder.scale1) * i32::from(ov)) >> 8);
    ((v as i16).wrapping_sub(6), (h as i16).wrapping_sub(6))
}

/// `HandleHeldWeaponSprite`.
pub fn handle_held_weapon(
    w: &mut HeldWeapon,
    holder: &Holder,
    g: &mut PlayerGlobals,
    rng: &mut Random,
    shots: &mut Vec<NewShot>,
    events: &mut Vec<WeaponEvent>,
) {
    if w.frozen {
        return;
    }
    w.pos = held_weapon_pos(holder, g.player_dir);
    w.at = w.pos;
    let gun = match current_item(g) {
        item::STAPLE_GUN => Some(0),
        item::BADASS => Some(1),
        item::SHRAPNEL_GUN => Some(2),
        _ => None,
    };
    w.face = gun.map(|_| g.player_dir + if g.recoil > 0 { 36 } else { 0 });
    if let Some(gun) = gun {
        w.gun = gun;
    }
    if g.recoil > 1 {
        make_new_shot(w, g, rng, shots, events);
    }
    if g.fire_hold > 0 {
        w.charging = true;
    }
    w.flag8b = true;
}

/// `MakeNewShot`: the shots of `gShotKind` from the weapon's muzzle.
fn make_new_shot(w: &HeldWeapon, g: &PlayerGlobals, rng: &mut Random, shots: &mut Vec<NewShot>, events: &mut Vec<WeaponEvent>) {
    let (sh, sv) = SHOT_SPEEDS[g.player_dir as usize];
    let base = Speed { h: sh, v: sv };
    let (v, h) = w.pos;
    // a pellet with a random spread: 4 px of position, ±256 of speed
    let mut pellet = |kind: i16, sign_h: i16, sign_v: i16| {
        let dv = rng.below(4);
        let dh = rng.below(4);
        let mut speed = base;
        speed.h = speed.h.wrapping_add(rng.below(0x200)).wrapping_sub(0x100);
        speed.v = speed.v.wrapping_add(rng.below(0x200)).wrapping_sub(0x100);
        NewShot {
            kind,
            h: h.wrapping_add(2).wrapping_add(sign_h.wrapping_mul(dh)),
            v: v.wrapping_add(2).wrapping_add(sign_v.wrapping_mul(dv)),
            speed,
        }
    };
    match g.shot_kind {
        k @ (0 | 1 | 4) => {
            let mut speed = base;
            if k == 1 {
                speed.h = speed.h.wrapping_add(rng.below(0x400)).wrapping_sub(0x200);
                speed.v = speed.v.wrapping_add(rng.below(0x400)).wrapping_sub(0x200);
            }
            shots.push(NewShot { kind: k, h: h.wrapping_add(2), v: v.wrapping_add(2), speed });
        }
        // the shrapnel gun: 2, 3 or 4 pellets
        k @ 5..=7 => {
            for _ in 0..k - 3 {
                let s = pellet(0, 1, -1);
                shots.push(s);
            }
        }
        2 if g.ammo.badass < 1 => {
            // out of ammo: it blows up in his hands
            events.push(WeaponEvent::Explosion { h: w.at.1.wrapping_sub(26), v: w.at.0.wrapping_sub(26), a: 12, b: 2 });
            events.push(WeaponEvent::Sound(602));
            events.push(WeaponEvent::Sound(239));
        }
        2 => {
            for (sign_h, sign_v) in [(1, -1), (1, 1), (-1, -1), (-1, 1)] {
                let s = pellet(1, sign_h, sign_v);
                shots.push(s);
            }
        }
        _ => {}
    }
}

/// The face of a shot sprite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShotFace {
    #[default]
    None,
    /// Frame 0–6 of the shot's own animation (kinds 0, 1, 4).
    Frame(i16),
    /// The power of the swivel flies as a spinning Harry: his face `n`.
    Harry(i16),
}

/// A player shot sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shot {
    pub kind: i16,
    /// (v, h).
    pub pos: (i16, i16),
    pub at: (i16, i16),
    /// (h, v), 24.8.
    pub fix: (i32, i32),
    pub speed: Speed,
    /// The animation frame (0–6), or Harry's face for kind 10.
    pub dir: i16,
    /// Frames until the shot can hit walls (3 at the start).
    pub timer: i16,
    pub face: ShotFace,
    pub hot_rect: Rect,
    /// +0x8b: set at the start, cleared when the timer runs out, set again
    /// by a wall contact.
    pub flag8b: bool,
    /// Done (it hit something).
    pub frozen: bool,
}

/// `SATNewSprite` + `SetupPlayerShotSprite` for a shot of `kind` at (v,
/// h), with the speed and direction its maker gives it.
pub fn new_shot(kind: i16, pos: (i16, i16), speed: Speed, dir: i16) -> Shot {
    Shot {
        kind,
        pos,
        at: pos,
        fix: (i32::from(pos.1) << 8, i32::from(pos.0) << 8),
        speed,
        dir,
        timer: 3,
        face: ShotFace::None,
        hot_rect: if kind < 10 { [-2, -2, 10, 10] } else { [27, 27, 37, 37] },
        flag8b: true,
        frozen: false,
    }
}

/// `HandlePlayerShotSprite`: animate and fly.
pub fn handle_shot(s: &mut Shot) {
    if s.frozen {
        return;
    }
    if s.timer > 0 {
        s.timer -= 1;
        if s.timer == 0 {
            s.flag8b = false;
        }
    }
    if s.kind < 10 {
        s.dir = s.dir.wrapping_add(1);
        if s.dir > 6 {
            s.dir = 0;
        }
        if matches!(s.kind, 0 | 1 | 4) {
            s.face = ShotFace::Frame(s.dir);
        }
    } else {
        s.dir = s.dir.wrapping_add(2);
        if s.dir > 35 {
            s.dir = 0;
        }
        s.face = ShotFace::Harry(s.dir);
    }
    s.fix.0 = s.fix.0.wrapping_add(i32::from(s.speed.h));
    s.fix.1 = s.fix.1.wrapping_add(i32::from(s.speed.v));
    let (h, v) = ((s.fix.0 >> 8) as i16, (s.fix.1 >> 8) as i16);
    s.pos = (v, h);
    s.at = (v, h);
}

/// `HitPlayerShotTileSprite`: a shot touches a tile (from
/// `tiles::touched_tiles`). Only foreground tiles stop it, and only once
/// its timer has run out: a small rect at the shot must overlap the
/// tile's shape (kinds up to 0x37), then `WallBounce` with the shot's mask
/// (face 4021, `masks::from_face`) must report contact. The shot stops
/// (`frozen`); a soda can explodes. The touched tiles of the frame are
/// all still handled, so a shot can stop - with its sound - more than once.
pub fn hit_shot_tile(s: &mut Shot, shot_mask: &Mask, walls: &[Mask], hit: TileHit, events: &mut Vec<WeaponEvent>) {
    s.flag8b = true;
    if !hit.foreground {
        return;
    }
    let kind = hit.kind;
    let wall = kind <= 0x37;
    // the right edge is taken from atV, not atH (BUGS.md)
    let me = [s.at.0.wrapping_add(4), s.at.1.wrapping_add(4), s.at.0.wrapping_add(6), s.at.0.wrapping_add(6)];
    if wall && !sect(me, offset(fg_rect(kind), hit.tile.0, hit.tile.1)) {
        return;
    }
    if s.timer >= 1 {
        return;
    }
    if wall {
        let mut body = Body { pos: s.pos, at: s.at, fix: s.fix, speed: s.speed };
        // the table has 32 wall shapes; higher kinds read past it (as in
        // `hit_player_tile`; never in Episode 1)
        let wall_mask = usize::try_from(kind).ok().and_then(|k| walls.get(k));
        let r = wall_bounce(&mut body, kind, hit.tile, (4, 4), 1, s.hot_rect, false, &mut |b| match wall_mask {
            Some(w) => mask_hit(&mut b.pos, shot_mask, hit.tile, w),
            None => false,
        });
        (s.pos, s.at, s.fix, s.speed) = (body.pos, body.at, body.fix, body.speed);
        if r as u8 == 0 {
            return;
        }
    }
    s.frozen = true;
    if s.kind == 4 {
        events.push(WeaponEvent::Sound(500));
        events.push(WeaponEvent::Explosion { h: s.pos.1.wrapping_sub(28), v: s.pos.0.wrapping_sub(28), a: 14, b: 0 });
    } else {
        events.push(WeaponEvent::Sound3D(4700));
    }
}

/// `HitPlayerShotSprite` (the shot is `sp`): it stops at colliders (and
/// sliding doors; 3-D sound 4700) and at dart enemies (co-workers,
/// scientists), which costs Harry 2 corporate favor and, half of the
/// time, makes them say one of four things; the struck sprite flinches
/// (layer 10, +0x8a). A soda can (kind 4) bursts on anything but Harry,
/// his weapon and effects: sound 500 and an explosion (kind 14) at its
/// position − 28. Robots, blobs and grates take damage here too - with
/// their kinds.
pub fn hit_player_shot(sp: &mut Sprite, other: &mut Sprite, favor: &mut i16, rng: &mut Random, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen {
        return;
    }
    match other.task {
        Some(Task::Collider | Task::SlidingDoor) => {
            events.push(SpriteEvent::Sound3D(4700));
            sp.frozen = true;
        }
        Some(Task::DartEnemy) => {
            sp.frozen = true;
            *favor = favor.wrapping_sub(2);
            if rng.below(100) < 50 {
                let remark = match rng.below(4) {
                    0 => 253,
                    1 => 254,
                    2 => 255,
                    _ => 256,
                };
                events.push(SpriteEvent::Sound3D(remark));
            }
            other.layer = 10;
            other.b8a = true;
        }
        _ => {}
    }
    if sp.kind == 4 && !matches!(other.task, Some(Task::Player | Task::HeldWeapon | Task::Effect)) {
        sp.frozen = true;
        events.push(SpriteEvent::Sound(500));
        events.push(SpriteEvent::Explosion { h: sp.pos.1.wrapping_sub(28), v: sp.pos.0.wrapping_sub(28), a: 14, b: 0 });
    }
}

/// HrSp ids of the faces: the held weapon (`dir`, `dir` + 36 recoiling)
/// and the shots (staple 0, B.A.D.A.S.S. 1, soda can 4: 7 frames each).
pub mod face {
    pub const WEAPON: i16 = 3501;
    pub const WEAPON_RECOIL: i16 = 3551;
    pub const STAPLE: i16 = 4001;
    pub const BADASS: i16 = 4011;
    pub const CAN: i16 = 3061;
}

/// The HrSp id of a held weapon face.
pub fn weapon_face(f: Option<i16>) -> i16 {
    match f {
        Some(f) if f >= 36 => face::WEAPON_RECOIL + f - 36,
        Some(f) => face::WEAPON + f,
        None => 0,
    }
}

impl Shot {
    /// The shot from its record.
    pub fn from_sprite(r: &Sprite) -> Shot {
        let face = match (r.kind, r.face) {
            (_, 0) => ShotFace::None,
            (10.., f) => ShotFace::Harry(f - crate::game::PLAYER_FACES),
            (_, _) => ShotFace::Frame(r.dir),
        };
        Shot {
            kind: r.kind,
            pos: r.pos,
            at: r.at,
            fix: r.fix,
            speed: r.speed,
            dir: r.dir,
            timer: r.timer,
            face,
            hot_rect: r.hot_rect,
            flag8b: r.b8b,
            frozen: r.frozen,
        }
    }

    /// Back into its record.
    pub fn into_sprite(&self, r: &mut Sprite) {
        (r.kind, r.pos, r.at, r.fix, r.speed, r.dir, r.timer, r.hot_rect, r.b8b, r.frozen) =
            (self.kind, self.pos, self.at, self.fix, self.speed, self.dir, self.timer, self.hot_rect, self.flag8b, self.frozen);
        r.face = match self.face {
            ShotFace::None => 0,
            ShotFace::Harry(f) => crate::game::PLAYER_FACES + f,
            ShotFace::Frame(n) => match self.kind {
                0 => face::STAPLE + n,
                1 => face::BADASS + n,
                4 => face::CAN + n,
                _ => r.face,
            },
        };
    }
}

/// `SetupPlayerShotSprite` on a record (as `new_shot`).
pub fn setup_player_shot(sp: &mut Sprite) {
    sp.init();
    let s = new_shot(sp.kind, sp.pos, Speed::default(), 0);
    s.into_sprite(sp);
    sp.layer = 11;
    sp.task = Some(Task::PlayerShot);
    sp.hit_task = Some(crate::sprite::HitTask::PlayerShot);
    sp.hit_tile_task = Some(crate::sprite::HitTileTask::PlayerShot);
    sp.face = 0;
}
