//! Harry, the player sprite: his reactions to tiles (`HitPlayerTileSprite`).

use crate::bounce::{Body, Rect, bg_tile_bounce, rect_bounce_fake, wall_bounce};
use crate::level::Level;
use crate::mask::{Mask, mask_hit};
use crate::physics::Physics;
use crate::tiles::{KICK_RECTS, TileHit, bg_rect, fg_rect, offset, sect};

/// Harry's hot rect relative to his face when not kicking.
pub const HOT_RECT: Rect = [22, 22, 42, 42];
/// The hot rect during the kick's reach frame.
pub const KICK_HOT_RECT: Rect = [0, 0, 64, 64];

/// The player sprite's fields the collision code uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub body: Body,
    /// 0–35, the direction of travel.
    pub dir: i16,
    /// Pose offset (0, 36, 72, 108): the face drawn is `dir + pose`.
    pub pose: i16,
    /// `sp->air` (jump state).
    pub air: i16,
    /// `sp->hotRect`; `KICK_HOT_RECT` while the kick reaches out.
    pub hot_rect: Rect,
}

impl Player {
    fn kicking(&self) -> bool {
        self.hot_rect[1] == 0
    }
}

/// The globals `HitPlayerTileSprite` reads and writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileState {
    /// A foreground wall was hit this frame (`PTR_DAT_10065e58`).
    pub wall_contact: bool,
    /// A background obstacle was hit this frame (`PTR_DAT_10065e54`).
    pub bg_contact: bool,
    /// Frames until damage tiles hurt again (`PTR_DAT_10065e44`).
    pub ouch_cooldown: i16,
    /// `gComfortDelta`, applied once per frame by `HandlePlayerSprite`.
    pub comfort_delta: i16,
    /// `gWet`.
    pub wet: i16,
    /// `gFloorKind`.
    pub floor_kind: i16,
    /// `gKickContact`.
    pub kick_contact: i16,
}

/// The collision masks (NOTES.md "Collision masks").
pub struct Masks {
    /// Harry's body (cicn 299).
    pub body: Mask,
    /// The background obstacle (cicn 290).
    pub obstacle: Mask,
    /// The 32 wall shapes (PICT 160).
    pub walls: Vec<Mask>,
    /// Masks of Harry's faces 107–143 (`None` for the others).
    pub faces: Vec<Option<Mask>>,
}

/// Sounds the tile reactions ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileSound {
    /// `Ouch`: one of three voice samples (picked with `SATRand(100)`
    /// unless a voice is playing or caffeine works).
    Ouch,
    /// Rolling into water (`STPlay3DSoundPitched`, pitch 55000), only if
    /// that sound is not playing already.
    Water,
}

/// The kick's pixel test against a tile mask: sets the contact and puts
/// Harry back where he was (`HitPlayerTileSprite`).
fn kick_test(p: &mut Player, st: &mut TileState, masks: &Masks, hit: &TileHit, tile_rect: Rect, tile_mask: &Mask) {
    let foot = offset(KICK_RECTS[p.dir.rem_euclid(36) as usize], p.body.at.0, p.body.at.1);
    if !sect(foot, offset(tile_rect, hit.tile.0, hit.tile.1)) {
        return;
    }
    let Some(face) = masks.faces.get((p.dir + p.pose) as usize).and_then(Option::as_ref) else { return };
    let saved = p.body.at;
    if mask_hit(&mut p.body.pos, face, hit.tile, tile_mask) {
        st.kick_contact = 256;
        p.body.at = saved;
        p.body.pos = saved;
    }
}

/// `HitPlayerTileSprite` (PPC): Harry touches a tile. `sounds` receives the
/// sounds to play.
pub fn hit_player_tile(
    p: &mut Player,
    st: &mut TileState,
    level: &Level,
    masks: &Masks,
    physics: &Physics,
    hit: TileHit,
    sounds: &mut Vec<TileSound>,
) {
    let tile = hit.tile;
    let kind = hit.kind;
    if hit.foreground {
        if p.kicking() {
            if kind < 0x37 {
                if let Some(wall) = masks.walls.get(kind as usize) {
                    kick_test(p, st, masks, &hit, fg_rect(kind), wall);
                }
            }
        } else if kind < 32 {
            let first = !st.wall_contact;
            let body_mask = &masks.body;
            let wall = &masks.walls[kind.max(0) as usize];
            let r = wall_bounce(&mut p.body, kind, tile, 32, physics.bounce, HOT_RECT, first, &mut |b| {
                mask_hit(&mut b.pos, body_mask, tile, wall)
            });
            if r as u8 != 0 {
                st.wall_contact = true;
            }
        } else if kind < 0x38 {
            rect_bounce_fake(&mut p.body, tile, bg_rect(kind), 32, i32::from(physics.bounce), HOT_RECT);
            st.wall_contact = true;
        }
        return;
    }
    if p.air != 0 {
        return;
    }
    match kind {
        2 => st.floor_kind = 3,
        1 if !st.bg_contact => {
            if p.kicking() {
                kick_test(p, st, masks, &hit, bg_rect(1), &masks.obstacle);
            } else {
                let body_mask = &masks.body;
                let obstacle = &masks.obstacle;
                let fg = |x: i32, y: i32| level.fg_kind(level.fg_tile(x, y));
                let r = bg_tile_bounce(&mut p.body, tile, 32, physics.bounce, HOT_RECT, true, &mut |b| {
                    mask_hit(&mut b.pos, body_mask, tile, obstacle)
                }, &fg);
                st.bg_contact = r as u8 != 0;
            }
        }
        3 | 4 => {
            if st.ouch_cooldown == 0 && !p.kicking() {
                st.comfort_delta -= if kind == 3 { 7 } else { 12 };
                st.ouch_cooldown = 15;
                sounds.push(TileSound::Ouch);
            }
        }
        5 => st.comfort_delta += 1,
        6 if !p.kicking() => {
            if st.wet < 0xab {
                sounds.push(TileSound::Water);
            }
            if st.wet < 200 {
                st.wet = (st.wet + 25).min(200);
            }
        }
        _ => {}
    }
}
