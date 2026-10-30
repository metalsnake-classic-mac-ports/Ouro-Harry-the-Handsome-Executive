//! One pass of the main loop (`GameLoop`, PPC 0x1000959c) over a level's
//! sprites: `HandleIdleSprites`, `HandleSprites` (SAT's run: every task,
//! the sort pass, the collision pass with the hit tasks; then
//! `CheckPlayerCollision`), `FindUpperLeftCorner`, `CheckTileCollisions`,
//! the bookkeeping of `EraseSprites`, the meters and `CheckGameEvents`.
//!
//! Harry, his weapon and every other sprite are records in the
//! [`World`]; the routines ported with their own structs (`Harry`,
//! `HeldWeapon`) get a copy made from the record and written back.
//!
//! Not yet here (their kinds come later): player shots and what they hit,
//! effects (explosions), Harry's death, the level's ripple and the frontend
//! parts (sounds, drawing, `ShowBalloon`, `CopierSave` - passed in).

use crate::air::Air;
use crate::bounce::Rect;
use crate::collider::{ColliderWorld, CopierSave, centre, handle_collider, hit_collider, is_repelled, repel};
use crate::dart_enemy::{handle_dart_enemy, hit_dart_enemy};
use crate::frame::SLOTS;
use crate::inventory::{EMPTY, item, remove_item, switch_item};
use crate::keys::Ammo;
use crate::death::{DeathEvent, handle_player_death};
use crate::enemy_shot::{ShotMasks, handle_enemy_shot, hit_enemy_shot, hit_enemy_shot_tile};
use crate::button::{handle_button, handle_sliding_door, hit_button};
use crate::furniture::{self, FurnitureMasks, handle_barrel, handle_chair, hit_barrel, hit_chair};
use crate::frame::{FrameSound, Harry, PlayerGlobals, handle_player};
use crate::keys::{Bindings, KeyEvent, KeyInput, KeyMap, handle_keys};
use crate::kinds::{
    BalloonText, CoolerMasks, LevelCounter, ObjectMasks, PlantHitMasks, handle_bonus, handle_cooler, handle_door, handle_item,
    handle_message, handle_plant, hit_cooler, hit_door, hit_exit, hit_message, hit_object_tile, hit_plant, hit_plant_tile,
};
use crate::level::Level;
use crate::mask::Mask;
use crate::physics::Physics;
use crate::player::{Masks, Player, TileSound, hit_player_tile};
use crate::ramp::{RampMasks, handle_ramp, hit_ramp, hit_ramp_tile};
use crate::random::Random;
use crate::sat::{absolute, collisions, sort_pass};
use crate::effect::handle_effect;
use crate::shots::{HeldWeapon, Holder, Shot, WeaponEvent, handle_held_weapon, handle_shot, hit_player_shot, hit_shot_tile, weapon_face};
use crate::sprite::{Face, HitTask, HitTileTask, Sprite, SpriteEvent, Task};
use crate::tiles::{sect, touched_tiles};
use crate::touch::{TouchEvent, TouchWorld, touch};
use crate::view::{View, find_upper_left_corner};
use crate::air::AirSound;
use crate::moves::MoveSound;
use crate::sound::{Channel, RATE, SoundEvent, snd, stereo};
use crate::words::Words;
use crate::world::{Setup, World};

/// The scroll routine's origin (`FUN_100251c8`, `_DAT_1006596c/70`): the
/// view's corner (v, h), not below 0 and not past 3648.
fn scroll_origin(view: (i16, i16)) -> (i32, i32) {
    let clamp = |x: i16| {
        let x = x.max(0);
        if (i32::from(x.wrapping_add(480)) / 32) > 128 {
            3648
        } else {
            i32::from(x)
        }
    };
    (clamp(view.0), clamp(view.1))
}

/// How often `WrapDrawSprite` calls the draw proc for a face of `size`
/// (width, height) at `pos` (v, h): never unless it reaches into the view
/// from the origin; then once, and again for each wrap of the 480-px ring
/// buffer it crosses (a scaled face that crosses the right edge goes
/// through the scale port, once).
fn wrap_draws(pos: (i16, i16), size: (i16, i16), origin: (i32, i32), unscaled: bool) -> usize {
    let (ov, oh) = origin;
    let (mut y, mut x) = (i32::from(pos.0), i32::from(pos.1));
    let (mut w, mut hgt) = (i32::from(size.0), i32::from(size.1));
    if !(oh <= x + w && ov <= y + hgt && x <= oh + 448 && y <= ov + 448) {
        return 0;
    }
    if ov + 448 < y + hgt {
        hgt = ov + 448 - y;
    }
    if oh + 480 < x + w {
        w = oh + 480 - x;
    }
    if x < oh {
        w -= oh - x;
        x = oh;
    }
    if y < ov {
        hgt -= ov - y;
        y = ov;
    }
    let (w, hgt) = (i32::from(w as i16), i32::from(hgt as i16));
    let y = i32::from((y as i16) % 480);
    let x = i32::from((x as i16) % 480);
    if unscaled || (x >= 0 && x + w < 480) {
        let down = y - 480 + hgt > 0;
        let right = x - 480 + w > 0;
        1 + usize::from(down) + usize::from(right) + usize::from(right && down)
    } else {
        1
    }
}

/// Harry's faces: HrSp 1101 + pose (`dir` + 36 × pose set).
pub const PLAYER_FACES: Face = 1101;

/// The preferences a frame reads.
#[derive(Clone, Copy, Debug)]
pub struct Prefs {
    /// prefs +8: the scroll mode (`view::mode`); above 2 also keeps a
    /// message balloon that did not fit pending.
    pub scroll: i16,
    /// prefs +2 and +3.
    pub even_v: bool,
    pub blit: bool,
    pub bindings: Bindings,
}

/// The masks and face sizes the routines need.
pub struct Assets<'a> {
    pub player: &'a Masks,
    /// Plant and chair (cicn 599), barrel and cooler (cicn 598).
    pub plant: &'a Mask,
    pub barrel: &'a Mask,
    /// The cooler's face mask (HrSp 2002).
    pub cooler: &'a Mask,
    /// The enemy shots' mask (cicn 1299).
    pub shot: &'a Mask,
    /// The ramps' masks (cicn 571–574).
    pub ramps: [&'a Mask; 4],
    /// The player shots' mask (HrSp 4021).
    pub player_shot: &'a Mask,
    /// The robots' and reversors' wall mask (HrSp 4251, cicn 598 encoded,
    /// so with the faces' mirrored bits) and the blobs' (HrSp 1451).
    pub robot: &'a Mask,
    pub blob: &'a Mask,
    /// The bounds (top, left, bottom, right) of a face; (0, 0, 0, 0) for
    /// none.
    pub face_bounds: &'a dyn Fn(Face) -> Rect,
}

/// The state of a level being played.
pub struct Game {
    pub world: World,
    pub level: Level,
    pub g: PlayerGlobals,
    pub view: View,
    pub physics: Physics,
    pub rng: Random,
    pub prefs: Prefs,
    pub counter: LevelCounter,
    /// The counter as `CheckGameEvents` saw it last (`_DAT_10072722`).
    pub seen_level: i16,
    /// `PTR_DAT_10066118`, cleared by every plant's task.
    pub plant_flag: bool,
    /// `PTR_DAT_1006619c` / `PTR_DAT_10066198`: a robot bounced off a wall
    /// / an obstacle since the last robot's task (`robot::RobotWorld`).
    pub robot_flags: (bool, bool),
    /// The same for the reversors (`PTR_DAT_10066188` / `…84`).
    pub reversor_flags: (bool, bool),
    /// And for the blobs (`PTR_DAT_10065f00` / `…efc`).
    pub blob_flags: (bool, bool),
    /// `_DAT_1006c4ac`, the magnets' delay: 2 at launch (data section),
    /// never reset, so it carries over from level to level.
    pub magnet: i16,
    /// The drawn gun (`PTR_DAT_100660dc`).
    pub gun: u8,
    /// Harry's record (`*_DAT_10065980`), set by `start`.
    pub harry: Option<usize>,
    /// The sentence generators' word lists and last picks.
    pub words: Words,
    /// The sounds of the last frame, in order.
    pub sounds: Vec<SoundEvent>,
    /// What the level started with (what a saved game keeps).
    pub level_start: LevelStart,
}

/// The state at a level's start, set by `UpLevel` (and the copier's save):
/// comfort `_DAT_10065b1c`, surface `PTR_DAT_10065b04`, favor
/// `PTR_DAT_10065b10`, score `_DAT_10065af0`, ammunition `_DAT_10065ae0` /
/// `…adc` / `…ad8`, the inventory copy at `_DAT_10065ac0`. A new game
/// starts with 72 / 72 / 72, no score, no ammunition, no items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelStart {
    pub score: i32,
    pub comfort: i16,
    pub surface: i16,
    pub favor: i16,
    pub ammo: Ammo,
    pub inventory: [(i16, i16); SLOTS],
}

impl Default for LevelStart {
    fn default() -> Self {
        LevelStart {
            score: 0,
            comfort: 72,
            surface: 72,
            favor: 72,
            ammo: Ammo::default(),
            inventory: [(EMPTY, 0); SLOTS],
        }
    }
}

impl LevelStart {
    fn of(g: &PlayerGlobals) -> Self {
        LevelStart {
            score: g.score,
            comfort: g.comfort,
            surface: g.surface,
            favor: g.favor,
            ammo: g.ammo,
            inventory: g.inventory,
        }
    }
}

/// How `UpLevel` ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpLevel {
    /// The next level is set up and the game loop goes on.
    Next,
    /// The counter is past the world's last level: `Win`, the loop ends.
    Won,
}

/// What a `ShowBalloon` call ended with: (shown, mode, the sprite's `pos`).
type BalloonCall = (bool, i16, (i16, i16));

/// A balloon the game asks the frontend to show (`ShowBalloon(sprite,
/// text, mode)`), with what the frontend needs to place it: the sprite's
/// `at` (v, h), the view's corner (v, h) and Harry's top-left (h, v) at
/// that moment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Balloon {
    pub text: BalloonText,
    pub mode: i16,
    pub at: (i16, i16),
    pub view: (i16, i16),
    pub player: (i16, i16),
}

/// The frontend's `ShowBalloon`: show the balloon if it fits (and wait for
/// a key) and tell whether it did.
pub type ShowBalloonFn<'a> = dyn FnMut(&Balloon) -> bool + 'a;

/// What a frame ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameEnd {
    Running,
    /// The level counter changed (`UpLevel`).
    LevelDone,
    /// Harry's death ran its course (`HandlePlayerSpriteDeath` set the
    /// "player off" flag, `_DAT_10065acc`).
    Died,
}

/// Harry as the ported player routines take him, from his record.
pub fn harry_from(r: &Sprite) -> Harry {
    Harry {
        player: Player {
            body: r.body(),
            dir: r.dir,
            face: r.face.wrapping_sub(PLAYER_FACES),
            air: r.air,
            hot_rect: r.hot_rect,
        },
        air: Air { air: r.air, slide: r.slide, last_tile: r.last_tile, flip: r.flip, scale: r.scale[0] },
        frozen: r.frozen,
        dead: false,
        blast_cooldown: r.blast_cooldown,
        ramp_cooldown: r.f94,
    }
}

/// Write Harry back into his record.
pub fn harry_into(h: &Harry, r: &mut Sprite) {
    let p = &h.player;
    r.set_body(p.body);
    r.dir = p.dir;
    r.face = p.face.wrapping_add(PLAYER_FACES);
    r.hot_rect = p.hot_rect;
    r.set_air_state(h.air);
    r.air = p.air;
    r.frozen = h.frozen;
    r.blast_cooldown = h.blast_cooldown;
    r.f94 = h.ramp_cooldown;
}

/// `SetupPlayerSprite` on a new record at (h, v): kind 69, layer 10, the
/// direction from `gPlayerDir` mod 35 (sic: 35, so 35 becomes 0), his
/// normal hot rect; the move timers, pose, impulse, wetness, ripple and
/// comfort delta cleared. (The inventory rebuild after a restored game and
/// the cooldown `_DAT_10065e68` are not ported.)
pub fn setup_player(sp: &mut Sprite, g: &mut PlayerGlobals) {
    sp.init();
    sp.kind = 69;
    sp.dir = g.player_dir % 35;
    sp.speed = Default::default();
    sp.layer = 10;
    g.tile.wet = 0;
    sp.task = Some(Task::Player);
    sp.hit_task = None;
    sp.hit_tile_task = Some(HitTileTask::Player);
    sp.fix = (i32::from(sp.pos.1) << 8, i32::from(sp.pos.0) << 8);
    sp.face = PLAYER_FACES + sp.dir;
    g.moves.inch = 0;
    g.moves.kick = 0;
    g.moves.scoot = 0;
    g.moves.pose = 0;
    g.moves.impulse = 0;
    sp.hot_rect = crate::player::HOT_RECT;
    sp.b8b = true;
    g.wavy_on = false;
    g.tile.comfort_delta = 0;
}

/// `SetupHeldWeaponSprite`: layer 11, hot rect (−4, −4, 16, 16), the
/// weapon's place for `gPlayerDir`. It sets no hit task (SAT cleared it).
pub fn setup_held_weapon(sp: &mut Sprite, holder: &Holder, g: &PlayerGlobals) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::HeldWeapon);
    sp.hit_tile_task = None;
    sp.layer = 11;
    sp.hot_rect = [-4, -4, 16, 16];
    sp.face = crate::shots::face::WEAPON + g.player_dir;
    sp.pos = crate::shots::held_weapon_pos(holder, g.player_dir);
    sp.at = sp.pos;
}

impl Game {
    /// The level start of `GameLoop`: `SetupLevelSprites`, Harry at (h,
    /// v) and his weapon (both at the front of the list), the view and
    /// its target at (h, v) − 208, then one `FindUpperLeftCorner`.
    /// Returns the object ids the port cannot make yet.
    pub fn start(&mut self, h: i16, v: i16, dir: i16, assets: &Assets) -> Vec<i16> {
        self.world.sprites.player = self.g.player_pos;
        let missing = self.world.setup_level_sprites(&self.level, &mut self.rng);
        let p = self.world.sprites.new_sprite(0, h, v);
        self.harry = Some(p);
        setup_player(&mut self.world.sprites.recs[p], &mut self.g);
        self.world.sprites.recs[p].dir = dir;
        self.g.player_pos = (h.wrapping_sub(32), v.wrapping_sub(32));
        let corner = (v.wrapping_sub(208), h.wrapping_sub(208));
        self.view.target = corner;
        self.view.corner = corner;
        let holder = self.holder(p, assets);
        let w = self.world.sprites.new_sprite(0, h, v);
        setup_held_weapon(&mut self.world.sprites.recs[w], &holder, &self.g);
        find_upper_left_corner(&mut self.view, self.g.player_pos, self.prefs.scroll, self.prefs.even_v, self.prefs.blit);
        self.seen_level = self.counter.current;
        missing
    }

    /// `UpLevel`, which `CheckGameEvents` calls when the level counter
    /// changed (after a frame that ended with `FrameEnd::LevelDone`;
    /// `levels` are the world's). The jingle 4800; the debug items
    /// 11397–11399 go; past the last level the game is won. Otherwise
    /// (counter −255: the level after the one before) the score grows by
    /// (level − 1) × 5000, the state is kept as the level's start, every
    /// sprite is killed and the next level set up as the loop goes on:
    /// Harry's position at its start − 32, the view and its target at the
    /// start − 240, `SetupLevelSprites`, Harry there with the start
    /// direction, his weapon at the start itself. Not here: the pictures
    /// before and after (`ScrollPlot`), the autosave (prefs +1), the
    /// music, and the unregistered game's stop after level 5
    /// (`LevelLimit`) - the port plays as registered. Returns the object
    /// ids the port cannot make yet with `Next`.
    pub fn up_level(&mut self, levels: &[Level], assets: &Assets) -> (UpLevel, Vec<i16>) {
        self.play(snd::LEVEL_DONE, 0x100, None, RATE, Channel::Effect);
        let g = &mut self.g;
        let mut again = true;
        while again {
            again = false;
            for s in 0..SLOTS {
                let (id, n) = g.inventory[s];
                if (item::DEBUG_ITEM..=item::DEBUG_ITEM + 2).contains(&id) && n > 0 {
                    switch_item(g, s as i16);
                    remove_item(g, s as i16);
                    again = true;
                }
            }
        }
        let won = (levels.len() as i16) < self.counter.current;
        if self.counter.current == -255 {
            self.counter.current = self.counter.previous.wrapping_add(1);
        }
        let Some(level) = usize::try_from(self.counter.current - 1).ok().and_then(|n| levels.get(n)).filter(|_| !won) else {
            self.seen_level = self.counter.current;
            return (UpLevel::Won, Vec::new());
        };
        g.score = g.score.wrapping_add(i32::from(self.counter.current - 1) * 5000);
        self.level_start = LevelStart::of(g);
        self.world.sprites.kill_all();
        self.level = level.clone();
        self.physics = self.level.effective_physics();
        let (sh, sv) = self.level.start;
        self.g.player_pos = (sh.wrapping_sub(32), sv.wrapping_sub(32));
        let corner = (sv.wrapping_sub(240), sh.wrapping_sub(240));
        self.view.target = corner;
        self.view.corner = corner;
        self.world.sprites.player = self.g.player_pos;
        let missing = self.world.setup_level_sprites(&self.level, &mut self.rng);
        let p = self.world.sprites.new_sprite(0, sh.wrapping_sub(32), sv.wrapping_sub(32));
        self.harry = Some(p);
        setup_player(&mut self.world.sprites.recs[p], &mut self.g);
        self.world.sprites.recs[p].dir = self.level.start_dir;
        let holder = self.holder(p, assets);
        let w = self.world.sprites.new_sprite(0, sh, sv);
        setup_held_weapon(&mut self.world.sprites.recs[w], &holder, &self.g);
        self.seen_level = self.counter.current;
        (UpLevel::Next, missing)
    }

    fn player(&self) -> Option<usize> {
        self.world.sprites.list.iter().copied().find(|&i| self.world.rec(i).task == Some(Task::Player))
    }

    fn holder(&self, p: usize, assets: &Assets) -> Holder {
        let r = self.world.rec(p);
        Holder { at: r.at, scale0: r.scale[0], scale1: r.scale[1], face_bounds: (assets.face_bounds)(r.face) }
    }

    /// A sprite a task asked for (at the front of the list).
    fn spawn(&mut self, kind: i16, h: i16, v: i16, speed: Option<crate::physics::Speed>) {
        let setup = if (11001..=19999).contains(&kind) { Setup::Item } else { Setup::EnemyShot };
        self.spawn_with(kind, h, v, setup, speed);
    }

    fn spawn_with(&mut self, kind: i16, h: i16, v: i16, setup: Setup, speed: Option<crate::physics::Speed>) -> usize {
        self.world.sprites.player = self.g.player_pos;
        let i = self.world.sprites.spawn(kind, h, v, setup, &mut self.rng);
        let r = &mut self.world.sprites.recs[i];
        r.hot_rect2 = absolute(r.hot_rect, r.pos);
        if let Some(s) = speed {
            r.speed = s;
        }
        i
    }

    /// `EasyExplosion(h, v, 0, 0, kind, dir)`.
    fn explosion(&mut self, h: i16, v: i16, kind: i16, dir: i16) -> usize {
        let i = self.spawn_with(kind, h, v, Setup::Effect, Some(Default::default()));
        let r = &mut self.world.sprites.recs[i];
        r.dir = dir;
        r.kind = kind;
        i
    }

    /// The events of a sprite's routine; its 3-D sounds at `at`.
    fn sprite_events(&mut self, events: Vec<SpriteEvent>, at: (i16, i16)) {
        for e in events {
            match e {
                SpriteEvent::Spawn { kind, h, v, speed } => self.spawn(kind, h, v, speed),
                SpriteEvent::Explosion { h, v, a, b } => {
                    self.explosion(h, v, a, b);
                }
                SpriteEvent::MovingExplosion { h, v, speed, a, b } => {
                    let i = self.explosion(h, v, a, b);
                    self.world.sprites.recs[i].speed = speed;
                }
                SpriteEvent::DynamicSound { id, volume, speed: _ } => {
                    // the port does not move the sound with the sprite yet
                    self.play(id, volume, Some(at), RATE, Channel::Effect);
                }
                SpriteEvent::Shot { kind, h, v, speed, dir, fix, to_fix, timer, f7c } => {
                    let i = self.spawn_with(kind, h, v, Setup::EnemyShot, Some(speed));
                    let r = &mut self.world.sprites.recs[i];
                    r.dir = dir;
                    if let Some(t) = timer {
                        r.timer = t;
                    }
                    if let Some(f) = f7c {
                        r.f7c = f;
                    }
                    if let Some(f) = fix {
                        r.fix = f;
                        if to_fix {
                            r.pos = ((f.1 >> 8) as i16, (f.0 >> 8) as i16);
                            r.at = r.pos;
                        }
                    }
                }
                SpriteEvent::Generate { kind, h, v } => {
                    self.world.sprites.player = self.g.player_pos;
                    self.world.generate_sprite(h, v, kind, -1, &mut self.rng);
                }
                SpriteEvent::Flame { pipe, h, v, kind } => {
                    let i = self.spawn_with(pipe, h, v, Setup::Effect, None);
                    let r = &mut self.world.sprites.recs[i];
                    r.kind = kind;
                    r.f7c = pipe;
                }
                SpriteEvent::ExplosionAt { h, v, a, b, timer } => {
                    let i = self.explosion(h, v, a, b);
                    self.world.sprites.recs[i].timer = timer;
                }
                SpriteEvent::Ghost { kind, h, v, dir } => {
                    let i = self.spawn_with(kind, h, v, Setup::Ghost, None);
                    let r = &mut self.world.sprites.recs[i];
                    r.dir = dir;
                    r.state = 0;
                }
                SpriteEvent::GhostsDie => {
                    for &k in &self.world.sprites.list {
                        let r = &mut self.world.sprites.recs[k];
                        if r.task == Some(Task::Ghost) {
                            r.state = -5;
                        }
                    }
                }
                // the music is not ported yet
                SpriteEvent::FadeMusic => {}
                SpriteEvent::StopSound(id) => self.sounds.push(SoundEvent::Stop(id)),
                SpriteEvent::BigBlast { h, v } => {
                    let i = self.explosion(h, v, 0, 0);
                    let r = &mut self.world.sprites.recs[i];
                    r.layer = 500;
                    r.scale[0] = 0x200;
                    r.scale[1] = 0x200;
                }
                SpriteEvent::Score(n) => self.g.score = self.g.score.wrapping_add(n),
                SpriteEvent::Forget(i) => {
                    if let Some(e) = usize::try_from(i).ok().and_then(|i| self.level.sprites.get_mut(i)) {
                        e.present = false;
                    }
                }
                SpriteEvent::Sound3DPitched { id, rate } => self.play(id, 0x100, Some(at), rate, Channel::Effect),
                SpriteEvent::Sound(id) => self.play(id, 0x100, None, RATE, Channel::Effect),
                SpriteEvent::Sound3D(id) => self.play(id, 0x100, Some(at), RATE, Channel::Effect),
            }
        }
    }

    /// What Harry's death asks for.
    fn death_events(&mut self, events: Vec<DeathEvent>) {
        for e in events {
            match e {
                // the music is not ported yet
                DeathEvent::FadeMusic => {}
                DeathEvent::Sound(id) => self.play(id, 0x100, None, RATE, Channel::Effect),
                DeathEvent::Explosion { h, v, dir, behind } => {
                    let i = self.explosion(h, v, 0, dir);
                    if behind {
                        self.world.sprites.recs[i].b8b = true;
                    }
                }
                DeathEvent::Flame { kind, h, v } => {
                    let i = self.spawn_with(kind, h, v, Setup::Effect, None);
                    let r = &mut self.world.sprites.recs[i];
                    r.f6e = 3;
                    r.f7c = kind;
                    r.b8b = true;
                }
                DeathEvent::Shake(s) => self.view.shake = s,
            }
        }
    }

    fn weapon_events(&mut self, events: Vec<WeaponEvent>, at: (i16, i16)) {
        for e in events {
            match e {
                WeaponEvent::Explosion { h, v, a, b } => {
                    self.explosion(h, v, a, b);
                }
                WeaponEvent::Sound(id) => self.play(id, 0x100, None, RATE, Channel::Effect),
                WeaponEvent::Sound3D(id) => self.play(id, 0x100, Some(at), RATE, Channel::Effect),
            }
        }
    }

    /// A sound: `STPlayRegSound` (`at` none) or `STPlay3DSound(Pitched)`.
    fn play(&mut self, id: i16, volume: i16, at: Option<(i16, i16)>, rate: u32, channel: Channel) {
        let (left, right) = match at {
            Some(at) => stereo(self.view.corner, at, volume),
            None => (volume.max(0) as u16, volume.max(0) as u16),
        };
        self.sounds.push(SoundEvent::Play { id, left, right, rate, channel });
    }

    /// Harry's sounds of his frame.
    fn player_sounds(&mut self, sounds: Vec<FrameSound>) {
        for s in sounds {
            match s {
                FrameSound::Move(MoveSound::ScootWater { volume }) => self.play(snd::SCOOT_WET, volume, None, RATE, Channel::Effect),
                FrameSound::Move(MoveSound::ScootFloor { kind, pitch }) => {
                    if let Some(&id) = usize::try_from(kind - 1).ok().and_then(|k| snd::SCOOT.get(k)) {
                        self.play(id, pitch, None, RATE, Channel::Effect);
                    }
                }
                FrameSound::Move(MoveSound::KickHit) => self.play(snd::KICK_HIT, 0x100, None, RATE, Channel::Effect),
                FrameSound::Air(a) => {
                    let id = match a {
                        AirSound::HalfPipeOut => snd::HALF_PIPE_OUT,
                        AirSound::Land => snd::LAND,
                        AirSound::Bounce => snd::BOUNCE,
                        AirSound::Fall => snd::FALL,
                        AirSound::Return => snd::RETURN,
                    };
                    self.play(id, 0x100, None, RATE, Channel::Effect);
                }
                FrameSound::Chatter(n) => {
                    if let Some(&id) = snd::CHATTER.get(n as usize) {
                        self.play(id, 0x100, None, RATE, Channel::Chatter);
                    }
                }
            }
        }
    }

    /// One frame. `keys` are the keys held this frame.
    pub fn frame(&mut self, keys: &KeyMap, assets: &Assets, show: &mut ShowBalloonFn, save: &mut CopierSave) -> FrameEnd {
        self.sounds.clear();
        self.world.handle_idle_sprites(self.view.corner);
        // SATRun: the tasks in list order
        let mut k = 0;
        let mut died = false;
        while k < self.world.sprites.list.len() {
            let i = self.world.sprites.list[k];
            if self.world.rec(i).task.is_some() {
                died |= self.task(i, keys, assets, show);
            }
            let r = &mut self.world.sprites.recs[i];
            r.hot_rect2 = absolute(r.hot_rect, r.pos);
            // tasks put new sprites at the front: find this one again
            k = self.world.sprites.list.iter().position(|&x| x == i).unwrap_or(k) + 1;
        }
        let recs = &self.world.sprites.recs;
        sort_pass(&mut self.world.sprites.list, |i| recs[i].layer);
        let order = self.world.sprites.list.clone();
        let mut pairs = Vec::new();
        collisions(&order, |i| recs[i].layer, |i| recs[i].hot_rect2, |a, b| pairs.push((a, b)));
        for (a, b) in pairs {
            if self.world.rec(a).hit_task.is_some() {
                self.hit(a, b, assets, show, save);
            }
            if self.world.rec(b).hit_task.is_some() {
                self.hit(b, a, assets, show, save);
            }
        }
        for &i in &order {
            let r = &mut self.world.sprites.recs[i];
            if r.task.is_none() {
                r.kind = 0;
                r.hit_task = None;
            }
        }
        self.check_player_collision(assets);
        find_upper_left_corner(&mut self.view, self.g.player_pos, self.prefs.scroll, self.prefs.even_v, self.prefs.blit);
        self.check_tile_collisions(assets);
        self.world.erase_sprites(&mut self.level);
        self.draw_pass(assets);
        self.update_meters();
        // CheckGameEvents
        if self.seen_level != self.counter.current {
            self.seen_level = self.counter.current;
            return FrameEnd::LevelDone;
        }
        if died { FrameEnd::Died } else { FrameEnd::Running }
    }

    /// `UpdateMeters(1)`'s state part: the three meters clamped to 0–144
    /// when one differs from its shown value, the shown values one step
    /// closer.
    fn update_meters(&mut self) {
        let g = &mut self.g;
        if g.comfort == g.comfort_shown && g.surface == g.surface_shown && g.favor == g.favor_shown {
            return;
        }
        for x in [&mut g.comfort, &mut g.surface, &mut g.favor] {
            *x = (*x).clamp(0, 144);
        }
        let step = |v: i16, s: &mut i16| {
            if v < *s {
                *s -= 1;
            } else if *s < v {
                *s += 1;
            }
        };
        step(g.comfort, &mut g.comfort_shown);
        step(g.surface, &mut g.surface_shown);
        step(g.favor, &mut g.favor_shown);
    }

    /// Run sprite `i`'s task. Returns whether Harry's death is over.
    fn task(&mut self, i: usize, keys: &KeyMap, assets: &Assets, show: &mut ShowBalloonFn) -> bool {
        let mut sp = std::mem::take(&mut self.world.sprites.recs[i]);
        let mut events = Vec::new();
        let mut died = false;
        match sp.task {
            Some(Task::Player) => {
                let mut h = harry_from(&sp);
                let input = KeyInput { keys, bindings: &self.prefs.bindings, debug: false, gulp_playing: false, voice_playing: false };
                let mut key_events = Vec::new();
                let mut sounds: Vec<FrameSound> = Vec::new();
                let effects = handle_player(
                    &mut h,
                    &mut self.g,
                    &self.level,
                    &self.physics,
                    &mut self.rng,
                    |h, g, rng| handle_keys(h, g, &input, rng, &mut key_events),
                    &mut sounds,
                );
                harry_into(&h, &mut sp);
                if h.dead {
                    sp.task = Some(Task::PlayerDeath);
                    sp.timer = 0;
                }
                if let Some(c) = effects.recentre {
                    // JumpScale moves the view onto Harry where he comes back
                    self.view.corner = c;
                    self.view.target = c;
                }
                let at = sp.at;
                self.world.sprites.recs[i] = std::mem::take(&mut sp);
                for e in key_events {
                    match e {
                        KeyEvent::Shot { h, v, dir, speed } => {
                            let s = self.spawn_with(10, h, v, Setup::PlayerShot, Some(speed));
                            self.world.sprites.recs[s].dir = dir;
                        }
                        KeyEvent::Explosion { h, v, flag } => {
                            self.explosion(h, v, flag, 0);
                        }
                        KeyEvent::Ripple(_) => panic!("the ripple is not ported into the frame yet"),
                        KeyEvent::Sound(id) => self.play(id, 0x100, None, RATE, Channel::Effect),
                        KeyEvent::ChargeSound(id) => self.play(id, 0x100, None, RATE, Channel::Charge),
                        KeyEvent::StopCharge => self.sounds.push(SoundEvent::StopCharge),
                        KeyEvent::VoiceSound(id) => self.play(id, 0x100, None, RATE, Channel::Voice),
                        KeyEvent::Sound3D { snd, pitch } => self.play(snd, 0x100, Some(at), u32::from(pitch), Channel::Effect),
                        KeyEvent::Balloon(_) => {}
                    }
                }
                self.player_sounds(sounds);
                sp = std::mem::take(&mut self.world.sprites.recs[i]);
            }
            Some(Task::HeldWeapon) => {
                let holder = match self.harry {
                    Some(p) => self.holder(p, assets),
                    None => Holder { at: (0, 0), scale0: 0x100, scale1: 0x100, face_bounds: [0; 4] },
                };
                let mut w = HeldWeapon {
                    pos: sp.pos,
                    at: sp.at,
                    face: None,
                    gun: self.gun,
                    charging: sp.b8a,
                    flag8b: sp.b8b,
                    frozen: sp.frozen,
                };
                let (mut shots, mut wev) = (Vec::new(), Vec::new());
                let frozen = w.frozen;
                handle_held_weapon(&mut w, &holder, &mut self.g, &mut self.rng, &mut shots, &mut wev);
                if !frozen {
                    (sp.pos, sp.at, sp.b8a, sp.b8b) = (w.pos, w.at, w.charging, w.flag8b);
                    sp.face = weapon_face(w.face);
                }
                self.gun = w.gun;
                let at = sp.at;
                self.world.sprites.recs[i] = std::mem::take(&mut sp);
                for s in shots {
                    self.spawn_with(s.kind, s.h, s.v, Setup::PlayerShot, Some(s.speed));
                }
                self.weapon_events(wev, at);
                sp = std::mem::take(&mut self.world.sprites.recs[i]);
            }
            Some(Task::Bonus) => handle_bonus(&mut sp),
            Some(Task::Item) => handle_item(&mut sp),
            Some(Task::Door) => handle_door(&mut sp, &mut self.level),
            Some(Task::Cooler) => handle_cooler(&mut sp, &mut self.level, &mut events),
            Some(Task::Plant) => {
                let mut s = Vec::new();
                handle_plant(&mut sp, &self.level, &self.physics, &mut self.rng, &mut self.plant_flag, &mut s);
                for a in s {
                    // HandleHalfPipe / JumpScale with the bump sound (none for falling)
                    if a != AirSound::Fall {
                        self.play(snd::PLANT, 0x100, None, RATE, Channel::Effect);
                    }
                }
            }
            Some(Task::Barrel) => {
                let mut s = Vec::new();
                handle_barrel(&mut sp, &mut self.level, &self.physics, &mut self.rng, &mut self.view.shake, 0, &mut events, &mut s);
                for a in s {
                    // JumpScale with the bump sound; HandleHalfPipe with none
                    if matches!(a, AirSound::Land | AirSound::Bounce) {
                        self.play(furniture::snd::BARREL_BUMP, 0x100, None, RATE, Channel::Effect);
                    }
                }
            }
            Some(Task::Chair) => {
                let mut s = Vec::new();
                handle_chair(&mut sp, &self.level, &self.physics, &mut self.rng, 0, &mut s);
                for a in s {
                    let id = match a {
                        AirSound::HalfPipeOut => furniture::snd::CHAIR_BUMP,
                        AirSound::Land | AirSound::Bounce => furniture::snd::CHAIR_KICK,
                        _ => continue,
                    };
                    self.play(id, 0x100, None, RATE, Channel::Effect);
                }
            }
            Some(Task::Message) => {
                let (view, player) = (self.view.corner, self.g.player_pos);
                let mut calls = Vec::new();
                handle_message(&mut sp, &self.level, &mut self.words, &mut self.rng, &mut |s, t, m| {
                    let r = show(&Balloon { text: t, mode: m, at: s.at, view, player });
                    calls.push((r, m, s.pos));
                    r
                });
                self.balloons(calls, sp.at);
            }
            Some(Task::Collider) => {
                let p = self.harry;
                let mut speed = p.map(|p| self.world.rec(p).speed).unwrap_or_default();
                let mut w = ColliderWorld {
                    level: &mut self.level,
                    g: &mut self.g,
                    harry_speed: &mut speed,
                    magnet: &mut self.magnet,
                    fan_sound_playing: false,
                    rng: &mut self.rng,
                };
                let blows = handle_collider(&mut sp, &mut w, &mut events);
                if let Some(p) = p {
                    self.world.sprites.recs[p].speed = speed;
                }
                if blows {
                    for &t in &self.world.sprites.list {
                        let r = &mut self.world.sprites.recs[t];
                        if r.task.is_some_and(is_repelled) {
                            let c = centre(r);
                            repel(&sp, c, &mut r.speed);
                        }
                    }
                }
            }
            Some(Task::DartEnemy) => handle_dart_enemy(&mut sp, self.view.corner, &self.level, &mut self.rng, &mut events),
            Some(Task::Ramp) => handle_ramp(&mut sp, &self.level, &self.physics),
            Some(Task::EnemyShot) => {
                let harry = self.harry.map_or((0, 0), |p| centre(self.world.rec(p)));
                handle_enemy_shot(&mut sp, harry, &mut events);
            }
            Some(Task::PlayerShot) => {
                let mut s = Shot::from_sprite(&sp);
                handle_shot(&mut s);
                s.into_sprite(&mut sp);
            }
            Some(Task::Effect) => handle_effect(&mut sp),
            Some(Task::Boss) => {
                let (view, player) = (self.view.corner, self.g.player_pos);
                let mut boss = self.world.sprites.boss;
                crate::boss::handle_boss(&mut sp, &mut boss, &self.level, view, player, &mut self.rng, &mut self.view.shake, &mut events);
                self.world.sprites.boss = boss;
            }
            Some(Task::BossDeath) => {
                let mut boss = self.world.sprites.boss;
                crate::boss::handle_boss_death(&mut sp, &mut boss, &mut self.rng, &mut self.view.shake, &mut events);
                self.world.sprites.boss = boss;
            }
            Some(Task::Scientist) => {
                let (view, player) = (self.view.corner, self.g.player_pos);
                crate::scientist::handle_scientist(&mut sp, view, player, &mut self.rng, &mut events);
            }
            Some(Task::Blob) => {
                let drop = usize::try_from(sp.level_index).ok().filter(|&i| i < 256).map_or(0, |i| self.level.sprites[i].info[3]);
                let (mut wall, mut obstacle) = self.blob_flags;
                let mut s = Vec::new();
                crate::blob::handle_blob(&mut sp, &self.level, self.g.player_pos, drop, &mut self.rng, &mut wall, &mut obstacle, &mut events, &mut s);
                self.blob_flags = (wall, obstacle);
                for a in s {
                    if matches!(a, AirSound::HalfPipeOut | AirSound::Land | AirSound::Bounce) {
                        self.play(crate::blob::snd::HIT, 0x100, None, RATE, Channel::Effect);
                    }
                }
            }
            Some(Task::Reversor) => {
                let (mut wall, mut obstacle) = self.reversor_flags;
                let mut s = Vec::new();
                crate::reversor::handle_reversor(&mut sp, &self.level, &mut self.rng, &mut wall, &mut obstacle, &mut s);
                self.reversor_flags = (wall, obstacle);
                for a in s {
                    if matches!(a, AirSound::Land | AirSound::Bounce) {
                        self.play(crate::robot::snd::HIT, 0x100, None, RATE, Channel::Effect);
                    }
                }
            }
            Some(Task::Boulder) => crate::boulder::handle_boulder(&mut sp, &mut self.level, &mut self.rng, &mut events),
            Some(Task::Turret) => {
                let drop = usize::try_from(sp.level_index).ok().filter(|&i| i < 256).map_or(0, |i| self.level.sprites[i].info[3]);
                crate::turret::handle_turret(&mut sp, self.g.player_pos, drop, &mut self.rng, &mut events);
            }
            Some(Task::Grate) => crate::grate::handle_grate(&mut sp, &mut events),
            Some(Task::Ghost) => {
                let (view, player) = (self.view.corner, self.g.player_pos);
                crate::ghost::handle_ghost(&mut sp, view, player, &mut self.rng, &mut events);
            }
            Some(Task::FirePipe) => {
                let info1 = usize::try_from(sp.level_index).ok().filter(|&i| i < 256).map(|i| self.level.sprites[i].info[1]);
                crate::firepipe::handle_fire_pipe(&mut sp, info1, &mut self.rng, &mut events);
            }
            Some(Task::Robot) => {
                let (mut wall, mut obstacle) = self.robot_flags;
                let mut s = Vec::new();
                let mut w = crate::robot::RobotWorld {
                    level: &mut self.level,
                    g: &mut self.g,
                    rng: &mut self.rng,
                    wall_done: &mut wall,
                    obstacle_done: &mut obstacle,
                };
                crate::robot::handle_robot(&mut sp, &mut w, 0, &mut events, &mut s);
                self.robot_flags = (wall, obstacle);
                for a in s {
                    // JumpScale with the hit sound
                    if matches!(a, AirSound::Land | AirSound::Bounce) {
                        self.play(crate::robot::snd::HIT, 0x100, None, RATE, Channel::Effect);
                    }
                }
            }
            Some(Task::MailGuy) => crate::mailguy::handle_mail_guy(&mut sp, &mut self.level, &mut self.rng, &mut events),
            Some(Task::CoWorker) => crate::coworker::handle_coworker(&mut sp, &mut self.rng),
            Some(Task::Button) => handle_button(&mut sp, &mut self.level, &mut events),
            Some(Task::SlidingDoor) => handle_sliding_door(&mut sp, &self.level, &mut events),
            Some(Task::PlayerDeath) => {
                let mut dev = Vec::new();
                died = handle_player_death(&mut sp, &mut self.g, &mut dev);
                self.world.sprites.recs[i] = std::mem::take(&mut sp);
                self.death_events(dev);
                sp = std::mem::take(&mut self.world.sprites.recs[i]);
            }
            Some(Task::Exit | Task::Stairs) | None => {}
        }
        let at = sp.at;
        self.world.sprites.recs[i] = sp;
        self.sprite_events(events, at);
        died
    }

    /// Sprite `a`'s hit task with sprite `b`.
    fn hit(&mut self, a: usize, b: usize, assets: &Assets, show: &mut ShowBalloonFn, save: &mut CopierSave) {
        let mut sp = std::mem::take(&mut self.world.sprites.recs[a]);
        let mut o = std::mem::take(&mut self.world.sprites.recs[b]);
        let mut events = Vec::new();
        let retry = self.prefs.scroll > 2;
        match sp.hit_task {
            Some(HitTask::Exit) => hit_exit(&mut sp, &o, &self.level, &mut self.counter),
            Some(HitTask::Door) => hit_door(&sp, &mut o),
            Some(HitTask::Cooler) => {
                let ms = CoolerMasks { body: &assets.player.body, cooler: assets.cooler };
                hit_cooler(&mut sp, &mut o, &ms, &mut self.g.tile.kick_contact, &mut events);
            }
            Some(HitTask::Plant) => {
                let ms = PlantHitMasks { body: &assets.player.body, plant: assets.plant, barrel: assets.barrel };
                hit_plant(&mut sp, &mut o, &ms, &self.physics, &mut self.rng, &mut self.g.tile.kick_contact, false, &mut events);
            }
            Some(HitTask::Barrel) => {
                let ms = FurnitureMasks { body: &assets.player.body, plant: assets.plant, barrel: assets.barrel };
                let caffeine = self.g.caffeine > 0;
                hit_barrel(&mut sp, &mut o, &ms, &self.physics, &mut self.rng, &mut self.g.tile.kick_contact, caffeine, false, &mut events);
            }
            Some(HitTask::Chair) => {
                let ms = FurnitureMasks { body: &assets.player.body, plant: assets.plant, barrel: assets.barrel };
                hit_chair(&mut sp, &mut o, &ms, &self.physics, &mut self.g.tile.kick_contact, false, &mut events);
            }
            Some(HitTask::Message) => {
                let (view, player) = (self.view.corner, self.g.player_pos);
                let mut calls = Vec::new();
                hit_message(&mut sp, &o, &self.level, retry, &mut self.words, &mut self.rng, &mut |s, t, m| {
                    let r = show(&Balloon { text: t, mode: m, at: s.at, view, player });
                    calls.push((r, m, s.pos));
                    r
                });
                self.balloons(calls, sp.at);
            }
            Some(HitTask::Collider) => {
                let (view, player) = (self.view.corner, self.g.player_pos);
                let mut calls = Vec::new();
                hit_collider(&mut sp, &o, &mut self.level, &mut self.words, &mut self.rng, save, &mut |s, t, m| {
                    let r = show(&Balloon { text: t, mode: m, at: s.at, view, player });
                    calls.push((r, m, s.pos));
                    r
                });
                self.balloons(calls, sp.at);
            }
            Some(HitTask::EnemyShot) => hit_enemy_shot(&mut sp, &mut o, &mut events),
            Some(HitTask::DartEnemy) => hit_dart_enemy(&mut sp, &mut o, &mut self.g, &mut self.rng, &mut events),
            Some(HitTask::Ramp) => hit_ramp(&mut sp, &o, self.physics.bounce),
            Some(HitTask::PlayerShot) => hit_player_shot(&mut sp, &mut o, &mut self.g.favor, &mut self.rng, &mut events),
            Some(HitTask::Boss) => {
                let mut boss = self.world.sprites.boss;
                crate::boss::hit_boss(&mut sp, &mut o, &mut boss, &mut self.rng, &mut self.g.tile.kick_contact, &mut events);
                self.world.sprites.boss = boss;
            }
            Some(HitTask::Scientist) => crate::scientist::hit_scientist(&mut sp, &mut o, &mut self.g.favor, &mut self.rng, &mut events),
            Some(HitTask::Blob) => crate::blob::hit_blob(&mut sp, &o, &self.physics, &mut self.g.tile.kick_contact, &mut events),
            Some(HitTask::Reversor) => crate::reversor::hit_reversor(&mut sp, &mut o, &self.physics, &mut events),
            Some(HitTask::Boulder) => crate::boulder::hit_boulder(&sp, &mut o),
            Some(HitTask::Turret) => crate::turret::hit_turret(&mut sp, &mut o, &mut events),
            Some(HitTask::Grate) => crate::grate::hit_grate(&mut sp, &o, &mut self.rng),
            Some(HitTask::Ghost) => crate::ghost::hit_ghost(&mut sp, &mut o, &mut events),
            Some(HitTask::Robot) => {
                let caffeine = self.g.caffeine > 0;
                crate::robot::hit_robot(&mut sp, &mut o, &self.physics, &mut self.rng, &mut self.g.tile.kick_contact, caffeine, &mut events);
            }
            Some(HitTask::MailGuy) => crate::mailguy::hit_mail_guy(&sp, &mut o),
            Some(HitTask::Button) => hit_button(&mut sp, &o, &mut events),
            Some(HitTask::Bonus | HitTask::Item | HitTask::Stairs | HitTask::SlidingDoor | HitTask::FirePipe) | None => {}
            Some(t) => panic!("hit task {t:?} is not in the frame yet"),
        }
        let at = sp.at;
        self.world.sprites.recs[a] = sp;
        self.world.sprites.recs[b] = o;
        self.sprite_events(events, at);
    }

    /// The game's part of `ShowBalloon` after the frontend's: a shown
    /// balloon of mode 1, 3 or 5 plays its sound at a random pitch
    /// (`SATRand(16000)`); one that did not fit, with the page scroll (3)
    /// not paging, nudges the view 16 px toward the sprite.
    fn balloons(&mut self, calls: Vec<BalloonCall>, at: (i16, i16)) {
        for (shown, mode, pos) in calls {
            if shown {
                if mode == 5 {
                    self.sounds.push(SoundEvent::Stop(snd::PHONE_RING));
                }
                if matches!(mode, 1 | 3 | 5) {
                    let pitch = 56000 + self.rng.below(16000) as u32;
                    self.play(snd::BALLOON, 0x100, Some(at), pitch, Channel::Effect);
                }
            } else if self.prefs.scroll == 3 && self.view.steps == 0 {
                let (ph, pv) = (i32::from(self.g.player_pos.0) + 32, i32::from(self.g.player_pos.1) + 32);
                let c = &mut self.view.corner;
                c.1 = if i32::from(pos.1) + 32 < ph { c.1.wrapping_sub(16) } else { c.1.wrapping_add(16) };
                c.0 = if i32::from(pos.0) + 32 < pv { c.0.wrapping_sub(16) } else { c.0.wrapping_add(16) };
            }
        }
    }

    /// `Ouch`: it halts the voices, so (unless caffeine works) it always
    /// picks one of its three with the random generator.
    fn ouch(&mut self) {
        self.sounds.push(SoundEvent::HaltVoices);
        if self.g.caffeine == 0 {
            let r = self.rng.below(100);
            let id = snd::OUCH[if r < 43 { 0 } else if r < 86 { 1 } else { 2 }];
            self.play(id, 0x100, None, RATE, Channel::Voice);
        }
    }

    /// What drawing does to the game (`DrawSpritesBehindTiles` /
    /// `DrawSpritesAboveTiles` → `WrapDrawSprite` → the face's draw proc,
    /// once for each copy in the 480-px ring buffer the screen is drawn
    /// in): every sprite with a task and a face whose `at` lies within the
    /// view − 64 … + 512 is drawn where its face reaches into the view.
    /// Some draw procs change the sprite (the draw proc goes with the
    /// face): a hit robot, big robot, turret or reversor (layer 10) goes
    /// back to 7, a blob too (+0x8a), a hit grate to 9; the vending
    /// machine sets +0x78; Harry's face sets +0x8a while his comfort falls
    /// or the shrapnel gun charges, and on caffeine draws a random number
    /// (`SATRand(10)`) - as does the boss in phase 3 (`SATRand(15)`).
    fn draw_pass(&mut self, assets: &Assets) {
        let (vv, vh) = (i32::from(self.view.corner.0), i32::from(self.view.corner.1));
        let origin = scroll_origin(self.view.corner);
        // behind the tiles (+0x8b set), then above them
        for behind in [true, false] {
            for k in 0..self.world.sprites.list.len() {
                let i = self.world.sprites.list[k];
                let sp = &self.world.sprites.recs[i];
                if sp.task.is_none() || sp.face == 0 || sp.b8b != behind {
                    continue;
                }
                let (v, h) = (i32::from(sp.at.0), i32::from(sp.at.1));
                if !(vv - 64 <= v && v < vv + 512 && vh - 64 <= h && h < vh + 512) {
                    continue;
                }
                let b = (assets.face_bounds)(sp.face);
                let unscaled = sp.scale[0] == 0x100 && sp.scale[1] == 0x100;
                let (pos, size) = if unscaled {
                    (sp.at, (b[3], b[2]))
                } else {
                    // FindScaledPosAndBounds
                    let fw = b[3].wrapping_sub(b[1]);
                    let fh = b[2].wrapping_sub(b[0]);
                    let w = ((i32::from(sp.scale[0]) * i32::from(fw)) >> 8) as i16;
                    let hh = ((i32::from(sp.scale[1]) * i32::from(fh)) >> 8) as i16;
                    let pos = (
                        sp.at.0.wrapping_add(((i32::from(fh) - i32::from(hh)) >> 1) as i16),
                        sp.at.1.wrapping_add(((i32::from(fw) - i32::from(w)) >> 1) as i16),
                    );
                    if sp.scale[0] <= 0x100 && sp.scale[1] <= 0x100 {
                        (pos, (b[3], b[2]))
                    } else {
                        (pos, (b[1].wrapping_add(w), b[0].wrapping_add(hh)))
                    }
                };
                let calls = wrap_draws(pos, size, origin, unscaled);
                for _ in 0..calls {
                    self.drawn(i);
                }
            }
        }
    }

    /// One call of a sprite's draw proc.
    fn drawn(&mut self, i: usize) {
        let g = &self.g;
        let boss = self.world.sprites.boss;
        let sp = &mut self.world.sprites.recs[i];
        match sp.face {
            // TintDrawProc (Harry)
            1101..=1244 => {
                if sp.kind == 10 {
                    sp.b8a = true;
                } else if g.caffeine < 1 {
                    let item = usize::try_from(g.cur_slot)
                        .ok()
                        .and_then(|s| g.inventory.get(s))
                        .map_or(-1, |e| e.0);
                    if g.fire_hold < 1 || item != 11999 {
                        if g.comfort < g.comfort_shown {
                            sp.b8a = true;
                        }
                    } else {
                        sp.b8a = true;
                    }
                } else {
                    self.rng.below(10);
                }
            }
            // RobotTintDrawProc, MGTintDrawProc, TurretTintDrawProc
            4201..=4236 | 4701..=4772 | 4601..=4632 => {
                if sp.layer == 10 {
                    sp.layer = 7;
                }
            }
            1401..=1424 => crate::blob::drawn(sp),
            3401..=3404 => crate::grate::drawn(sp),
            // VendingDrawProc
            1902 | 1903 => {
                let empty = usize::try_from(sp.level_index)
                    .ok()
                    .and_then(|x| self.level.sprites.get(x))
                    .is_some_and(|e| e.info[1] < 1);
                if sp.f78 == 1 && empty {
                    sp.b8a = true;
                }
                sp.f78 = 1;
            }
            // BossTintDrawProc
            4902..=4937 => {
                if sp.f6e < 0 {
                    sp.b8a = true;
                } else if (boss.anim == 2 || boss.anim == 6 || boss.phase != 2) && boss.phase == 3 {
                    self.rng.below(15);
                }
            }
            _ => {}
        }
    }

    /// `CheckPlayerCollision`: the first sprite with Harry's task against
    /// every other sprite with a task, `at` + hot rect → `HitPlayerSprite`.
    fn check_player_collision(&mut self, _assets: &Assets) {
        let Some(p) = self.player() else { return };
        let list = self.world.sprites.list.clone();
        for o in list {
            let other = self.world.rec(o);
            if o == p || other.task.is_none() || other.task == Some(Task::Player) {
                continue;
            }
            let me = self.world.rec(p);
            if !sect(absolute(me.hot_rect, me.at), absolute(other.hot_rect, other.at)) {
                continue;
            }
            let mut h = harry_from(me);
            let mut other = std::mem::take(&mut self.world.sprites.recs[o]);
            let mut w = TouchWorld { level: &mut self.level, physics: &self.physics, voice_playing: false };
            let mut events = Vec::new();
            touch(&mut h, &mut self.g, &mut other, &mut w, &mut events);
            for e in events {
                match e {
                    TouchEvent::Ouch => self.ouch(),
                    TouchEvent::Sound(id) => self.play(id, 0x100, None, RATE, Channel::Effect),
                    TouchEvent::VoiceSound(id) => self.play(id, 0x100, None, RATE, Channel::Voice),
                    TouchEvent::HaltTalking => self.sounds.push(SoundEvent::HaltVoices),
                    TouchEvent::Remark(id) => self.play(id, 0x100, None, RATE, Channel::Remark),
                }
            }
            self.world.sprites.recs[o] = other;
            harry_into(&h, &mut self.world.sprites.recs[p]);
        }
    }

    /// `CheckTileCollisions`: every sprite with a tile task, the tiles
    /// around its face's centre.
    fn check_tile_collisions(&mut self, assets: &Assets) {
        let list = self.world.sprites.list.clone();
        for i in list {
            let r = self.world.rec(i);
            let Some(task) = r.hit_tile_task else { continue };
            let mut hits = Vec::new();
            touched_tiles(&self.level, r.at, r.hot_rect, (assets.face_bounds)(r.face), |h| hits.push(h));
            for hit in hits {
                let sp = &mut self.world.sprites.recs[i];
                let walls = &assets.player.walls;
                match task {
                    HitTileTask::Player => {
                        let mut h = harry_from(sp);
                        let mut sounds = Vec::new();
                        hit_player_tile(&mut h.player, &mut self.g.tile, &self.level, assets.player, &self.physics, hit, &mut sounds);
                        harry_into(&h, sp);
                        let at = sp.at;
                        for s in sounds {
                            match s {
                                TileSound::Ouch => self.ouch(),
                                TileSound::Water => self.play(snd::WATER, 0x100, Some(at), 55000, Channel::Effect),
                            }
                        }
                    }
                    HitTileTask::Plant => {
                        let ms = ObjectMasks { plant: assets.plant, walls, obstacle: &assets.player.obstacle };
                        hit_plant_tile(sp, hit, &self.level, &ms, &self.physics);
                    }
                    HitTileTask::Chair => {
                        let ms = ObjectMasks { plant: assets.plant, walls, obstacle: &assets.player.obstacle };
                        hit_object_tile(sp, hit, &self.level, &ms, 18, &self.physics);
                    }
                    HitTileTask::Barrel => {
                        let ms = ObjectMasks { plant: assets.barrel, walls, obstacle: &assets.player.obstacle };
                        hit_object_tile(sp, hit, &self.level, &ms, 12, &self.physics);
                    }
                    HitTileTask::Robot => {
                        let (wall, obstacle) = &mut self.robot_flags;
                        crate::robot::hit_robot_tile(sp, hit, &self.level, assets.robot, walls, &assets.player.obstacle, wall, obstacle);
                    }
                    HitTileTask::Reversor => {
                        let (wall, obstacle) = &mut self.reversor_flags;
                        crate::reversor::hit_reversor_tile(sp, hit, &self.level, assets.robot, walls, &assets.player.obstacle, wall, obstacle);
                    }
                    HitTileTask::Boss => {}
                    HitTileTask::Blob => {
                        let (wall, obstacle) = &mut self.blob_flags;
                        crate::blob::hit_blob_tile(sp, hit, assets.blob, walls, wall, obstacle);
                    }
                    HitTileTask::Boulder => crate::boulder::hit_boulder_tile(sp, hit),
                    HitTileTask::MailGuy => crate::mailguy::hit_mail_guy_tile(sp, hit),
                    HitTileTask::EnemyShot => {
                        let ms = ShotMasks { shot: assets.shot, walls, obstacle: &assets.player.obstacle };
                        hit_enemy_shot_tile(sp, hit, &self.level, &ms);
                    }
                    HitTileTask::Ramp => {
                        let ms = RampMasks { ramps: assets.ramps, walls, obstacle: &assets.player.obstacle };
                        hit_ramp_tile(sp, hit, &self.level, &ms, self.physics.bounce);
                    }
                    HitTileTask::PlayerShot => {
                        let mut s = Shot::from_sprite(sp);
                        let mut wev = Vec::new();
                        hit_shot_tile(&mut s, assets.player_shot, walls, hit, &mut wev);
                        s.into_sprite(sp);
                        let at = sp.at;
                        self.weapon_events(wev, at);
                    }
                }
            }
        }
    }
}
