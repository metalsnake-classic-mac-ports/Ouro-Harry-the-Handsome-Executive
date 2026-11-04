//! A level of a world: two 128 × 128 tile grids, the sprite list and the
//! level info (the original's `Hlvl` resource, see NOTES.md "Level").

use crate::physics::Physics;

/// Tiles per row and column.
pub const SIZE: usize = 128;
/// Pixels per tile side.
pub const TILE: i32 = 32;
/// Entries of the sprite list.
pub const SPRITES: usize = 256;

/// "No tile" in the foreground grid.
pub const NO_TILE: u16 = 0xffff;

/// One entry of the level's sprite list (16 bytes in the original).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LevelSprite {
    /// Byte 0: the entry is used.
    pub present: bool,
    /// Byte 1 (always 0 in Episode 1; kept).
    pub flags: u8,
    /// The editor's object id (901 door, 9200 copier, 21000.. memos, ...).
    pub id: i16,
    /// The four info fields (meaning per object kind).
    pub info: [i16; 4],
    /// Position of the sprite's top-left + 6 (v, h), in pixels.
    pub v: i16,
    pub h: i16,
}

/// A level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Level {
    pub name: String,
    /// The level's number (0x12032; `CheckWorldCorruption` compares it and
    /// the name of each Episode 1 level with fixed values).
    pub number: i16,
    /// The level brings its own sounds (byte 0x12034, `LoadLevelSounds`;
    /// level 10: the water scoot sounds).
    pub own_sounds: bool,
    /// Background tiles, column major: `bg[x * SIZE + y]`.
    pub bg: Vec<u16>,
    /// Foreground tiles, column major; `NO_TILE` = none.
    pub fg: Vec<u16>,
    pub sprites: Vec<LevelSprite>,
    /// Level info fields #0–#15 (player tint, fall location, pictures before
    /// / after, scroll music, ripple, ...).
    pub info: [i16; 16],
    /// Kind of each background tile 0–95 (info #16–#39, nibbles).
    pub bg_kinds: [u8; 96],
    /// Kind of the foreground tiles 32–55 (info #40–#51: bytes, high byte
    /// first, stored as kind + 1, so 0 = −1 = no collision -
    /// `Unstuff16BitInto8Bits`). All −1 in Episode 1.
    pub fg_kinds: [i16; 24],
    /// Start position (h, v) in pixels.
    pub start: (i16, i16),
    /// Start direction (0 = keep the default).
    pub start_dir: i16,
    pub music: i16,
    /// Foreground / background tile set PICT ids, 0 = the defaults.
    pub fg_set: i16,
    pub bg_set: i16,
    /// Physics flag (0x12100): when 0 the game uses the default physics
    /// even though `physics` may hold values (editor leftovers, e.g. level
    /// 1: 50, 70, 246, 252, 100).
    pub physics_set: bool,
    /// The stored physics values (0x12102..0x1210b).
    pub physics: Physics,
    /// Everything not decoded yet (the custom sprite tables at 0x11000 and
    /// 0x11340, empty in Episode 1; 0x1210c; old name bytes), as the original bytes with the decoded
    /// fields zeroed - so a level can be written back exactly.
    pub rest: Vec<u8>,
}

impl Level {
    /// The two custom sprite tables (`savegame::CUSTOM` of the `Hlvl`);
    /// zeros when the level does not keep them (the data pack's levels).
    pub fn custom(&self) -> Vec<u8> {
        self.rest.get(crate::savegame::CUSTOM).map_or_else(|| vec![0; crate::savegame::CUSTOM.len()], <[u8]>::to_vec)
    }

    /// Set the custom sprite tables (from a copier save).
    pub fn set_custom(&mut self, custom: &[u8]) {
        let r = crate::savegame::CUSTOM;
        if self.rest.len() < r.end {
            self.rest.resize(r.end, 0);
        }
        self.rest[r.start..r.start + custom.len().min(r.len())].copy_from_slice(&custom[..custom.len().min(r.len())]);
    }

    /// `LoadLevelPhysics`: the physics the game uses in this level.
    pub fn effective_physics(&self) -> Physics {
        if self.physics_set { self.physics } else { Physics::default() }
    }

    fn index(x: i32, y: i32) -> usize {
        (x.clamp(0, SIZE as i32 - 1) as usize) * SIZE + y.clamp(0, SIZE as i32 - 1) as usize
    }

    /// `GetBGTile`: the background tile at tile (x, y), clamped to the grid.
    pub fn bg_tile(&self, x: i32, y: i32) -> u16 {
        self.bg[Self::index(x, y)]
    }

    /// `GetFGTile`: the foreground tile at tile (x, y), clamped to the grid.
    pub fn fg_tile(&self, x: i32, y: i32) -> u16 {
        self.fg[Self::index(x, y)]
    }

    /// `LookupBGTileKind`: kind of a background tile; values outside 0–95
    /// are their own kind.
    pub fn bg_kind(&self, tile: u16) -> i16 {
        let t = tile as i16;
        if (0..96).contains(&t) { i16::from(self.bg_kinds[t as usize]) } else { t }
    }

    /// `LookupFGTileKind`: kind of a foreground tile; tiles 32–55 have a
    /// kind from the level, all others are their own kind (−1 = none).
    pub fn fg_kind(&self, tile: u16) -> i16 {
        let t = tile as i16;
        if (32..56).contains(&t) { self.fg_kinds[(t - 32) as usize] } else { t }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_tables_work_without_the_original_bytes() {
        // a pack level keeps no `rest`: a copier save there must not panic
        let mut l = Level {
            name: String::new(),
            number: 1,
            own_sounds: false,
            bg: vec![0; SIZE * SIZE],
            fg: vec![NO_TILE; SIZE * SIZE],
            sprites: vec![LevelSprite::default(); SPRITES],
            info: [0; 16],
            bg_kinds: [0; 96],
            fg_kinds: [-1; 24],
            start: (0, 0),
            start_dir: 0,
            music: 0,
            fg_set: 0,
            bg_set: 0,
            physics_set: false,
            physics: Physics::default(),
            rest: Vec::new(),
        };
        assert_eq!(l.custom(), vec![0; crate::savegame::CUSTOM.len()]);
        let t: Vec<u8> = (0..crate::savegame::CUSTOM.len()).map(|i| i as u8).collect();
        l.set_custom(&t);
        assert_eq!(l.custom(), t);
    }
}
