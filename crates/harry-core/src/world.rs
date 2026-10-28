//! The sprites of a level as the original keeps them: SAT's list of sprite
//! records with its free list, the idle sprite table, the level start
//! (`SetupLevelSprites`, `GenerateSprite`) and the bookkeeping of
//! `EraseSprites`.
//!
//! SAT reuses records: a new sprite takes the most recently freed record
//! and SAT clears only its task and hit task, so fields neither the setup
//! nor `IdleToActiveSprite` sets keep the old sprite's values. The port
//! keeps the records in an arena with the same free list to get the same
//! leftovers.

use crate::collider::setup_collider;
use crate::dart_enemy::setup_dart_enemy;
use crate::enemy_shot::setup_enemy_shot;
use crate::kinds::{setup_bonus, setup_cooler, setup_door, setup_exit, setup_item, setup_message, setup_plant};
use crate::level::Level;
use crate::props::{Props, prop_face};
use crate::ramp::setup_ramp;
use crate::random::Random;
use crate::bounce::Rect;
use crate::physics::Speed;
use crate::sprite::{Face, HitTask, HitTileTask, Sprite, Task};

/// A setup routine (the `TV_Setup…Sprite` passed to `SATNewSprite`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setup {
    Bonus,
    Item,
    Exit,
    Door,
    Cooler,
    Plant,
    Message,
    Collider,
    DartEnemy,
    Ramp,
    EnemyShot,
    PlayerShot,
    Effect,
    Barrel,
    Chair,
    Stairs,
    Button,
    SlidingDoor,
    CoWorker,
    MailGuy,
}

/// Run a setup routine on a sprite.
pub fn run_setup(setup: Setup, sp: &mut Sprite, rng: &mut Random) {
    match setup {
        Setup::Bonus => setup_bonus(sp),
        Setup::Item => setup_item(sp),
        Setup::Exit => setup_exit(sp),
        Setup::Door => setup_door(sp),
        Setup::Cooler => setup_cooler(sp),
        Setup::Plant => setup_plant(sp),
        Setup::Message => setup_message(sp),
        Setup::Collider => setup_collider(sp, rng),
        Setup::DartEnemy => setup_dart_enemy(sp),
        Setup::Ramp => setup_ramp(sp),
        Setup::EnemyShot => setup_enemy_shot(sp),
        Setup::PlayerShot => crate::shots::setup_player_shot(sp),
        Setup::Effect => crate::effect::setup_effect(sp),
        Setup::Barrel => crate::furniture::setup_barrel(sp),
        Setup::Chair => crate::furniture::setup_chair(sp, rng),
        Setup::Stairs => crate::stairs::setup_stairs(sp),
        Setup::Button => crate::button::setup_button(sp),
        Setup::SlidingDoor => crate::button::setup_sliding_door(sp),
        Setup::CoWorker => crate::coworker::setup_coworker(sp),
        Setup::MailGuy => crate::mailguy::setup_mail_guy(sp),
    }
}

/// SAT's sprite records: the list (front first) and the free list.
#[derive(Clone, Debug, Default)]
pub struct Sprites {
    /// Every record ever made; an index is a record's identity.
    pub recs: Vec<Sprite>,
    /// The list, in SAT order.
    pub list: Vec<usize>,
    /// Freed records, the most recently freed last.
    pub free: Vec<usize>,
}

impl Sprites {
    /// `SATNewSprite` up to the setup: a freed record (task and hit task
    /// cleared, everything else as it was) or a new zeroed one, with kind
    /// and position, at the front of the list.
    pub fn new_sprite(&mut self, kind: i16, h: i16, v: i16) -> usize {
        let i = match self.free.pop() {
            Some(i) => {
                let r = &mut self.recs[i];
                r.task = None;
                r.hit_task = None;
                i
            }
            None => {
                self.recs.push(Sprite::default());
                self.recs.len() - 1
            }
        };
        let r = &mut self.recs[i];
        r.kind = kind;
        r.pos = (v, h);
        self.list.insert(0, i);
        i
    }

    /// `SATNewSprite` with a setup.
    pub fn spawn(&mut self, kind: i16, h: i16, v: i16, setup: Setup, rng: &mut Random) -> usize {
        let i = self.new_sprite(kind, h, v);
        run_setup(setup, &mut self.recs[i], rng);
        i
    }

    /// `SATKillSprite` (`FUN_1005cb9c`): out of the list, onto the free list.
    pub fn kill(&mut self, i: usize) {
        if let Some(p) = self.list.iter().position(|&x| x == i) {
            self.list.remove(p);
        }
        self.free.push(i);
    }

    /// `KillAllSprites`: every sprite in the list, front first.
    pub fn kill_all(&mut self) {
        while let Some(&i) = self.list.first() {
            self.kill(i);
        }
    }
}

/// One slot of the idle table (0x44 bytes at 0x1006dcf0): what
/// `ActiveToIdleSprite` saves of a sprite while it is parked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IdleSlot {
    pub used: bool,
    /// The record while the sprite is in the list.
    pub active: Option<usize>,
    pub face: Face,
    pub b8a: bool,
    pub kind: i16,
    pub dir: i16,
    pub layer: i16,
    /// (v, h)
    pub at: (i16, i16),
    pub level_index: i16,
    pub state: i16,
    pub timer: i16,
    pub f7c: i16,
    pub hot_rect: Rect,
    pub speed: Speed,
    pub f6e: i32,
    pub b8b: bool,
    pub b8c: bool,
    pub scale: [i16; 2],
    pub air: i16,
    pub task: Option<Task>,
    pub hit_task: Option<HitTask>,
    pub hit_tile_task: Option<HitTileTask>,
}

/// Slots of the idle table.
pub const IDLE_SLOTS: usize = 255;

/// The sprites of a level: SAT's records, the idle table and the props.
#[derive(Clone, Debug)]
pub struct World {
    pub sprites: Sprites,
    pub idle: Vec<IdleSlot>,
    pub bg_props: Props,
    pub fg_props: Props,
}

impl Default for World {
    fn default() -> Self {
        World { sprites: Sprites::default(), idle: vec![IdleSlot::default(); IDLE_SLOTS], bg_props: Props::default(), fg_props: Props::default() }
    }
}

/// Round toward zero to an even pixel.
fn even(x: i16) -> i16 {
    x / 2 * 2
}

impl World {
    /// The record of a list entry.
    pub fn rec(&self, i: usize) -> &Sprite {
        &self.sprites.recs[i]
    }

    /// `ActiveToIdleSprite`: save the slot's sprite and kill its task (it
    /// stays in the list until `EraseSprites`).
    pub fn active_to_idle(&mut self, slot: usize) {
        let Some(i) = self.idle[slot].active else { return };
        let r = &mut self.sprites.recs[i];
        let s = &mut self.idle[slot];
        s.face = r.face;
        s.b8a = r.b8a;
        s.f7c = r.f7c;
        s.kind = r.kind;
        s.dir = r.dir;
        s.layer = r.layer;
        s.at = r.at;
        s.level_index = r.level_index;
        s.state = r.state;
        s.timer = r.timer;
        s.hot_rect = r.hot_rect;
        s.speed = r.speed;
        s.f6e = r.f6e;
        s.b8b = r.b8b;
        s.b8c = r.b8c;
        s.scale = [r.scale[0], r.scale[1]];
        s.air = r.air;
        s.task = r.task;
        s.hit_task = r.hit_task;
        s.hit_tile_task = r.hit_tile_task;
        r.task = None;
        r.frozen = false;
        s.active = None;
    }

    /// `IdleToActiveSprite`: a new record (no setup) with the saved fields.
    pub fn idle_to_active(&mut self, slot: usize) {
        let (kind, at) = (self.idle[slot].kind, self.idle[slot].at);
        let i = self.sprites.new_sprite(kind, at.1, at.0);
        self.idle[slot].active = Some(i);
        let s = &self.idle[slot];
        let r = &mut self.sprites.recs[i];
        r.f86 = 0;
        r.b8a = true;
        r.face = s.face;
        r.f7c = s.f7c;
        r.f82 = -1;
        r.f84 = -1;
        r.kind = s.kind;
        r.dir = s.dir;
        r.layer = s.layer;
        r.at = s.at;
        r.level_index = s.level_index;
        r.state = s.state;
        r.timer = s.timer;
        r.hot_rect = s.hot_rect;
        r.speed = s.speed;
        r.f6e = s.f6e;
        r.b8b = s.b8b;
        r.b8c = s.b8c;
        r.scale[0] = s.scale[0];
        r.scale[1] = s.scale[1];
        r.air = s.air;
        r.task = s.task;
        r.hit_task = s.hit_task;
        r.hit_tile_task = s.hit_tile_task;
        r.fix_from_at();
        r.frozen = false;
        r.idle_index = slot as i16;
    }

    /// `AddIdleSprite`: make the sprite with its setup, park it in the
    /// first free slot (position and level index as given, `face` if not
    /// 0) and free its record. A full table drops it (the original shows
    /// an error dialog).
    #[allow(clippy::too_many_arguments)]
    pub fn add_idle(&mut self, kind: i16, h: i16, v: i16, index: i16, face: Face, setup: Setup, rng: &mut Random) {
        let Some(slot) = self.idle.iter().position(|s| !s.used) else { return };
        let i = self.sprites.spawn(kind, h, v, setup, rng);
        self.idle[slot].used = true;
        self.idle[slot].active = Some(i);
        self.active_to_idle(slot);
        let s = &mut self.idle[slot];
        s.kind = kind;
        s.at = (v, h);
        s.level_index = index;
        s.face = if face != 0 { face } else { self.sprites.recs[i].face };
        self.sprites.kill(i);
    }

    /// `GenerateSprite` for one entry of the level's sprite list (`h`, `v`
    /// = its position − 6). Returns false for the object ids whose kinds
    /// the port does not have yet (desks, barrels, chairs, sliding doors,
    /// scientists, co-workers, mail guys, blobs, robots, reversors,
    /// turrets, the boss, hazards, grates, fire pipes, ghosts, boulders,
    /// stairs, buttons); ids that make nothing return true.
    pub fn generate_sprite(&mut self, h: i16, v: i16, kind: i16, index: i16, rng: &mut Random) -> bool {
        let idle = |w: &mut World, h: i16, v: i16, setup: Setup, rng: &mut Random| w.add_idle(kind, h, v, index, 0, setup, rng);
        match kind {
            301..=319 | 1100..=1299 | 1700..=1849 | 1850..=1899 | 1900..=1999
            | 2000..=2099 | 2200..=2299 | 3001 | 3900..=3999 | 4200..=4299 | 5000..=5999 | 6000..=6099 => return false,
            500 | 501 => idle(self, even(h), even(v), Setup::Cooler, rng),
            550 => idle(self, even(h), even(v), Setup::Barrel, rng),
            560 => idle(self, even(h), even(v), Setup::Chair, rng),
            601 => idle(self, even(h), even(v), Setup::Plant, rng),
            700..=799 => idle(self, h, v, Setup::Bonus, rng),
            900..=949 => idle(self, even(h), even(v), Setup::Door, rng),
            1000..=1049 => match prop_face(kind) {
                Some((_, face)) => self.bg_props.add((v, h), face),
                None => idle(self, h, v, Setup::DartEnemy, rng),
            },
            1300..=1449 => idle(self, even(h) + 1, even(v), Setup::CoWorker, rng),
            1601 | 1611 | 1621 | 1631 => {
                let i = self.sprites.spawn(kind, h, v, Setup::MailGuy, rng);
                self.sprites.recs[i].level_index = index;
            }
            1600..=1699 => {
                if let Some((true, face)) = prop_face(kind) {
                    self.fg_props.add((v, h), face);
                }
            }
            2301..=2304 => {
                let i = self.sprites.spawn(kind, h, v, Setup::Collider, rng);
                self.sprites.recs[i].level_index = index;
            }
            3100 | 3101 | 9199 | 9200 | 9300 | 9600 | 9800 | 9401..=9499 => idle(self, h, v, Setup::Collider, rng),
            9700..=9799 => idle(self, h, v, Setup::Ramp, rng),
            950..=999 => idle(self, even(h), even(v) + i16::from(kind == 951), Setup::SlidingDoor, rng),
            9500..=9599 => idle(self, h, v, Setup::Button, rng),
            9900..=9909 => idle(self, h, v, Setup::Stairs, rng),
            9990..=9999 => idle(self, h, v, Setup::Exit, rng),
            11001..=19999 => idle(self, h, v, Setup::Item, rng),
            21000.. => idle(self, h, v, Setup::Message, rng),
            _ => {}
        }
        true
    }

    /// The sprite part of `SetupLevelSprites`: all sprites killed, the
    /// idle table and the prop tables emptied, then `GenerateSprite` for
    /// each used entry with an id. Returns the ids the port could not make.
    pub fn setup_level_sprites(&mut self, level: &Level, rng: &mut Random) -> Vec<i16> {
        self.sprites.kill_all();
        self.idle = vec![IdleSlot::default(); IDLE_SLOTS];
        self.bg_props = Props::default();
        self.fg_props = Props::default();
        let mut missing = Vec::new();
        for (i, e) in level.sprites.iter().enumerate() {
            if e.present && e.id != 0 && !self.generate_sprite(e.h.wrapping_sub(6), e.v.wrapping_sub(6), e.id, i as i16, rng) {
                missing.push(e.id);
            }
        }
        missing
    }

    /// `HandleIdleSprites`: with the view's top-left (v, h), each used
    /// slot in order - a parked sprite comes back when its `at` is within
    /// 348 px of the view's corner − 200 on both axes, an active one is
    /// parked when it is farther on either axis and its hot rect does not
    /// touch the 448 × 448 view.
    pub fn handle_idle_sprites(&mut self, view: (i16, i16)) {
        let r = [view.0, view.1, view.0.wrapping_add(448), view.1.wrapping_add(448)];
        for slot in 0..IDLE_SLOTS {
            if !self.idle[slot].used {
                continue;
            }
            let at = self.idle[slot].at;
            let dh = (i32::from(view.1) - i32::from(at.1) + 200).abs();
            let dv = (i32::from(view.0) - i32::from(at.0) + 200).abs();
            match self.idle[slot].active {
                None => {
                    if dh < 348 && dv < 348 {
                        self.idle_to_active(slot);
                    }
                }
                Some(i) => {
                    if dh > 348 || dv > 348 {
                        let s = &self.sprites.recs[i];
                        let me = crate::sat::absolute(s.hot_rect, s.at);
                        if !crate::tiles::sect(r, me) {
                            self.active_to_idle(slot);
                        }
                    }
                }
            }
        }
    }

    /// The bookkeeping of `EraseSprites` (each drawn frame): sprites
    /// without a task or frozen leave the list (a frozen level sprite is
    /// also marked gone in the level and its idle slot freed); the others
    /// remember this frame's `at`, face and scale (+0x82…+0x86,
    /// scale2/3), clear +0x8a, count down +0x94 / +0x96 while not 0, and
    /// write their kind and position (+ 6) into their level entry.
    pub fn erase_sprites(&mut self, level: &mut Level) {
        let list = self.sprites.list.clone();
        for i in list {
            let r = &mut self.sprites.recs[i];
            let li = usize::try_from(r.level_index).ok().filter(|&x| x < 256);
            if r.task.is_none() || r.frozen {
                if let Some(li) = li.filter(|_| r.frozen) {
                    level.sprites[li].present = false;
                    if let Some(s) = usize::try_from(r.idle_index).ok().filter(|&x| x < 256).and_then(|x| self.idle.get_mut(x)) {
                        s.used = false;
                    }
                }
                self.sprites.kill(i);
            } else {
                r.b8a = false;
                r.f86 = i32::from(r.face);
                r.f84 = r.at.1;
                r.f82 = r.at.0;
                r.scale[2] = r.scale[0];
                r.scale[3] = r.scale[1];
                if r.f94 != 0 {
                    r.f94 = r.f94.wrapping_sub(1);
                }
                if r.blast_cooldown != 0 {
                    r.blast_cooldown = r.blast_cooldown.wrapping_sub(1);
                }
                if let Some(li) = li {
                    let e = &mut level.sprites[li];
                    e.id = r.kind;
                    e.h = r.at.1.wrapping_add(6);
                    e.v = r.at.0.wrapping_add(6);
                }
            }
        }
    }
}
