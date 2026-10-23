//! Harry's moves in his swivel chair: scoot, inch and power kick
//! (`MoveChair`, `DoScoot`, `DoKick`, `DoInch` in the original's Sprite
//! Source). Each move is a timer that a key sets and `HandlePlayerSprite`
//! counts down before `MoveChair` runs; the timer value selects the pose and
//! the thrust of the frame. The tables come from the routines' jump tables
//! (NOTES.md, "Player movement").

use crate::physics::{Physics, Speed, apply_moded_impulse};

/// Timer values set by the keys (`HandleKeys`).
pub const SCOOT_START: i16 = 21;
pub const INCH_START: i16 = 17;
pub const KICK_START: i16 = 21;

/// The move globals of the original (`gScootTimer`, `gKickTimer`,
/// `gInchTimer`, `gPoseOffset`, `gImpulse`, `gKickPower`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Moves {
    pub scoot: i16,
    pub kick: i16,
    pub inch: i16,
    /// 0, 36, 72 or 108: which of the four sets of 36 frames is drawn.
    pub pose: i16,
    /// Thrust of the current frame; keeps its value between move frames.
    pub impulse: i16,
    /// Firmness of what the kick hit, kept for kick frames 12 and 11.
    pub kick_power: i16,
}

/// Sounds the moves ask for (`STPlayRegSound` calls); played by the
/// frontend. The two numbers are the original's arguments (priority /
/// volume and pitch).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveSound {
    /// Scooting while wet (`DoScoot` frame 17, `gWet > 0`).
    ScootWater { volume: i16 },
    /// Scoot sound of the floor kind 1, 2 or 3 (frame 17).
    ScootFloor { kind: i16, pitch: i16 },
    /// The power kick hit something (frame 13).
    KickHit,
}

/// The parts of the player's sprite `MoveChair` uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chair {
    pub speed: Speed,
    /// `sp->dir`, 0–35.
    pub dir: i16,
    /// `sp->hurt`: moves are blocked from 23 on.
    pub hurt: i16,
    /// `sp->hotRect` (top, left, bottom, right); the kick widens it.
    pub hot_rect: [i16; 4],
}

/// What `MoveChair` reads from the rest of the game.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoveInput {
    /// `gCaffeineTimer`: > 0 multiplies the thrust by 2.5.
    pub caffeine: i16,
    /// `gWet` 0–200: weakens the thrust.
    pub wet: i16,
    /// `gFloorKind`, for the scoot sound.
    pub floor_kind: i16,
    /// `gKickContact`: firmness touched this frame (0 = nothing).
    pub kick_contact: i16,
}

/// `DoScoot` (PPC 0x100279c0); `p` is the thrust of `MoveChair`.
fn scoot(m: &mut Moves, p: i32, input: &MoveInput, sounds: &mut Vec<MoveSound>) {
    let t = m.scoot;
    if t <= 0 || !(3..=20).contains(&t) {
        return;
    }
    let p16 = i32::from(p as i16);
    match t {
        3 => m.pose = 0,
        5 => m.impulse = 0,
        6 => {
            m.pose = 72;
            m.impulse = 0;
        }
        7 => m.impulse = (p16 >> 1) as i16,
        8 => m.impulse = p16 as i16,
        9 => m.impulse = p.wrapping_add(p16 >> 1) as i16,
        10 => {
            m.pose = 108;
            m.impulse = (p << 1) as i16;
        }
        11 | 15 => m.impulse = p.wrapping_mul(3) as i16,
        12 | 14 => m.impulse = (p << 2) as i16,
        13 => {
            m.pose = 72;
            m.impulse = (p << 2) as i16;
        }
        16 => {
            m.pose = 0;
            m.impulse = (p << 1) as i16;
        }
        17 => {
            m.pose = 36;
            m.impulse = p16 as i16;
            if input.wet > 0 {
                sounds.push(MoveSound::ScootWater { volume: input.wet >> 1 });
            }
            if (1..=3).contains(&input.floor_kind) {
                sounds.push(MoveSound::ScootFloor { kind: input.floor_kind, pitch: 0x80 - (input.wet >> 1) });
            }
        }
        18..=20 => m.pose = 36,
        _ => {}
    }
}

/// `DoKick` (PPC 0x10027c18); `k` = physics kick value.
fn kick(m: &mut Moves, chair: &mut Chair, k: i32, input: &MoveInput, sounds: &mut Vec<MoveSound>) {
    let t = m.kick;
    if t <= 0 || !(4..=20).contains(&t) {
        return;
    }
    let k16 = i32::from(k as i16);
    match t {
        4 => m.pose = 0,
        6 => m.pose = 36,
        7 => m.impulse = 0,
        8 => {
            m.pose = 0;
            m.impulse = 0;
        }
        10 | 15 | 18 => m.pose = 72,
        11 => m.impulse = ((k16 * 7).wrapping_mul(i32::from(m.kick_power)) >> 8) as i16,
        12 => m.impulse = ((k16 * 9).wrapping_mul(i32::from(m.kick_power)) >> 8) as i16,
        13 => {
            if input.kick_contact != 0 {
                m.kick_power = input.kick_contact;
                m.impulse = ((k16 * 10).wrapping_mul(i32::from(input.kick_contact)) >> 8) as i16;
                sounds.push(MoveSound::KickHit);
            } else {
                // A kick into the air: skip to frame 11 with a small push.
                m.kick = 11;
                m.impulse = 50;
            }
        }
        14 => {
            m.pose = 108;
            chair.hot_rect = [0, 0, 64, 64];
        }
        20 => {
            m.pose = 72;
            m.kick = 16;
        }
        _ => {}
    }
}

/// `DoInch` (PPC 0x10027dd0): like a scoot backwards, ending with a short
/// push forwards.
fn inch(m: &mut Moves, p: i32) {
    let t = m.inch;
    if t <= 0 || !(3..=16).contains(&t) {
        return;
    }
    let half = i32::from(p as i16) >> 1;
    match t {
        3..=5 => {
            m.impulse = 0;
            m.pose = 0;
        }
        8 => {
            m.impulse = (p << 1) as i16;
            m.pose = 36;
        }
        9 => m.impulse = (-half) as i16,
        10 | 15 => m.impulse = p.wrapping_neg() as i16,
        11 => {
            m.impulse = p.wrapping_add(half).wrapping_neg() as i16;
            m.pose = 0;
        }
        12 | 13 => m.impulse = (p << 1).wrapping_neg() as i16,
        14 => {
            m.pose = 72;
            m.impulse = p.wrapping_add(half).wrapping_neg() as i16;
        }
        16 => {
            m.pose = 72;
            m.impulse = (-half) as i16;
        }
        _ => {}
    }
}

/// `MoveChair` (PPC): the thrust of this frame from the running moves,
/// applied in the direction Harry faces.
pub fn move_chair(
    chair: &mut Chair,
    m: &mut Moves,
    physics: &Physics,
    input: &MoveInput,
    sounds: &mut Vec<MoveSound>,
) {
    let mut p = i32::from(physics.scoot);
    if input.caffeine > 0 {
        p = (f64::from(p) * 2.5) as i32;
    }
    if input.wet > 0 {
        p = (256 - i32::from(input.wet)) * i32::from(p as i16) >> 8;
    }
    if chair.hurt < 23 {
        scoot(m, p, input, sounds);
        kick(m, chair, i32::from(physics.kick), input, sounds);
        inch(m, p);
        apply_moded_impulse(&mut chair.speed, chair.dir, m.impulse);
    } else {
        m.inch = 0;
        m.kick = 0;
        m.scoot = 0;
        m.pose = 0;
    }
}
