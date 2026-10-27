//! Speed, thrust and friction of sprites (Sprite Source / Sources in the
//! original).
//!
//! Speeds are in 1/256 pixel per frame and stored as 16-bit values like in
//! the original's sprite record (`speedV` +0x4e, `speedH` +0x50); all
//! arithmetic wraps like the original's `short` stores.

/// A sprite's speed (the original's `Point speed`: v first, then h).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Speed {
    pub h: i16,
    pub v: i16,
}

/// The five physics values of a level (`gPhysics`, `LoadLevelPhysics`):
/// level offset 0x12102.. when 0x12100 is set, else the defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Physics {
    /// Base thrust of scoot and inch.
    pub scoot: i16,
    /// Base thrust of the power kick.
    pub kick: i16,
    /// Friction on normal floor (floor kind 1), speed × f / 256 per frame.
    pub friction: i16,
    /// Friction on slippery floor (floor kind 3).
    pub slippery: i16,
    /// Bounce factor at walls, speed × b / 256.
    pub bounce: i16,
}

impl Default for Physics {
    fn default() -> Self {
        Physics { scoot: 50, kick: 70, friction: 245, slippery: 252, bounce: 150 }
    }
}

/// The sine table of `ApplyModedImpulse`: sin(10° · k) as the original's
/// source typed it, to three digits (data 0x100667f0..0x10066828).
pub const SINES: [f64; 10] = [0.0, 0.174, 0.342, 0.5, 0.643, 0.766, 0.866, 0.94, 0.985, 1.0];

/// `speed ± k · p` as the PowerPC build computes it: both in double, one
/// rounding for the product and one for the sum (no fused multiply-add),
/// then `fctiwz` (truncation toward zero) and a 16-bit store.
fn add_scaled(speed: i16, k: f64, p: i16, negative: bool) -> i16 {
    let product = k * f64::from(p);
    let sum = if negative { f64::from(speed) - product } else { f64::from(speed) + product };
    sum as i32 as i16
}

/// `ApplyModedImpulse` (PPC 0x1001f414): add thrust `p` in direction
/// `mode` (0–35, 10° steps; 0 = left, 9 = down, 18 = right, 27 = up).
/// Other modes do nothing.
pub fn apply_moded_impulse(speed: &mut Speed, mode: i16, p: i16) {
    let s = &SINES;
    match mode {
        0 => speed.h = speed.h.wrapping_sub(p),
        9 => speed.v = speed.v.wrapping_add(p),
        18 => speed.h = speed.h.wrapping_add(p),
        27 => speed.v = speed.v.wrapping_sub(p),
        1..=8 => {
            let k = mode as usize;
            speed.h = add_scaled(speed.h, s[9 - k], p, true);
            speed.v = add_scaled(speed.v, s[k], p, false);
        }
        10..=17 => {
            let k = (mode - 9) as usize;
            speed.h = add_scaled(speed.h, s[k], p, false);
            speed.v = add_scaled(speed.v, s[9 - k], p, false);
        }
        19..=26 => {
            let k = (mode - 18) as usize;
            speed.h = add_scaled(speed.h, s[9 - k], p, false);
            speed.v = add_scaled(speed.v, s[k], p, true);
        }
        28..=35 => {
            let k = (mode - 27) as usize;
            speed.h = add_scaled(speed.h, s[k], p, true);
            speed.v = add_scaled(speed.v, s[9 - k], p, true);
        }
        _ => {}
    }
}

/// `InverseMode`: the opposite direction.
pub fn inverse_mode(mode: i16) -> i16 {
    if mode < 18 { mode + 18 } else { mode - 18 }
}

/// `EnforceMaxSpeed` (PPC): limit each axis to `max`, scaling the other
/// axis by the same factor (integer division, truncating). The horizontal
/// axis is checked first.
pub fn enforce_max_speed(speed: &mut Speed, max: i32) {
    let limit = max as i16;
    let h = i32::from(speed.h).abs();
    if max < h {
        speed.v = (i32::from(speed.v) * max / h) as i16;
        speed.h = if speed.h < 1 { limit.wrapping_neg() } else { limit };
    }
    let v = i32::from(speed.v).abs();
    if max < v {
        speed.h = (i32::from(speed.h) * max / v) as i16;
        speed.v = if speed.v < 1 { limit.wrapping_neg() } else { limit };
    }
}

/// One axis of the floor friction in `HandlePlayerGlide`: below 25 the
/// speed stops (unless gliding), else it is scaled by f / 256.
fn friction_axis(s: i16, f: i16, gliding: bool) -> i16 {
    if i32::from(s).abs() < 25 {
        if gliding { s } else { 0 }
    } else {
        ((i32::from(f) * i32::from(s)) >> 8) as i16
    }
}

/// The state `HandlePlayerGlide` reads besides the speed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Floor {
    /// `sp->air` (jump state): friction applies at 0 and at 24–199.
    pub air: i16,
    /// `gFloorKind`: 1 normal, 2 (fixed 236), 3 slippery; others none.
    pub kind: i16,
    /// `gWet` 0–200: raises the friction factor toward 256.
    pub wet: i16,
    /// The glide counter (`PTR_DAT_10065e60`): ≥ 88 halves the friction.
    pub glide: i16,
}

/// `HandlePlayerGlide` (PPC 0x10026b34): floor friction for the player.
/// Returns the new surface resistance meter value when the floor kind sets
/// it (kinds 1 and 3: 5 · f − 1150 from the level's friction value, at
/// least 8), else `None`.
pub fn player_glide(speed: &mut Speed, floor: Floor, physics: &Physics) -> Option<i16> {
    if !(floor.air == 0 || (floor.air > 23 && floor.air < 200)) {
        return None;
    }
    let base = match floor.kind {
        2 => {
            speed.h = ((i32::from(speed.h) * 236) >> 8) as i16;
            speed.v = ((i32::from(speed.v) * 236) >> 8) as i16;
            return None;
        }
        1 => physics.friction,
        3 => physics.slippery,
        _ => return None,
    };
    let mut f = base;
    if floor.wet >= 1 {
        f = base.wrapping_add((((256 - i32::from(base)) * i32::from(floor.wet)) >> 8) as i16);
    }
    let gliding = floor.glide >= 0x58;
    if gliding {
        f = ((i32::from(f) + 256) >> 1) as i16;
    }
    speed.h = friction_axis(speed.h, f, gliding);
    speed.v = friction_axis(speed.v, f, gliding);
    let b = i32::from(base);
    let surface = (b + (b - 230) * 4 - 230) as i16;
    Some(surface.max(8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sines_have_the_original_bits() {
        // data 0x100667f0.. of the PPC build, in the order of the table there
        let bits = [
            (0.643, 0x3fe49374bc6a7efa_u64),
            (0.766, 0x3fe883126e978d50),
            (0.5, 0x3fe0000000000000),
            (0.866, 0x3febb645a1cac083),
            (0.342, 0x3fd5e353f7ced917),
            (0.94, 0x3fee147ae147ae14),
            (0.174, 0x3fc645a1cac08312),
            (0.985, 0x3fef851eb851eb85),
        ];
        for (v, b) in bits {
            assert_eq!(f64::to_bits(v), b);
            assert!(SINES.contains(&v));
        }
    }
}
