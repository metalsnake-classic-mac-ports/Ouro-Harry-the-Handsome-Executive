//! Half pipes, jumps and falling into holes (`HandleHalfPipe`, `JumpScale`).
//!
//! `air` (sprite +0x8e) is the jump state: 0 on the floor; 1–35 a jump
//! (the sprite grows and shrinks; from 23 on it cannot move); 215–220 the
//! flip at a half pipe's edge; 999–1045 falling into a hole (background kind
//! 7): shrinking, spinning, then back at the level's fall location.

use crate::bounce::Body;
use crate::level::Level;
use crate::random::Random;

/// The sprite fields of the jump state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Air {
    pub air: i16,
    /// Background kind under the sprite at the last `half_pipe` (+0x9c).
    pub slide: i16,
    /// The tile of the last `half_pipe` (h, v) (+0x98, +0x9a).
    pub last_tile: (i16, i16),
    /// Which way a half-pipe flip throws (+0x90): 0x26ad down, 0x26ae up,
    /// 0x26af right, 0x26b0 left.
    pub flip: i16,
    /// `scale0` = `scale1` (+0x52, +0x54): 0x100 = normal size.
    pub scale: i16,
}

/// Sounds asked for; the original passes them in as parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirSound {
    /// Thrown off a half pipe (`HandleHalfPipe`'s sound).
    HalfPipeOut,
    /// `JumpScale`'s first sound: landing (jump frame 23, flip every 4th frame).
    Land,
    /// The second: bouncing (jump frame 29, end of a flip).
    Bounce,
    /// The third: starting to fall into a hole.
    Fall,
    /// Back from the hole (comfort changed by the level).
    Return,
}

/// `HandleHalfPipe` (PPC): half-pipe tiles (background kinds 8–15) push
/// the sprite toward their middle; crossing the edge throws it off.
/// `player`: Harry pushes harder than other sprites.
pub fn half_pipe(body: &mut Body, a: &mut Air, half: i16, player: bool, level: &Level, sounds: &mut Vec<AirSound>) {
    let h = (body.fix.0 >> 8) + i32::from(half);
    let v = (body.fix.1 >> 8) + i32::from(half);
    let (sh, sv) = (h as i16, v as i16);
    let (tx, ty) = (sh >> 5, sv >> 5);
    let mid_h = i32::from(tx) * 32 + 16;
    let mid_v = i32::from(ty) * 32 + 16;
    let kind = level.bg_kind(level.bg_tile(i32::from(sh >> 5), i32::from(sv >> 5)));
    if a.air == 0 || (a.air > 23 && a.air < 200) {
        let s = &mut body.speed;
        if kind > 7 {
            let d = if kind < 12 { mid_h - h } else { mid_v - v };
            let (big, small, k) = if player { (0x90i16, 0x30i16, (d as i16).wrapping_mul(3)) } else { (0x60, 0x20, (d << 1) as i16) };
            match kind {
                8 => s.h = s.h.wrapping_add(big).wrapping_add(k),
                9 => s.h = s.h.wrapping_add(small).wrapping_add(k),
                10 => s.h = s.h.wrapping_sub(small.wrapping_sub(k)),
                11 => s.h = s.h.wrapping_sub(big.wrapping_sub(k)),
                12 => s.v = s.v.wrapping_add(big).wrapping_add(k),
                13 => s.v = s.v.wrapping_add(small).wrapping_add(k),
                14 => s.v = s.v.wrapping_sub(small.wrapping_sub(k)),
                15 => s.v = s.v.wrapping_sub(big.wrapping_sub(k)),
                _ => {}
            }
        }
        let mut out = false;
        if !(8..=15).contains(&a.slide) {
            let (mh, mv) = (mid_h as i16, mid_v as i16);
            let flip = match kind {
                8 if sh < mh && s.h > 0 => Some(0x26af),
                11 if mh < sh && s.h < 0 => Some(0x26b0),
                15 if mv < sv && s.v < 0 => Some(0x26ae),
                12 if sv < mv && s.v > 0 => Some(0x26ad),
                _ => None,
            };
            if let Some(f) = flip {
                a.air = 0xd7;
                a.flip = f;
            }
        } else {
            out = match a.slide {
                12 => kind != 12 && s.v < 0,
                8 => kind != 8 && s.h < 0,
                11 => kind != 11 && s.h > 0,
                15 => kind != 15 && s.v > 0,
                _ => false,
            };
        }
        if out {
            a.air = 1;
            sounds.push(AirSound::HalfPipeOut);
        }
    }
    a.last_tile = (tx, ty);
    a.slide = kind;
}

/// What `JumpScale` asks the game to do besides changing the sprite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JumpEffects {
    /// Fade out and centre the view on (v, h) (frame 1030 of a fall).
    pub recentre: Option<(i16, i16)>,
    /// Redraw the whole view (1031), fade in (1032).
    pub redraw: bool,
    pub fade_in: bool,
}

/// The scale `JumpScale` sets for jump frames 1–35 (relative to 0x100).
const JUMP_SCALE: [i16; 35] = [
    0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0x40, 0x45, 0x4b, 0x50, 0x50, 0x4b, 0x45, 0x40, 0x3b, 0x35, 0x30, 0x28, 0x20,
    0x18, 0x10, 8, 0, 8, 0x10, 0x18, 0x20, 0x18, 0x10, 8, 0, 8, 0x10, 8, 0,
];

/// The value the original adds when no case of `JumpScale` sets the scale
/// (air 999, the frame a sprite lands on a hole): register r30 as the
/// caller left it (BUGS.md). In `HandlePlayerSprite` it holds the address
/// 0x10068aa4 - so the scale becomes 0x8aa4 + 0x100 for one frame.
pub const UNSET_SCALE: i16 = 0x8aa4u16 as i16;
/// The same for `HandlePlantSprite` (r30 = 0x1007dfac, the address of the
/// plant's sound).
pub const PLANT_UNSET_SCALE: i16 = 0xdfacu16 as i16;

/// `JumpScale` (PPC 0x100223a8) (`is_player`: spinning and the view
/// follow). `comfort` is `gComfort`; `unset` the caller's r30 (see
/// `UNSET_SCALE`).
#[allow(clippy::too_many_arguments)]
pub fn jump_scale(
    body: &mut Body,
    a: &mut Air,
    dir: &mut i16,
    half: i16,
    is_player: bool,
    level: &Level,
    rng: &mut Random,
    comfort: &mut i16,
    unset: i16,
    sounds: &mut Vec<AirSound>,
) -> JumpEffects {
    let mut fx = JumpEffects::default();
    let kind = level.bg_kind(level.bg_tile(
        (i32::from(body.at.1) + i32::from(half)) >> 5,
        (i32::from(body.at.0) + i32::from(half)) >> 5,
    ));
    if a.air == 0 && kind == 7 {
        a.air = 1000;
    }
    if a.air == 0 {
        return fx;
    }
    let turn = |d: &mut i16| {
        *d += 1;
        if *d > 35 {
            *d -= 36;
        }
    };
    let mut scale = unset;
    let air = a.air;
    if air < 200 {
        if (1..=35).contains(&air) {
            scale = JUMP_SCALE[(air - 1) as usize];
        }
        match air {
            23 => {
                if kind == 7 {
                    a.air = 999;
                } else {
                    sounds.push(AirSound::Land);
                }
            }
            29 => {
                if kind == 7 {
                    scale = 0;
                    a.air = 999;
                } else {
                    sounds.push(AirSound::Bounce);
                }
            }
            35 => a.air = -1,
            _ => {}
        }
    } else if air < 300 {
        let s = &mut body.speed;
        match a.flip {
            0x26af if s.h < 0x200 => s.h = 0x200,
            0x26ad if s.v < 0x200 => s.v = 0x200,
            0x26ae if -0x200 < s.v => s.v = -0x200,
            // tests the vertical speed but sets the horizontal one (BUGS.md)
            0x26b0 if -0x200 < s.v => s.h = -0x200,
            _ => {}
        }
        match air % 4 {
            2 => scale = 0x20,
            0 => {
                scale = 0;
                sounds.push(AirSound::Land);
            }
            1 | 3 => scale = 0x10,
            _ => {}
        }
        if air > 0xdc {
            scale = 0;
            a.air = -1;
            sounds.push(AirSound::Bounce);
        }
    } else if air > 999 {
        if air == 1000 {
            sounds.push(AirSound::Fall);
            scale = -6;
        } else if air < 1030 {
            scale = (1000 - air) * 6;
            let s = &mut body.speed;
            s.h = (f64::from(s.h) * 0.8) as i32 as i16;
            s.v = (f64::from(s.v) * 0.8) as i32 as i16;
            if is_player {
                turn(dir);
            }
        } else if air == 1030 {
            let rh = i32::from(rng.below(7));
            body.fix.0 = (rh + (i32::from(level.info[1]) - i32::from(half)) - 3) * 256;
            let rv = i32::from(rng.below(7));
            body.fix.1 = (rv + (i32::from(level.info[2]) - i32::from(half)) - 3) * 256;
            let h = (body.fix.0 >> 8) as i16;
            let v = (body.fix.1 >> 8) as i16;
            body.pos = (v, h);
            body.at = (v, h);
            scale = 200;
            if is_player {
                fx.recentre = Some((v.wrapping_sub(0xd0), h.wrapping_sub(0xd0)));
            }
        } else if air < 1046 {
            if air == 1031 && is_player {
                fx.redraw = true;
            }
            if air == 1032 && is_player {
                fx.fade_in = true;
            }
            body.speed.h = 0;
            body.speed.v = 0;
            scale = (0x415 - air) * 8 + 0x50;
            if air == 0x415 {
                a.air = 10;
                if is_player {
                    *comfort = comfort.wrapping_add(level.info[3]);
                    sounds.push(AirSound::Return);
                }
            }
            if is_player {
                if a.air < 0x40e || a.air & 1 != 0 {
                    turn(dir);
                }
            }
        }
    }
    a.air = a.air.wrapping_add(1);
    a.scale = scale.wrapping_add(0x100);
    fx
}
