//! Harry's keys (`HandleKeys`, PPC 0x1002804c): turning, the move keys,
//! selecting and using items, the weapons, the cheat key sequences and
//! the debug keys. Called by `HandlePlayerSprite` once per frame.

use crate::frame::{Harry, PlayerGlobals};
use crate::inventory::{current_item, get_item, item, remove_item, switch_item};
use crate::physics::{Speed, apply_moded_impulse, inverse_mode};
use crate::random::Random;

/// The keyboard as `GetKeys` returns it: key code c is bit c & 7 of byte
/// c >> 3.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyMap(pub [u8; 16]);

impl KeyMap {
    pub fn down(&self, code: i16) -> bool {
        self.0[(code >> 3) as usize] >> (code & 7) & 1 != 0
    }

    pub fn press(&mut self, code: i16) {
        self.0[(code >> 3) as usize] |= 1 << (code & 7);
    }
}

/// Mac key codes the game uses.
pub mod code {
    pub const A: i16 = 0x00;
    pub const H: i16 = 0x04;
    pub const G: i16 = 0x05;
    pub const Z: i16 = 0x06;
    pub const V: i16 = 0x09;
    pub const B: i16 = 0x0b;
    pub const W: i16 = 0x0d;
    pub const E: i16 = 0x0e;
    pub const R: i16 = 0x0f;
    pub const Y: i16 = 0x10;
    pub const KEY_1: i16 = 0x12;
    pub const KEY_2: i16 = 0x13;
    pub const KEY_3: i16 = 0x14;
    pub const KEY_4: i16 = 0x15;
    pub const EQUALS: i16 = 0x18;
    pub const P: i16 = 0x23;
    pub const N: i16 = 0x2d;
    pub const SPACE: i16 = 0x31;
    pub const SHIFT: i16 = 0x38;
    pub const OPTION: i16 = 0x3a;
    pub const CONTROL: i16 = 0x3b;
    pub const F3: i16 = 0x63;
    pub const F13: i16 = 0x69;
    pub const F12: i16 = 0x6f;
    pub const PAGE_UP: i16 = 0x74;
    pub const F4: i16 = 0x76;
    pub const F2: i16 = 0x78;
    pub const F1: i16 = 0x7a;
    pub const LEFT: i16 = 0x7b;
    pub const RIGHT: i16 = 0x7c;
    pub const DOWN: i16 = 0x7d;
    pub const UP: i16 = 0x7e;
}

/// The key codes of the prefs (+0x10…+0x1c), defaults from the Key
/// Controls panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bindings {
    /// Rotate left: `dir` + 1.
    pub left: i16,
    /// Rotate right: `dir` − 1.
    pub right: i16,
    pub scoot: i16,
    pub inch: i16,
    pub kick: i16,
    pub select_item: i16,
    pub use_item: i16,
}

impl Default for Bindings {
    fn default() -> Bindings {
        Bindings {
            left: code::LEFT,
            right: code::RIGHT,
            scoot: code::DOWN,
            inch: code::UP,
            kick: code::SPACE,
            select_item: code::CONTROL,
            use_item: code::OPTION,
        }
    }
}

/// `snd` ids of the sounds `HandleKeys` plays.
pub mod snd {
    pub const VOICE: i16 = 210;
    pub const UNCOMFORTABLE: i16 = 224;
    pub const STAPLE: i16 = 230;
    pub const BADASS_CHARGE: i16 = 231;
    pub const BADASS_WEAK: i16 = 232;
    pub const BADASS_STRONG: i16 = 233;
    pub const SWIVEL_CHARGE: i16 = 237;
    pub const SWIVEL_SHOT: i16 = 238;
    pub const EMPTY: i16 = 239;
    pub const GULP: i16 = 241;
    pub const SHRAPNEL: i16 = 252;
    pub const SODA: i16 = 300;
    pub const EXPLOSION: i16 = 602;
    pub const SWITCH_ITEM: i16 = 4801;
}

/// What `HandleKeys` reads besides the keys.
#[derive(Clone, Copy, Debug)]
pub struct KeyInput<'a> {
    pub keys: &'a KeyMap,
    pub bindings: &'a Bindings,
    /// The debug flag (`*_DAT_10065b74`, always 0 in the shipped game).
    pub debug: bool,
    /// Is the gulp sound (241) playing? (blocks the size Easter egg)
    pub gulp_playing: bool,
    /// Is a voice playing? (then the mystery vial stays silent)
    pub voice_playing: bool,
}

/// What `HandleKeys` asks of the rest of the game, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEvent {
    /// `STPlayRegSound`.
    Sound(i16),
    /// A sound that becomes the charge sound (`_DAT_10068a9c`).
    ChargeSound(i16),
    /// Stop the charge sound.
    StopCharge,
    /// A sound that becomes the voice (`_DAT_10068aa0`).
    VoiceSound(i16),
    /// `STPlay3DSoundPitched` at Harry's `at` (v, h) as `HandleKeys` finds it.
    Sound3D { snd: i16, at: (i16, i16), pitch: u32 },
    /// `EasyExplosion` at (h, v); `flag` is its fifth parameter.
    Explosion { h: i16, v: i16, flag: i16 },
    /// `ShowBalloon`: one of the 52 random messages, or −1 for the
    /// B-E-N one.
    Balloon(i16),
    /// A player shot sprite (kind 10) at (h, v) moving in `dir`.
    Shot { h: i16, v: i16, dir: i16, speed: Speed },
    /// The ripple was toggled: the level's ripple fields (0x12088,
    /// 0x1208a, 0x1208c) become 100, 16, 0x300 (on, then
    /// `BuildSineTable`) or 0, 0, 0; the ripple phase (`*_DAT_1006592c`)
    /// becomes 0.
    Ripple(bool),
}

/// The keys 1–4 select these items.
const NUMBER_KEYS: [(i16, i16); 4] = [
    (code::KEY_1, item::STAPLE_GUN),
    (code::KEY_2, item::BADASS),
    (code::KEY_3, item::SHRAPNEL_GUN),
    (code::KEY_4, item::POWER_OF_THE_SWIVEL),
];

/// The P-A-R-G-H-Z and W-A-V-Y-G-R-A-V-Y steps after the first key.
const PARGHZ: [i16; 5] = [code::A, code::R, code::G, code::H, code::Z];
const BEN: [i16; 2] = [code::E, code::N];
const WAVYGRAVY: [i16; 8] = [code::A, code::V, code::Y, code::G, code::R, code::A, code::V, code::Y];

/// `HandleKeys`.
pub fn handle_keys(h: &mut Harry, g: &mut PlayerGlobals, input: &KeyInput, rng: &mut Random, events: &mut Vec<KeyEvent>) {
    let k = input.keys;
    let b = input.bindings;
    if input.debug {
        debug_keys(h, g, k, rng, events);
    }
    // the size Easter egg
    if k.down(code::SHIFT) && k.down(code::PAGE_UP) && k.down(code::EQUALS) && !input.gulp_playing {
        if h.air.scale == 0x100 {
            events.push(KeyEvent::Sound3D { snd: snd::GULP, at: h.player.body.at, pitch: 50000 });
            h.air.scale = 0x200;
        } else {
            h.air.scale = 0x100;
        }
    }
    let p = &mut h.player;
    if k.down(b.left) {
        p.dir = p.dir.wrapping_add(1);
    }
    if k.down(b.right) {
        p.dir = p.dir.wrapping_sub(1);
    }
    if k.down(b.select_item) {
        if g.inv_switch_delay == 0 {
            select(g, -1, events);
        }
    } else if let Some(&(_, id)) = NUMBER_KEYS.iter().find(|(c, _)| k.down(*c))
        && let Some(s) = g.inventory.iter().rposition(|e| e.0 == id)
        && s as i16 != g.cur_slot
    {
        select(g, s as i16, events);
    }
    use_item(h, g, input, rng, events);
    sequences(g, k, rng, events);
    let free = |g: &PlayerGlobals| g.moves.scoot == 0 && g.moves.kick == 0 && g.moves.inch == 0;
    if k.down(b.kick) && free(g) {
        g.moves.kick = 21;
    }
    if h.player.air < 1 {
        if k.down(b.scoot) && free(g) {
            g.moves.scoot = 21;
        }
        if k.down(b.inch) && free(g) {
            g.moves.inch = 17;
        }
    }
}

fn select(g: &mut PlayerGlobals, slot: i16, events: &mut Vec<KeyEvent>) {
    events.push(KeyEvent::Sound(snd::SWITCH_ITEM));
    switch_item(g, slot);
    g.inv_switch_delay = 5;
    g.fire_hold = 0;
}

/// The debug keys: F1 comfort 144, F2 staple gun with 100 staples, F12
/// comfort 0, `=` an explosion near Harry, F13 / F4 / F3 items.
fn debug_keys(h: &mut Harry, g: &mut PlayerGlobals, k: &KeyMap, rng: &mut Random, events: &mut Vec<KeyEvent>) {
    if k.down(code::F1) {
        g.comfort = 144;
    }
    if k.down(code::F2) {
        get_item(g, item::STAPLE_GUN);
        g.ammo.staple = 100;
    }
    if k.down(code::F12) {
        g.comfort = 0;
    }
    if k.down(code::EQUALS) && h.blast_cooldown == 0 {
        events.push(KeyEvent::Sound(snd::EXPLOSION));
        let dv = rng.below(128);
        let dh = rng.below(128);
        let (v, hh) = h.player.body.at;
        events.push(KeyEvent::Explosion { h: hh.wrapping_add(dh).wrapping_sub(32), v: v.wrapping_add(dv).wrapping_sub(32), flag: 0 });
    }
    if k.down(code::F13) {
        get_item(g, item::SODA_CAN);
    }
    if k.down(code::F4) {
        get_item(g, item::DEBUG_ITEM);
    }
    if k.down(code::F3) {
        get_item(g, item::CAFFEINE_PILL);
    }
}

/// The use key: firing on release (charged weapons) or on press.
fn use_item(h: &mut Harry, g: &mut PlayerGlobals, input: &KeyInput, rng: &mut Random, events: &mut Vec<KeyEvent>) {
    let id = current_item(g);
    if !input.keys.down(input.bindings.use_item) {
        if g.fire_hold > 0 {
            release(h, g, id, rng, events);
        }
        return;
    }
    let ready = g.fire_delay == 0;
    match id {
        item::STAPLE_GUN if ready => {
            if g.ammo.staple < 1 {
                events.push(KeyEvent::Sound(snd::EMPTY));
            } else {
                g.ammo.staple -= 1;
                let pitch = rng.below(6000) as u32 + 62000;
                events.push(KeyEvent::Sound3D { snd: snd::STAPLE, at: h.player.body.at, pitch });
                g.recoil = 3;
                g.shot_kind = 0;
            }
            g.fire_delay = 10;
        }
        item::BADASS | item::POWER_OF_THE_SWIVEL if ready => {
            if g.fire_hold == 0 {
                events.push(KeyEvent::StopCharge);
                let s = if id == item::BADASS { snd::BADASS_CHARGE } else { snd::SWIVEL_CHARGE };
                events.push(KeyEvent::ChargeSound(s));
            }
            g.fire_hold = g.fire_hold.wrapping_add(1);
        }
        item::SHRAPNEL_GUN if ready => g.fire_hold = g.fire_hold.wrapping_add(1),
        item::CAFFEINE_PILL if ready => {
            if g.item_use_delay == 0 {
                events.push(KeyEvent::Sound(snd::GULP));
                remove_item(g, g.cur_slot);
                g.caffeine = 270;
                g.fire_delay = 10;
                g.item_use_delay = 8;
            }
        }
        // the vial ignores the fire delay
        item::MYSTERY_VIAL => {
            if g.item_use_delay == 0 {
                mystery_vial(h, g, input, rng, events);
                remove_item(g, g.cur_slot);
                g.item_use_delay = 10;
            }
        }
        item::SODA_CAN if ready && g.item_use_delay == 0 => {
            events.push(KeyEvent::Sound(snd::SODA));
            remove_item(g, g.cur_slot);
            g.fire_delay = 10;
            g.recoil = 3;
            g.shot_kind = 4;
            g.item_use_delay = 6;
        }
        _ => {}
    }
}

/// The use key released while charging.
fn release(h: &mut Harry, g: &mut PlayerGlobals, id: i16, rng: &mut Random, events: &mut Vec<KeyEvent>) {
    let hold = g.fire_hold;
    match id {
        item::BADASS => {
            events.push(KeyEvent::StopCharge);
            let strong = hold >= 34;
            if g.ammo.badass < 1 {
                // without ammo the strong shot still goes off (BUGS.md)
                if strong {
                    events.push(KeyEvent::Sound(snd::BADASS_STRONG));
                    g.shot_kind = 2;
                    g.recoil = 3;
                } else {
                    events.push(KeyEvent::Sound(snd::EMPTY));
                }
            } else {
                if strong {
                    events.push(KeyEvent::Sound(snd::BADASS_STRONG));
                    g.shot_kind = 2;
                    g.ammo.badass = (g.ammo.badass - 6).max(0);
                } else {
                    events.push(KeyEvent::Sound(snd::BADASS_WEAK));
                    g.shot_kind = 1;
                    g.ammo.badass = (g.ammo.badass - 2).max(0);
                }
                g.recoil = 3;
            }
            g.fire_delay = 4;
            g.fire_hold = 0;
        }
        item::SHRAPNEL_GUN => {
            if g.ammo.shrapnel < 1 {
                events.push(KeyEvent::Sound(snd::EMPTY));
            } else {
                events.push(KeyEvent::Sound(snd::SHRAPNEL));
                let (kind, cost) = match hold {
                    ..8 => (0, 1),
                    8..16 => (5, if rng.below(100) < 50 { 1 } else { 2 }),
                    16..24 => (6, 2),
                    _ => (7, if rng.below(100) < 50 { 2 } else { 3 }),
                };
                g.shot_kind = kind;
                g.ammo.shrapnel = (g.ammo.shrapnel - cost).max(0);
                g.recoil = 3;
            }
            g.fire_delay = 4;
            g.fire_hold = 0;
        }
        // the power of the swivel (or whatever is selected now)
        _ if hold <= 20 => g.fire_hold = 0,
        _ if g.comfort < 10 => {
            events.push(KeyEvent::Sound(snd::UNCOMFORTABLE));
            g.fire_hold = 10;
        }
        _ => {
            g.comfort -= 2;
            events.push(KeyEvent::Sound(snd::SWIVEL_SHOT));
            g.fire_hold = 10;
            let p = &h.player;
            let mut speed = Speed::default();
            apply_moded_impulse(&mut speed, inverse_mode(p.dir), 2000);
            events.push(KeyEvent::Shot { h: p.body.at.1, v: p.body.at.0, dir: p.dir, speed });
        }
    }
}

/// The mystery vial: an explosion on Harry, then one of six effects.
fn mystery_vial(h: &mut Harry, g: &mut PlayerGlobals, input: &KeyInput, rng: &mut Random, events: &mut Vec<KeyEvent>) {
    let (v, hh) = h.player.body.pos;
    events.push(KeyEvent::Explosion { h: hh, v, flag: -1 });
    let voice = |events: &mut Vec<KeyEvent>| {
        if !input.voice_playing {
            events.push(KeyEvent::VoiceSound(snd::VOICE));
        }
    };
    match rng.below(6) {
        0 => {
            voice(events);
            g.caffeine = 270;
            g.fire_delay = 10;
        }
        1 | 3 => {
            voice(events);
            g.comfort = 100;
        }
        2 => {
            events.push(KeyEvent::Sound(snd::VOICE));
            events.push(KeyEvent::Sound(snd::UNCOMFORTABLE));
            g.tile.comfort_delta -= 15;
        }
        4 => {
            voice(events);
            g.ammo = Ammo { staple: 100, badass: 100, shrapnel: 100 };
        }
        _ => {
            voice(events);
            // no lower bound here (BUGS.md)
            g.ammo.staple = g.ammo.staple.wrapping_sub(25);
            g.ammo.badass = g.ammo.badass.wrapping_sub(25);
            g.ammo.shrapnel = g.ammo.shrapnel.wrapping_sub(25);
        }
    }
}

/// The key sequences: P-A-R-G-H-Z → a random message, B-E-N → a fixed
/// one, W-A-V-Y-G-R-A-V-Y → the ripple on / off. The first key opens a
/// window (200 / 200 / 250 frames, counted down by `HandlePlayerSprite`);
/// each further key must be pressed within it. The last step fires one
/// frame after its key.
fn sequences(g: &mut PlayerGlobals, k: &KeyMap, rng: &mut Random, events: &mut Vec<KeyEvent>) {
    if k.down(code::P) && g.cheat1 == 0 {
        g.cheat1 = 200;
        g.cheat1_step = 1;
    }
    if k.down(code::B) && g.cheat2 == 0 {
        g.cheat2 = 200;
        g.cheat2_step = 1;
    }
    if k.down(code::W) && g.wavy == 0 {
        g.wavy = 250;
        g.wavy_step = 1;
    }
    if g.cheat1 > 0 {
        match g.cheat1_step {
            s @ 1..=5 if k.down(PARGHZ[s as usize - 1]) => g.cheat1_step += 1,
            6 => {
                events.push(KeyEvent::Balloon(rng.below(52)));
                g.cheat1_step = 0;
                g.cheat1 = 0;
            }
            _ => {}
        }
    }
    if g.cheat2 > 0 {
        match g.cheat2_step {
            s @ 1..=2 if k.down(BEN[s as usize - 1]) => g.cheat2_step += 1,
            3 => {
                events.push(KeyEvent::Balloon(-1));
                g.cheat2_step = 0;
                g.cheat2 = 0;
            }
            _ => {}
        }
    }
    if g.wavy > 0 {
        match g.wavy_step {
            s @ 1..=8 if k.down(WAVYGRAVY[s as usize - 1]) => g.wavy_step += 1,
            9 => {
                g.wavy_on = !g.wavy_on;
                events.push(KeyEvent::Ripple(g.wavy_on));
                g.wavy_step = 0;
                g.wavy = 0;
            }
            _ => {}
        }
    }
}

/// Ammunition of the three guns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ammo {
    pub staple: i16,
    pub badass: i16,
    pub shrapnel: i16,
}
