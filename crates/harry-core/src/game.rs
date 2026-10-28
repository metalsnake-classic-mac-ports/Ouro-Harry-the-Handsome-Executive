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
use crate::enemy_shot::{ShotMasks, handle_enemy_shot, hit_enemy_shot, hit_enemy_shot_tile};
use crate::frame::{FrameSound, Harry, PlayerGlobals, handle_player};
use crate::keys::{Bindings, KeyEvent, KeyInput, KeyMap, handle_keys};
use crate::kinds::{
    CoolerMasks, LevelCounter, ObjectMasks, PlantHitMasks, ShowBalloon, handle_bonus, handle_cooler, handle_door, handle_item,
    handle_message, handle_plant, hit_cooler, hit_door, hit_exit, hit_message, hit_plant, hit_plant_tile,
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
use crate::words::Words;
use crate::world::{Setup, World};

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
    /// `_DAT_1006c4ac`, the magnets' delay.
    pub magnet: i16,
    /// The drawn gun (`PTR_DAT_100660dc`).
    pub gun: u8,
    /// Harry's record (`*_DAT_10065980`), set by `start`.
    pub harry: Option<usize>,
    /// The sentence generators' word lists and last picks.
    pub words: Words,
}

/// What a `ShowBalloon` call ended with: (shown, mode, the sprite's `pos`).
type BalloonCall = (bool, i16, (i16, i16));

/// What a frame ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameEnd {
    Running,
    /// The level counter changed (`UpLevel`).
    LevelDone,
    /// Harry died (his task became `HandlePlayerSpriteDeath`; not ported).
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
        let i = self.world.sprites.spawn(kind, h, v, setup, &mut self.rng);
        let r = &mut self.world.sprites.recs[i];
        r.hot_rect2 = absolute(r.hot_rect, r.pos);
        if let Some(s) = speed {
            r.speed = s;
        }
        i
    }

    /// `EasyExplosion(h, v, 0, 0, kind, dir)`.
    fn explosion(&mut self, h: i16, v: i16, kind: i16, dir: i16) {
        let i = self.spawn_with(kind, h, v, Setup::Effect, Some(Default::default()));
        let r = &mut self.world.sprites.recs[i];
        r.dir = dir;
        r.kind = kind;
    }

    fn sprite_events(&mut self, events: Vec<SpriteEvent>) {
        for e in events {
            match e {
                SpriteEvent::Spawn { kind, h, v, speed } => self.spawn(kind, h, v, speed),
                SpriteEvent::Explosion { h, v, a, b } => self.explosion(h, v, a, b),
                _ => {}
            }
        }
    }

    fn weapon_events(&mut self, events: Vec<WeaponEvent>) {
        for e in events {
            if let WeaponEvent::Explosion { h, v, a, b } = e {
                self.explosion(h, v, a, b);
            }
        }
    }

    /// One frame. `keys` are the keys held this frame.
    pub fn frame(&mut self, keys: &KeyMap, assets: &Assets, show: &mut ShowBalloon, save: &mut CopierSave) -> FrameEnd {
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

    /// Run sprite `i`'s task. Returns whether Harry died.
    fn task(&mut self, i: usize, keys: &KeyMap, assets: &Assets, show: &mut ShowBalloon) -> bool {
        let mut sp = std::mem::take(&mut self.world.sprites.recs[i]);
        let mut events = Vec::new();
        let mut died = false;
        match sp.task {
            Some(Task::Player) => {
                let mut h = harry_from(&sp);
                let input = KeyInput { keys, bindings: &self.prefs.bindings, debug: false, gulp_playing: false, voice_playing: false };
                let mut key_events = Vec::new();
                let mut sounds: Vec<FrameSound> = Vec::new();
                handle_player(
                    &mut h,
                    &mut self.g,
                    &self.level,
                    &self.physics,
                    &mut self.rng,
                    |h, g, rng| handle_keys(h, g, &input, rng, &mut key_events),
                    &mut sounds,
                );
                died = h.dead;
                harry_into(&h, &mut sp);
                if died {
                    sp.task = Some(Task::PlayerDeath);
                    sp.timer = 0;
                }
                self.world.sprites.recs[i] = std::mem::take(&mut sp);
                for e in key_events {
                    match e {
                        KeyEvent::Shot { h, v, dir, speed } => {
                            let s = self.spawn_with(10, h, v, Setup::PlayerShot, Some(speed));
                            self.world.sprites.recs[s].dir = dir;
                        }
                        KeyEvent::Explosion { h, v, flag } => self.explosion(h, v, flag, 0),
                        KeyEvent::Ripple(_) => panic!("the ripple is not ported into the frame yet"),
                        _ => {}
                    }
                }
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
                self.world.sprites.recs[i] = std::mem::take(&mut sp);
                for s in shots {
                    self.spawn_with(s.kind, s.h, s.v, Setup::PlayerShot, Some(s.speed));
                }
                self.weapon_events(wev);
                sp = std::mem::take(&mut self.world.sprites.recs[i]);
            }
            Some(Task::Bonus) => handle_bonus(&mut sp),
            Some(Task::Item) => handle_item(&mut sp),
            Some(Task::Door) => handle_door(&mut sp, &mut self.level),
            Some(Task::Cooler) => handle_cooler(&mut sp, &mut self.level, &mut events),
            Some(Task::Plant) => {
                let mut s = Vec::new();
                handle_plant(&mut sp, &self.level, &self.physics, &mut self.rng, &mut self.plant_flag, &mut s);
            }
            Some(Task::Message) => {
                let mut calls = Vec::new();
                handle_message(&mut sp, &self.level, &mut self.words, &mut self.rng, &mut |s, t, m| {
                    let r = show(s, t, m);
                    calls.push((r, m, s.pos));
                    r
                });
                self.balloons(calls);
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
            Some(Task::Exit | Task::PlayerDeath) | None => {}
        }
        self.world.sprites.recs[i] = sp;
        self.sprite_events(events);
        died
    }

    /// Sprite `a`'s hit task with sprite `b`.
    fn hit(&mut self, a: usize, b: usize, assets: &Assets, show: &mut ShowBalloon, save: &mut CopierSave) {
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
            Some(HitTask::Message) => {
                let mut calls = Vec::new();
                hit_message(&mut sp, &o, &self.level, retry, &mut self.words, &mut self.rng, &mut |s, t, m| {
                    let r = show(s, t, m);
                    calls.push((r, m, s.pos));
                    r
                });
                self.balloons(calls);
            }
            Some(HitTask::Collider) => {
                let mut calls = Vec::new();
                hit_collider(&mut sp, &o, &mut self.level, &mut self.words, &mut self.rng, save, &mut |s, t, m| {
                    let r = show(s, t, m);
                    calls.push((r, m, s.pos));
                    r
                });
                self.balloons(calls);
            }
            Some(HitTask::EnemyShot) => hit_enemy_shot(&mut sp, &mut o, &mut events),
            Some(HitTask::DartEnemy) => hit_dart_enemy(&mut sp, &mut o, &mut self.g, &mut self.rng, &mut events),
            Some(HitTask::Ramp) => hit_ramp(&mut sp, &o, self.physics.bounce),
            Some(HitTask::PlayerShot) => hit_player_shot(&mut sp, &mut o, &mut self.g.favor, &mut self.rng, &mut events),
            Some(HitTask::Bonus | HitTask::Item) | None => {}
            Some(t) => panic!("hit task {t:?} is not in the frame yet"),
        }
        self.world.sprites.recs[a] = sp;
        self.world.sprites.recs[b] = o;
        self.sprite_events(events);
    }

    /// The game's part of `ShowBalloon` after the frontend's: a shown
    /// balloon of mode 1, 3 or 5 plays its sound at a random pitch
    /// (`SATRand(16000)`); one that did not fit, with the page scroll (3)
    /// not paging, nudges the view 16 px toward the sprite.
    fn balloons(&mut self, calls: Vec<BalloonCall>) {
        for (shown, mode, pos) in calls {
            if shown {
                if matches!(mode, 1 | 3 | 5) {
                    self.rng.below(16000);
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
        if self.g.caffeine == 0 {
            self.rng.below(100);
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
                if e == TouchEvent::Ouch {
                    self.ouch();
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
                        for s in sounds {
                            if s == TileSound::Ouch {
                                self.ouch();
                            }
                        }
                    }
                    HitTileTask::Plant => {
                        let ms = ObjectMasks { plant: assets.plant, walls, obstacle: &assets.player.obstacle };
                        hit_plant_tile(sp, hit, &self.level, &ms, &self.physics);
                    }
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
                        self.weapon_events(wev);
                    }
                }
            }
        }
    }
}
