//! Harry's frame (`HandlePlayerSprite`, PPC 0x10026e1c): everything his
//! sprite does each frame except reading the keys (`HandleKeys`, passed in)
//! and the tile collisions (`CheckTileCollisions`, a separate pass of the
//! main loop).

use crate::air::{Air, AirSound, JumpEffects, UNSET_SCALE, half_pipe, jump_scale};
use crate::bounce::Body;
use crate::keys::Ammo;
use crate::inventory::{EMPTY, current_item};
use crate::level::Level;
use crate::moves::{Chair, MoveInput, MoveSound, Moves, move_chair};
use crate::physics::{Floor, Physics, Speed, enforce_max_speed, player_glide};
use crate::player::{HOT_RECT, Player, TileState};
use crate::random::Random;

/// Inventory slots.
pub const SLOTS: usize = 12;

/// The game's globals around the player (NOTES.md, re/ghidra/globals.txt).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerGlobals {
    pub moves: Moves,
    pub tile: TileState,
    pub inv_switch_delay: i16,
    pub fire_delay: i16,
    /// `_DAT_10065e40`, counted down: frames until the mail cart can run
    /// Harry over again.
    pub t40: i16,
    pub item_use_delay: i16,
    pub recoil: i16,
    pub caffeine: i16,
    /// Windows of the key sequences P-A-R-G-H-Z, B-E-N, W-A-V-Y-G-R-A-V-Y.
    pub cheat1: i16,
    pub cheat2: i16,
    pub wavy: i16,
    /// `_DAT_10065e28` and `_DAT_10065e24`, counted down; meaning unknown.
    pub t28: i16,
    pub t24: i16,
    /// Frames until Harry may say something (`_DAT_10065ac8`).
    pub chatter: i16,
    /// Frames of shaking (random ±3 px on the drawn position).
    pub shake: i16,
    /// The glide counter (`PTR_DAT_10065e60`).
    pub glide: i16,
    /// `PTR_DAT_10065e5c` / `PTR_DAT_10065e50`, cleared each frame.
    pub flag5c: i16,
    pub flag50: bool,
    /// When set, the next frame clears the kick contact (`_DAT_10065e20`).
    pub kick_reset: bool,
    pub comfort: i16,
    pub comfort_shown: i16,
    /// Corporate favor (`gCorporateFavor`, 0–144; below 1 Harry is fired).
    pub favor: i16,
    pub favor_shown: i16,
    pub surface: i16,
    /// The surface resistance meter as drawn (`gSurfaceShown`).
    pub surface_shown: i16,
    /// The selected inventory slot, −1 none.
    pub cur_slot: i16,
    /// {object id, count}; id −1 = empty.
    pub inventory: [(i16, i16); SLOTS],
    /// Copies other sprites read: `gPlayerDir`, the position (h, v) and
    /// the speed (v, h).
    pub player_dir: i16,
    pub player_pos: (i16, i16),
    pub player_speed: (i16, i16),
    /// Ammunition (`gStapleAmmo`, `gBadassAmmo`, `gShrapnelAmmo`).
    pub ammo: Ammo,
    /// Frames the use key has been held with a charged weapon (`gFireHold`,
    /// the word after the inventory table).
    pub fire_hold: i16,
    /// The kind of the last player shot (`gShotKind`).
    pub shot_kind: i16,
    /// Progress in the key sequences (`PTR_DAT_10065e00`, `…dfc`,
    /// `…df8`); `cheat1`, `cheat2`, `wavy` are their windows.
    pub cheat1_step: i16,
    pub cheat2_step: i16,
    pub wavy_step: i16,
    /// The ripple is on (`gWavyOn`).
    pub wavy_on: bool,
    /// The score (`PTR_DAT_10065af8`; the panel counts up to it).
    pub score: i32,
}

impl PlayerGlobals {
    /// A new game (`InitGameGlobals`): the three meters at 72, nothing in
    /// the inventory, no ammo; the floor kind as the data section starts
    /// it (1).
    pub fn new_game() -> PlayerGlobals {
        let mut g = PlayerGlobals {
            comfort: 72,
            comfort_shown: 72,
            favor: 72,
            favor_shown: 72,
            surface: 72,
            surface_shown: 72,
            cur_slot: -1,
            inventory: [(EMPTY, 0); SLOTS],
            ..Default::default()
        };
        g.tile.floor_kind = 1;
        g
    }
}

/// Harry's sprite state beyond `Player`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Harry {
    pub player: Player,
    pub air: Air,
    /// `sp->frozen`: the frame does nothing.
    pub frozen: bool,
    /// The task became `HandlePlayerSpriteDeath`.
    pub dead: bool,
    /// Frames until an explosion can hurt him again (+0x96, set to 30 by
    /// `HandleExplodeRadius`).
    pub blast_cooldown: i16,
    /// Frames until a ramp can bounce him again (+0x94, 3 after a bounce,
    /// counted down by `EraseSprites`).
    pub ramp_cooldown: i16,
}

/// Sounds of the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameSound {
    Move(MoveSound),
    Air(AirSound),
    /// Idle chatter: a playing voice stops, then one of three samples
    /// (none when the chatter still plays).
    Chatter(Option<i16>),
}

/// `HandlePlayerSprite`; returns what `JumpScale` asks of the view and
/// the screen (falling into a hole). `keys` stands for `HandleKeys` (called after the
/// position sanity check, before anything else). `chatter_playing`: does
/// the last chatter still play once a playing voice is stopped (then no
/// sample is drawn)?
#[allow(clippy::too_many_arguments)]
pub fn handle_player(
    h: &mut Harry,
    g: &mut PlayerGlobals,
    level: &Level,
    physics: &Physics,
    rng: &mut Random,
    keys: impl FnOnce(&mut Harry, &mut PlayerGlobals, &mut Random),
    sounds: &mut Vec<FrameSound>,
    chatter_playing: bool,
) -> JumpEffects {
    if h.frozen {
        return JumpEffects::default();
    }
    let b = &mut h.player.body;
    if b.fix.0 < 1 || b.fix.1 < 1 || b.fix.0 > 0x100000 || b.fix.1 > 0x100000 {
        b.fix = (i32::from(b.pos.1) << 8, i32::from(b.pos.0) << 8);
    }
    keys(h, g, rng);
    g.flag5c = 0;
    g.flag50 = false;
    g.tile.bg_contact = false;
    g.tile.wall_contact = false;
    if g.tile.comfort_delta != 0 {
        if g.caffeine == 0 {
            g.comfort = g.comfort.wrapping_add(g.tile.comfort_delta);
        }
        g.tile.comfort_delta = 0;
    }
    for t in [
        &mut g.moves.scoot,
        &mut g.moves.kick,
        &mut g.moves.inch,
        &mut g.inv_switch_delay,
        &mut g.fire_delay,
        &mut g.tile.ouch_cooldown,
        &mut g.t40,
        &mut g.item_use_delay,
        &mut g.recoil,
        &mut g.caffeine,
        &mut g.cheat1,
        &mut g.cheat2,
        &mut g.wavy,
        &mut g.t28,
        &mut g.t24,
    ] {
        if *t > 0 {
            *t -= 1;
        }
    }
    if g.chatter > 0 {
        g.chatter -= 1;
        if g.chatter == 0 && rng.below(100) < 30 {
            sounds.push(FrameSound::Chatter((!chatter_playing).then(|| rng.below(3))));
        }
    }
    // The selected item gone? (slot −1 reads the chatter timer, never −1
    // here; BUGS.md)
    if current_item(g) == EMPTY {
        g.cur_slot = if g.inventory[0].0 == -1 { -1 } else { 0 };
    }
    let p = &mut h.player;
    p.hot_rect = HOT_RECT;
    if g.kick_reset {
        g.kick_reset = false;
        g.tile.kick_contact = 0;
    }
    if p.air > 0 {
        g.moves.inch = 0;
        g.moves.kick = 0;
        g.moves.scoot = 0;
        g.moves.pose = 0;
        g.moves.impulse = 0;
    }
    let mut chair = Chair { speed: p.body.speed, dir: p.dir, air: p.air, hot_rect: p.hot_rect };
    let input = MoveInput { caffeine: g.caffeine, wet: g.tile.wet, floor_kind: g.tile.floor_kind, kick_contact: g.tile.kick_contact };
    let mut move_sounds = Vec::new();
    move_chair(&mut chair, &mut g.moves, physics, &input, &mut move_sounds);
    sounds.extend(move_sounds.into_iter().map(FrameSound::Move));
    p.body.speed = chair.speed;
    p.hot_rect = chair.hot_rect;
    let floor = Floor { air: p.air, kind: g.tile.floor_kind, wet: g.tile.wet, glide: g.glide };
    if let Some(s) = player_glide(&mut p.body.speed, floor, physics) {
        g.surface = s;
    }
    let mut air_sounds = Vec::new();
    h.air.air = p.air;
    half_pipe(&mut p.body, &mut h.air, 32, true, level, &mut air_sounds);
    g.tile.kick_contact = 0;
    if p.dir >= 36 {
        p.dir = 0;
    } else if p.dir < 0 {
        p.dir = 35;
    }
    p.face = p.dir + g.moves.pose;
    let effects = jump_scale(&mut p.body, &mut h.air, &mut p.dir, 32, true, level, rng, &mut g.comfort, UNSET_SCALE, &mut air_sounds);
    p.air = h.air.air;
    sounds.extend(air_sounds.into_iter().map(FrameSound::Air));
    if g.tile.wet > 0 && (i32::from(p.body.speed.h).abs() >= 201 || i32::from(p.body.speed.v).abs() >= 201) {
        g.tile.wet = (g.tile.wet - 2).max(0);
    }
    enforce_max_speed(&mut p.body.speed, 2500);
    g.tile.floor_kind = 1;
    let Speed { h: sh, v: sv } = p.body.speed;
    g.player_speed = (sv, sh);
    move_body(&mut p.body);
    g.player_pos = (p.body.pos.1, p.body.pos.0);
    if g.shake > 0 {
        g.shake -= 1;
        let nh = rng.below(6).wrapping_add((p.body.fix.0 >> 8) as i16).wrapping_sub(3);
        p.body.pos.1 = nh;
        p.body.at.1 = nh;
        let nv = rng.below(6).wrapping_add((p.body.fix.1 >> 8) as i16).wrapping_sub(3);
        p.body.pos.0 = nv;
        p.body.at.0 = nv;
        g.player_pos = (nh, nv);
    }
    g.player_dir = p.dir;
    if (g.comfort_shown < 1 || g.favor_shown < 1) && p.air == 0 {
        h.dead = true;
    }
    effects
}

/// Integrate the speed: `fix += speed`, `pos = at = fix >> 8`.
fn move_body(b: &mut Body) {
    b.fix.0 = b.fix.0.wrapping_add(i32::from(b.speed.h));
    b.fix.1 = b.fix.1.wrapping_add(i32::from(b.speed.v));
    let h = (b.fix.0 >> 8) as i16;
    let v = (b.fix.1 >> 8) as i16;
    b.pos = (v, h);
    b.at = (v, h);
}
