//! Effects: explosions (`EasyExplosion`, `SetupEffectSprite`,
//! `HandleEffectSprite`) and how they push sprites (`HandleExplodeRadius`).
//! Effects have no hit task; the sprites they touch react in their own hit
//! tasks (plants, coolers, Harry). The fire pipes' flames (kinds 4200 and
//! up) are effects too and come with the fire pipes.

use crate::physics::{Speed, apply_moded_impulse, find_desired_direction};
use crate::sprite::{Sprite, Task};

/// The explosion's 13 faces, HrSp 2701….
pub const EXPLOSION_FACES: i16 = 2701;

/// The first fire kind (`0x1068`): `f7c` below it means an explosion.
pub const FIRE: i16 = 4200;

/// `SetupEffectSprite` for an explosion (kind below 4200): layer 20, no
/// hit task, timer 10, the first face, hot rect (4, 4, 60, 60) unless the
/// kind is negative (then none). `EasyExplosion` then sets speed, `dir`
/// and the kind.
pub fn setup_effect(sp: &mut Sprite) {
    sp.init();
    sp.task = Some(Task::Effect);
    sp.hit_task = None;
    sp.hit_tile_task = None;
    sp.layer = 20;
    sp.face = 0;
    sp.f6e = 0;
    sp.dir = 0;
    sp.timer = 10;
    sp.speed = Speed::default();
    sp.hot_rect = [0; 4];
    assert!(sp.kind < FIRE, "fire pipe flames are not ported yet");
    sp.b8b = false;
    sp.face = EXPLOSION_FACES;
    if sp.kind >= 0 {
        sp.hot_rect = [4, 4, 60, 60];
    }
    sp.at_from_pos();
}

/// `HandleEffectSprite`, explosions: the face of frame `dir` / 2 (none
/// outside 0–25), `dir` + 1, gone after frame 26; moves by its speed (whole
/// pixels, `pos` and `at`).
pub fn handle_effect(sp: &mut Sprite) {
    if sp.frozen {
        return;
    }
    assert!(sp.f7c < FIRE, "fire pipe flames are not ported yet");
    sp.face = if (0..=25).contains(&sp.dir) { EXPLOSION_FACES + sp.dir / 2 } else { 0 };
    sp.dir += 1;
    if sp.dir > 26 {
        sp.frozen = true;
    }
    let h = sp.pos.1.wrapping_add(sp.speed.h);
    let v = sp.pos.0.wrapping_add(sp.speed.v);
    sp.pos = (v, h);
    sp.at = (v, h);
}

/// `PEDistance`: the larger of |a| and |b| plus a third of the smaller.
pub fn pe_distance(a: i16, b: i16) -> i32 {
    let (a, b) = (i32::from(a).abs(), i32::from(b).abs());
    a.max(b) + a.min(b) / 3
}

/// `HandleExplodeRadius(sprite, half, effect)`: an explosion (not a
/// flame) pushes a sprite that has not been pushed in the last 30 frames
/// (`blast_cooldown`, +0x96, counted down by `EraseSprites`): within 32 px
/// (`pe_distance` of the centres - the sprite's at + `half`, the
/// effect's at + 32) by (32 − distance, at most 16) × 128 away from it.
/// Returns whether the sprite was in range (the cooldown starts either
/// way).
pub fn explode_radius(sp_at: (i16, i16), speed: &mut Speed, blast: &mut i16, half: i16, effect: &Sprite) {
    if effect.f7c >= FIRE || *blast >= 1 {
        return;
    }
    *blast = 30;
    let c = (sp_at.0.wrapping_add(half), sp_at.1.wrapping_add(half));
    let e = (effect.at.0.wrapping_add(32), effect.at.1.wrapping_add(32));
    let dh = (i32::from(c.1) - i32::from(e.1)).abs() as i16;
    let dv = (i32::from(c.0) - i32::from(e.0)).abs() as i16;
    let d = pe_distance(dh, dv) as i16;
    if d < 32 {
        let force = (32 - d).min(16);
        let dir = find_desired_direction(c, e);
        apply_moded_impulse(speed, dir, force << 7);
    }
}
