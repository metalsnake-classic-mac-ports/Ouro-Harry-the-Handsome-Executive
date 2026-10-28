//! The sprite record (0xa2 bytes in the original, `re/ghidra/sprite.txt`).
//! Every kind uses the same record and reads other sprites' fields, so the
//! port keeps it as one struct with the original's fields; fields whose
//! meaning depends on the kind keep their offset as name (`f6e`, ...).

use crate::air::Air;
use crate::bounce::{Body, Rect};
use crate::physics::Speed;

/// A face: the `HrSp` resource id of the picture, 0 = none.
pub type Face = i16;

/// A sprite's per-frame routine (`sp->task`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    Player,
    HeldWeapon,
    PlayerShot,
    Bonus,
    Item,
    Exit,
    Door,
    EnemyShot,
    Effect,
    Cooler,
    Plant,
    Message,
}

/// A sprite's collision routine (`sp->hitTask`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTask {
    Player,
    PlayerShot,
    Bonus,
    Item,
    Exit,
    Door,
    Cooler,
    Plant,
    Message,
}

/// A sprite's tile collision routine (`sp->hitTileTask`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTileTask {
    Player,
    PlayerShot,
    Plant,
}

/// What a sprite routine asks of the rest of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteEvent {
    /// `STPlayRegSound`.
    Sound(i16),
    /// `STPlay3DSound` at the sprite.
    Sound3D(i16),
    /// `EasyExplosion(h, v, 0, 0, a, b)`.
    Explosion { h: i16, v: i16, a: i16, b: i16 },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sprite {
    /// The editor's object id (SAT's kind).
    pub kind: i16,
    /// SAT's position (v, h).
    pub pos: (i16, i16),
    pub hot_rect: Rect,
    pub task: Option<Task>,
    pub hit_task: Option<HitTask>,
    pub hit_tile_task: Option<HitTileTask>,
    /// Drawing and list order (SAT sorts by it).
    pub layer: i16,
    pub speed: Speed,
    /// +0x52…+0x58, 0x100 = 1.0.
    pub scale: [i16; 4],
    /// The working position (v, h) the collision code moves.
    pub at: (i16, i16),
    /// (h, v), 24.8.
    pub fix: (i32, i32),
    pub dir: i16,
    /// Index in the level's sprite list, −1 none.
    pub level_index: i16,
    /// +0x6a: a message waiting to be shown to the player (the original
    /// keeps a pointer to his sprite).
    pub balloon_pending: bool,
    pub f6e: i32,
    pub state: i16,
    pub timer: i16,
    /// Slot in the idle sprite table, −1 none.
    pub idle_index: i16,
    pub f78: i32,
    pub f7c: i16,
    pub face: Face,
    pub f82: i16,
    pub f84: i16,
    pub f86: i32,
    pub b8a: bool,
    pub b8b: bool,
    pub b8c: bool,
    /// Jump state (see `air`).
    pub air: i16,
    pub flip: i16,
    /// Dead: the task does nothing, the sprite leaves the list when the
    /// frame is drawn.
    pub frozen: bool,
    pub f94: i16,
    /// Frames until an explosion hurts it again.
    pub blast_cooldown: i16,
    /// (h, v) of the last `half_pipe`.
    pub last_tile: (i16, i16),
    pub slide: i16,
}

impl Sprite {
    /// `SATNewSprite` on a fresh record: kind and position set, the rest
    /// zero. (A record from SAT's free list keeps the old sprite's values
    /// in the fields neither SAT nor the setup set.)
    pub fn new(kind: i16, h: i16, v: i16) -> Sprite {
        Sprite { kind, pos: (v, h), ..Default::default() }
    }

    /// `InitSprite` (PPC 0x1001e380), called by every setup routine.
    pub fn init(&mut self) {
        self.state = 0;
        self.timer = 0;
        self.idle_index = -1;
        self.f78 = 0;
        self.f7c = 0;
        self.face = 0;
        self.f82 = 0;
        self.f84 = 0;
        self.f86 = 0;
        self.b8a = false;
        self.b8b = false;
        self.b8c = false;
        self.scale = [0x100; 4];
        self.flip = 0;
        self.air = 0;
        self.level_index = -1;
        self.frozen = false;
        self.f94 = 0;
        self.blast_cooldown = 0;
        self.speed = Speed::default();
    }

    /// `at` = `pos`.
    pub fn at_from_pos(&mut self) {
        self.at = self.pos;
    }

    /// `fix` = `at` << 8.
    pub fn fix_from_at(&mut self) {
        self.fix = (i32::from(self.at.1) << 8, i32::from(self.at.0) << 8);
    }

    /// The moving part, as the bounce and jump code takes it.
    pub fn body(&self) -> Body {
        Body { pos: self.pos, at: self.at, fix: self.fix, speed: self.speed }
    }

    pub fn set_body(&mut self, b: Body) {
        (self.pos, self.at, self.fix, self.speed) = (b.pos, b.at, b.fix, b.speed);
    }

    /// The jump state, as `air` takes it (`scale` = scale0).
    pub fn air_state(&self) -> Air {
        Air { air: self.air, slide: self.slide, last_tile: self.last_tile, flip: self.flip, scale: self.scale[0] }
    }

    /// Back from `air_state`; a changed scale goes to scale0 and scale1.
    pub fn set_air_state(&mut self, a: Air) {
        if a.scale != self.scale[0] {
            self.scale[0] = a.scale;
            self.scale[1] = a.scale;
        }
        (self.air, self.slide, self.last_tile, self.flip) = (a.air, a.slide, a.last_tile, a.flip);
    }

    /// `fix` += speed, `pos` = `at` = fix >> 8.
    pub fn integrate(&mut self) {
        self.fix.0 = self.fix.0.wrapping_add(i32::from(self.speed.h));
        self.fix.1 = self.fix.1.wrapping_add(i32::from(self.speed.v));
        let (h, v) = ((self.fix.0 >> 8) as i16, (self.fix.1 >> 8) as i16);
        self.pos = (v, h);
        self.at = (v, h);
    }
}
