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
    /// `HandlePlayerSpriteDeath` (not ported yet).
    PlayerDeath,
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
    Collider,
    DartEnemy,
    Ramp,
    Barrel,
    Chair,
    Stairs,
    Button,
    SlidingDoor,
    CoWorker,
    MailGuy,
    Robot,
    FirePipe,
    Grate,
    Ghost,
    Turret,
    Reversor,
    Boulder,
    Scientist,
    Blob,
    Boss,
    /// `HandleBossSpriteDeath`.
    BossDeath,
    /// `HandleDeskSprite`: does nothing.
    Desk,
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
    Collider,
    EnemyShot,
    DartEnemy,
    Ramp,
    Barrel,
    Chair,
    /// `HitStairsSprite`: does nothing.
    Stairs,
    Button,
    /// `HitSlidingDoorSprite`: does nothing.
    SlidingDoor,
    MailGuy,
    Robot,
    /// `HitFirePipeSprite`: does nothing.
    FirePipe,
    Grate,
    Ghost,
    Turret,
    Reversor,
    Boulder,
    Scientist,
    Blob,
    Boss,
    /// `HitDeskSprite`: does nothing.
    Desk,
}

/// A sprite's tile collision routine (`sp->hitTileTask`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTileTask {
    Player,
    PlayerShot,
    Plant,
    EnemyShot,
    Ramp,
    Barrel,
    Chair,
    MailGuy,
    Robot,
    Reversor,
    Boulder,
    Blob,
    /// `HitBossTileSprite`: does nothing.
    Boss,
}

/// What a sprite routine asks of the rest of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteEvent {
    /// `STPlayRegSound`.
    Sound(i16),
    /// `STPlayRegSound` at a volume other than 0x100.
    SoundVolume { id: i16, volume: i16 },
    /// `STPlay3DSound(snd, …, volume, at)`: placed at `at` (v, h), the
    /// point the original passes (mostly the sprite's `at`, `pos` or
    /// `SpriteCenter` as the call finds it); volume 0x100 = full.
    Sound3D { id: i16, at: (i16, i16), volume: i16 },
    /// `STPlay3DSoundPitched` at `at` (v, h), rate 0x10000 = as recorded.
    Sound3DPitched { id: i16, at: (i16, i16), rate: u32 },
    /// `EasyExplosion(h, v, 0, 0, a, b)`.
    Explosion { h: i16, v: i16, a: i16, b: i16 },
    /// `EasyExplosion(h, v, 0, 0, a, b)` and its timer set.
    ExplosionAt { h: i16, v: i16, a: i16, b: i16, timer: i16 },
    /// `EasyExplosion(h, v, speed h, speed v, a, b)`: a moving one.
    MovingExplosion { h: i16, v: i16, speed: Speed, a: i16, b: i16 },
    /// `STPlayDynamicSound` at the sprite: a sound whose stereo position
    /// moves on with `speed` (px a frame) while it plays.
    DynamicSound { id: i16, volume: i16, at: (i16, i16), speed: Speed },
    /// `SATNewSprite(kind, h, v, SetupEnemyShotSprite)`, then speed and
    /// `dir` set - and, if given, `fix`, `timer` and `f7c`. With `to_fix`
    /// `pos` and `at` move to `fix` too (the robots); otherwise they stay
    /// until the shot's first frame (the boss).
    Shot { kind: i16, h: i16, v: i16, speed: Speed, dir: i16, fix: Option<(i32, i32)>, to_fix: bool, timer: Option<i16>, f7c: Option<i16> },
    /// `GenerateSprite(h, v, kind, −1)`: an object of the level's kinds,
    /// without a level entry.
    Generate { kind: i16, h: i16, v: i16 },
    /// A fire pipe's flame: `SATNewSprite(pipe, h, v, SetupEffectSprite)`,
    /// then the kind set to `kind` and `f7c` to `pipe`.
    Flame { pipe: i16, h: i16, v: i16, kind: i16 },
    /// `SATNewSprite(kind, h, v, SetupGhostSprite)`, then `dir` set and no
    /// hit points (the boss's ghosts).
    Ghost { kind: i16, h: i16, v: i16, dir: i16 },
    /// Every ghost in the list loses its hit points (−5).
    GhostsDie,
    /// `FadeMusic(0, 1)`.
    FadeMusic,
    /// `FUN_10056c0c`: stop every playing instance of a sound.
    StopSound(i16),
    /// The boss's last blast: an explosion (kind 0, frame 0) on layer 500
    /// at twice the size.
    BigBlast { h: i16, v: i16 },
    /// Points for Harry.
    Score(i32),
    /// Level entry `i` is cleared (byte 0): the sprite will not come back.
    Forget(i16),
    /// `SATNewSprite(kind, h, v, setup)`: a new sprite with the setup of
    /// its kind (enemy shots, the soda can item), then its speed set if
    /// the caller sets one.
    Spawn { kind: i16, h: i16, v: i16, speed: Option<Speed> },
}

impl SpriteEvent {
    /// `STPlay3DSound` at full volume at `at` (v, h).
    pub fn sound_3d(id: i16, at: (i16, i16)) -> SpriteEvent {
        SpriteEvent::Sound3D { id, at, volume: 0x100 }
    }
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
    /// +0x6e, per kind (a mode, a counter, a reach; see each module).
    pub f6e: i32,
    pub state: i16,
    pub timer: i16,
    /// Slot in the idle sprite table, −1 none.
    pub idle_index: i16,
    /// +0x78, per kind (the fan's sound, the vending machine's lit
    /// state, the enemy shot's slow count).
    pub f78: i32,
    /// +0x7c, per kind (the robots' mode, the enemy shot's countdown to
    /// homing / leaving its gun, a flame's fire pipe kind).
    pub f7c: i16,
    pub face: Face,
    /// +0x82 / +0x84: `at` (v, h) as the last frame was drawn
    /// (`EraseSprites`).
    pub last_at: (i16, i16),
    /// +0x86: the face as the last frame was drawn.
    pub last_face: i32,
    /// +0x8a, a flag of several uses: hit and drawn tinted (cleared by
    /// `EraseSprites`; set by hits and some draw procs), the held
    /// weapon's charging.
    pub b8a: bool,
    /// +0x8b, a flag of several uses: drawn behind the foreground tiles
    /// (effects, props, the fire pipe), "started" (buttons, boulders, the
    /// mail guy and others on their first frame), the held weapon's
    /// second flag. See each module.
    pub b8b: bool,
    /// +0x8c: kept with an idle sprite; nothing in the port reads it.
    pub b8c: bool,
    /// Jump state (see `air`).
    pub air: i16,
    pub flip: i16,
    /// Dead: the task does nothing, the sprite leaves the list when the
    /// frame is drawn.
    pub frozen: bool,
    /// +0x94: frames before a ramp can throw it again (`Rider::cooldown`).
    pub ramp_cooldown: i16,
    /// Frames until an explosion hurts it again.
    pub blast_cooldown: i16,
    /// (h, v) of the last `half_pipe`.
    pub last_tile: (i16, i16),
    pub slide: i16,
    /// SAT's `hotRect2` (+0x0e): the hot rect at `pos`, taken right after
    /// the sprite's task; the collision pass uses it.
    pub hot_rect2: Rect,
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
        self.last_at.0 = 0;
        self.last_at.1 = 0;
        self.last_face = 0;
        self.b8a = false;
        self.b8b = false;
        self.b8c = false;
        self.scale = [0x100; 4];
        self.flip = 0;
        self.air = 0;
        self.level_index = -1;
        self.frozen = false;
        self.ramp_cooldown = 0;
        self.blast_cooldown = 0;
        self.speed = Speed::default();
    }

    /// `SpriteCenter` (PPC 0x1001e458): the middle of the hot rect at
    /// `at`, (v, h).
    pub fn centre(&self) -> (i16, i16) {
        let mid = |a: i16, b: i16| ((i32::from(a) + i32::from(b)) >> 1) as i16;
        (self.at.0.wrapping_add(mid(self.hot_rect[0], self.hot_rect[2])), self.at.1.wrapping_add(mid(self.hot_rect[1], self.hot_rect[3])))
    }

    /// `at` = `pos`.
    pub fn at_from_pos(&mut self) {
        self.at = self.pos;
    }

    /// `fix` = `at` << 8.
    pub fn fix_from_at(&mut self) {
        self.fix = crate::bounce::fix_of(self.at);
    }

    /// `fix` = `pos` << 8.
    pub fn fix_from_pos(&mut self) {
        self.fix = crate::bounce::fix_of(self.pos);
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
