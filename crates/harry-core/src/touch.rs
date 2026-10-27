//! Harry touches another sprite (`HitPlayerSprite`, PPC 0x10029e74, called
//! by `CheckPlayerCollision` for every sprite whose hot rect overlaps his).
//! One long chain by the other sprite's task; the branches come with the
//! sprite kinds.

use crate::frame::{Harry, PlayerGlobals};
use crate::inventory::{get_item, item};
use crate::sprite::{Sprite, Task};

/// What touching asks of the rest of the game, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchEvent {
    /// `STPlayRegSound`.
    Sound(i16),
    /// A sound that becomes the voice (only when no voice is playing).
    VoiceSound(i16),
    /// `HaltTalkingSounds`: stop all voice samples.
    HaltTalking,
}

/// `snd` ids.
pub mod snd {
    pub const VOICE_COMFORT_10: i16 = 210;
    pub const VOICE_COMFORT_8: i16 = 211;
    pub const VOICE_COMFORT_20: i16 = 212;
    pub const PICK_UP: i16 = 214;
    pub const AMMO: i16 = 216;
    pub const COMFORT_FULL: i16 = 218;
    pub const AMMO_FULL: i16 = 219;
}

/// `HitPlayerSprite`. `voice_playing`: is a voice sample playing (then
/// the voice of a bonus is skipped)?
pub fn touch(h: &mut Harry, g: &mut PlayerGlobals, other: &mut Sprite, voice_playing: bool, events: &mut Vec<TouchEvent>) {
    if h.frozen || other.frozen {
        return;
    }
    let on_floor = h.player.air == 0;
    match other.task {
        Some(Task::Bonus) if on_floor => bonus(g, other, voice_playing, events),
        Some(Task::Item) if on_floor => pick_up(g, other, events),
        _ => {}
    }
}

/// Bonuses: 700–703 comfort (only when not full; 704–710 vanish without
/// effect), 711–713 ammo for the staple gun, B.A.D.A.S.S., shrapnel gun
/// (+20 while below 100, so up to 119). When full, a remark (at most every
/// 200 frames, `t24`).
fn bonus(g: &mut PlayerGlobals, other: &mut Sprite, voice_playing: bool, events: &mut Vec<TouchEvent>) {
    let full = |g: &mut PlayerGlobals, remark: i16, events: &mut Vec<TouchEvent>| {
        if g.t24 == 0 {
            g.t24 = 200;
            events.push(TouchEvent::HaltTalking);
            events.push(TouchEvent::Sound(remark));
        }
    };
    if other.kind < 711 {
        if g.comfort >= 144 {
            full(g, snd::COMFORT_FULL, events);
            return;
        }
        other.frozen = true;
        let (voice, gain) = match other.kind {
            700 | 701 => (snd::VOICE_COMFORT_10, 10),
            702 => (snd::VOICE_COMFORT_20, 20),
            703 => (snd::VOICE_COMFORT_8, 8),
            _ => return,
        };
        if !voice_playing {
            events.push(TouchEvent::VoiceSound(voice));
        }
        g.comfort = g.comfort.wrapping_add(gain);
        return;
    }
    let ammo = match other.kind {
        711 => &mut g.ammo.staple,
        712 => &mut g.ammo.badass,
        713 => &mut g.ammo.shrapnel,
        _ => return,
    };
    if *ammo < 100 {
        *ammo = ammo.wrapping_add(20);
        events.push(TouchEvent::HaltTalking);
        events.push(TouchEvent::Sound(snd::AMMO));
        other.frozen = true;
    } else {
        full(g, snd::AMMO_FULL, events);
    }
}

/// Items: a gun Harry already has gives 50 ammo (up to 100) and vanishes;
/// anything else goes into the inventory if there is room.
fn pick_up(g: &mut PlayerGlobals, other: &mut Sprite, events: &mut Vec<TouchEvent>) {
    let have = g.inventory.iter().any(|e| e.0 == other.kind);
    let mut take = true;
    let ammo = match other.kind {
        item::STAPLE_GUN => Some(&mut g.ammo.staple),
        item::BADASS => Some(&mut g.ammo.badass),
        item::SHRAPNEL_GUN => Some(&mut g.ammo.shrapnel),
        _ => None,
    };
    if let Some(ammo) = ammo {
        other.frozen = true;
        take = !have;
        *ammo = ammo.wrapping_add(50).min(100);
    }
    if take && get_item(g, other.kind) {
        other.frozen = true;
        events.push(TouchEvent::Sound(snd::PICK_UP));
    }
}
