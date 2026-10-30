//! Harry's death (`HandlePlayerSpriteDeath`): with comfort or favor shown
//! at 0 and on the ground, `HandlePlayerSprite` hands him to this task. He
//! spins faster and faster, bursts in flames and explosions at frame 45,
//! is gone from frame 52 and at frame 110 ends the game loop.

use crate::frame::PlayerGlobals;
use crate::game::PLAYER_FACES;
use crate::sprite::Sprite;

/// `snd` ids.
pub mod snd {
    /// Frame 18, comfort gone (`_DAT_10065b40`).
    pub const NO_COMFORT: i16 = 4901;
    /// Frame 18, favor gone (`_DAT_10065b44`).
    pub const NO_FAVOR: i16 = 4900;
    /// The burst.
    pub const BURST: i16 = 602;
}

/// What the death asks of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathEvent {
    /// `FadeMusic(0, 1)`.
    FadeMusic,
    /// `STPlayRegSound(id, 100, 0x100)`.
    Sound(i16),
    /// `EasyExplosion(h, v, 0, 0, 0, dir)`; `behind` sets its +0x8b.
    Explosion { h: i16, v: i16, dir: i16, behind: bool },
    /// A flame `SATNewSprite(kind, h, v, SetupEffectSprite)` with `f6e` 3
    /// (its last round), `f7c` its kind and +0x8b.
    Flame { kind: i16, h: i16, v: i16 },
    /// The view shakes (`_DAT_10065ad0`).
    Shake(i16),
}

/// `HandlePlayerSpriteDeath`, one frame. Every frame clears the shot kind
/// and the recoil. Frame 0: the music fades, no hit tasks, no current item.
/// Frames 1–51 turn him (2, then 3, 4 and 5 steps a frame); frame 18 says
/// why (comfort first); frame 45 makes an explosion on him, four flames
/// out of his centre (left, right, up, down), four explosions about it,
/// the burst sound and a shake of 36. His face is the standing one of his
/// direction until frame 51, then none. Frame 110 freezes him and returns
/// true: the game loop ends.
pub fn handle_player_death(sp: &mut Sprite, g: &mut PlayerGlobals, events: &mut Vec<DeathEvent>) -> bool {
    let (h, v) = (sp.at.1.wrapping_add(32), sp.at.0.wrapping_add(32));
    let mut done = false;
    g.shot_kind = 0;
    g.recoil = 0;
    match sp.timer {
        0 => {
            events.push(DeathEvent::FadeMusic);
            sp.hit_task = None;
            sp.hit_tile_task = None;
            g.cur_slot = -1;
        }
        1..=17 => sp.dir -= 2,
        18 => {
            sp.dir -= 2;
            if g.comfort_shown < 1 {
                events.push(DeathEvent::Sound(snd::NO_COMFORT));
            } else if g.favor_shown < 1 {
                events.push(DeathEvent::Sound(snd::NO_FAVOR));
            }
        }
        19..=30 => sp.dir -= 3,
        31..=44 => sp.dir -= 4,
        45 => {
            sp.dir -= 4;
            let (ah, av) = (sp.at.1, sp.at.0);
            events.push(DeathEvent::Explosion { h: ah, v: av, dir: 4, behind: false });
            events.push(DeathEvent::Flame { kind: 4204, h: h - 64, v: v - 12 });
            events.push(DeathEvent::Flame { kind: 4203, h, v: v - 12 });
            events.push(DeathEvent::Flame { kind: 4201, h: h - 12, v: v - 64 });
            events.push(DeathEvent::Flame { kind: 4202, h: h - 12, v });
            for (dh, dv) in [(-16, -16), (16, -16), (16, 16), (-16, 16)] {
                events.push(DeathEvent::Explosion { h: ah + dh, v: av + dv, dir: 0, behind: true });
            }
            events.push(DeathEvent::Sound(snd::BURST));
            events.push(DeathEvent::Shake(36));
        }
        46..=51 => sp.dir -= 5,
        110 => {
            done = true;
            sp.frozen = true;
        }
        _ => {}
    }
    if sp.dir >= 36 {
        sp.dir -= 35;
    } else if sp.dir < 0 {
        sp.dir += 35;
    }
    sp.face = if sp.timer < 52 { PLAYER_FACES + sp.dir } else { 0 };
    sp.timer += 1;
    done
}
