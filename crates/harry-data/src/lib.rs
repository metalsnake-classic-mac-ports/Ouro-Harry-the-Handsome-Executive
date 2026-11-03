//! The game data of Harry the Handsome Executive as the port uses it,
//! and the data pack it is stored in (`pack`). The pack is made from the
//! original files by `ouro-harry-extract` (crate `harry-original`).

pub mod level;
pub mod pack;

use std::collections::BTreeMap;

use harry_core::Level;
use harry_core::bounce::Rect;
use harry_core::mask::Mask;

/// An image of screen palette indices; `None` is transparent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Option<u8>>,
}

impl Image {
    pub fn get(&self, x: usize, y: usize) -> Option<u8> {
        self.pixels[y * self.width + x]
    }
}

/// A sprite face (`HrSp`): its picture and the two rects of its header
/// (`bounds` (top, left, bottom, right) - the game takes a sprite's
/// centre from it - and a second rect used for redrawing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    pub bounds: Rect,
    pub rect2: Rect,
    pub image: Image,
}

/// A tile set: the foreground (8 × 8 tiles of 32 px, transparent where
/// its mask is clear) and the background (8 × 12 tiles).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileSet {
    pub foreground: Image,
    pub background: Image,
}

/// The collision masks the game builds from `cicn`s and PICT 160.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaskImages {
    /// `cicn` masks by id: Harry's body 299, the obstacle 290, ramps
    /// 571–574, barrel 598, plant 599, enemy shots 1299.
    pub cicns: BTreeMap<i16, Mask>,
    /// The 32 wall shapes (PICT 160).
    pub walls: Vec<Mask>,
}

/// Everything the game reads.
#[derive(Clone, Debug)]
pub struct GameData {
    /// Faces by `HrSp` id.
    pub faces: BTreeMap<i16, Face>,
    /// Tile sets by the foreground `PICT` id (the background's id is
    /// stored with it): see `tile_set`.
    pub tile_sets: BTreeMap<(i16, i16), TileSet>,
    pub masks: MaskImages,
    /// Pictures by `PICT` id (`Harry Titles`, `PICTURES`): 129 the game
    /// screen (640 × 480), 130 the main menu, 132 the instructions, 150 the
    /// meters (147 × 44: empty, comfort, surface resistance, corporate
    /// favor, 11 px each), 4803 / 4804 the credits, 4901–4916 the menu's
    /// buttons.
    pub pictures: BTreeMap<i16, Image>,
    /// The world's pictures by `PICT` id (`Episode 1`): the levels' story
    /// pictures (scrolled) and title cards, the win pictures.
    pub world_pictures: BTreeMap<i16, Image>,
    /// The world's fields for its end (`Hwld` 0).
    pub world: WorldInfo,
    /// Colour icons by `cicn` id (`Harry Graphics`), transparent outside
    /// their mask: 10135–10137 the meters' rounded ends (9 × 9).
    pub icons: BTreeMap<i16, Image>,
    /// Episode 1, levels 1–15.
    pub levels: Vec<Level>,
    /// The world's texts (`Hwld` 0 + 0x3c9 + n · 256) by number, Mac
    /// Roman; empty ones are left out.
    pub texts: BTreeMap<i16, Vec<u8>>,
    /// The application's balloon texts (Mac Roman): 0–51 the random ones
    /// of the P-A-R-G-H-Z keys, −1 the B-E-N one (in the PowerPC data
    /// section at 0x3818 + n · 256 and 0x3718).
    pub messages: BTreeMap<i16, Vec<u8>>,
    /// The sentence generators' word lists (`STR#` 6900–6905, 6800–6801).
    pub words: harry_core::words::Words,
    /// Sounds by `snd` id (`Harry Sounds`).
    pub sounds: BTreeMap<i16, Sound>,
    /// The world's own sounds, used instead in levels that bring their own
    /// (`Level::own_sounds`).
    pub world_sounds: BTreeMap<i16, Sound>,
}

/// What the world file (`Hwld` 0) says beyond its levels; 0 = none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldInfo {
    /// +0x144: the music of the scrolled pictures.
    pub scroll_music: i16,
    /// +0x146 / +0x148: the picture scrolled after the last level, the
    /// one shown then (at its own frame on the whole screen).
    pub win_scroll: i16,
    pub win_show: i16,
    /// +0x14a: how long the win picture stays.
    pub win_seconds: i16,
    /// +0x14c: the music after the last level.
    pub win_music: i16,
}

impl WorldInfo {
    /// The fields from the `Hwld` 0 resource.
    pub fn from_hwld(d: &[u8]) -> WorldInfo {
        let f = |o: usize| d.get(o..o + 2).map_or(0, |b| i16::from_be_bytes([b[0], b[1]]));
        WorldInfo { scroll_music: f(0x144), win_scroll: f(0x146), win_show: f(0x148), win_seconds: f(0x14a), win_music: f(0x14c) }
    }

    /// The pictures a world uses: the levels' info #5–#8 (scroll after /
    /// before, show after / before) and the win pictures.
    pub fn picture_ids(&self, levels: &[Level]) -> std::collections::BTreeSet<i16> {
        levels.iter().flat_map(|l| l.info[5..9].to_vec()).chain([self.win_scroll, self.win_show]).filter(|&p| p != 0).collect()
    }
}

/// A sound: 8-bit unsigned mono samples (0x80 silence) at a rate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sound {
    pub rate: u32,
    pub samples: Vec<u8>,
}

/// The pictures the pack holds (`Harry Titles`): the game screen, the main
/// menu, the instructions, the meters, the credits (text, window) and the
/// main menu's buttons pressed (4901–4906) and released (4911–4916).
pub const PICTURES: [i16; 18] =
    [129, 130, 132, 150, 4803, 4804, 4901, 4902, 4903, 4904, 4905, 4906, 4911, 4912, 4913, 4914, 4915, 4916];

/// The colour icons the pack holds.
pub const ICONS: [i16; 3] = [10135, 10136, 10137];

/// The default tile sets (level fields 0): foreground 181, background 180.
pub const DEFAULT_FG: i16 = 181;
pub const DEFAULT_BG: i16 = 180;

impl GameData {
    /// The (foreground, background) set ids a level uses.
    pub fn tile_set_ids(level: &Level) -> (i16, i16) {
        (if level.fg_set == 0 { DEFAULT_FG } else { level.fg_set }, if level.bg_set == 0 { DEFAULT_BG } else { level.bg_set })
    }

    pub fn tile_set(&self, level: &Level) -> Option<&TileSet> {
        self.tile_sets.get(&Self::tile_set_ids(level))
    }

    /// A face's mask as `BuildMaskRegion` makes it.
    pub fn face_mask(&self, id: i16) -> Option<Mask> {
        let f = self.faces.get(&id)?;
        Some(Mask::from_face(f.image.width, f.image.height, &f.image.pixels))
    }
}

/// The Mac's standard 8-bit colour table (`clut` 8), the game's screen
/// palette: 0–214 the 6 × 6 × 6 cube from white, 215–254 red, green, blue
/// and grey ramps (EE … 11), 255 black.
pub fn clut8() -> [[u8; 3]; 256] {
    const RAMP: [u8; 10] = [0xEE, 0xDD, 0xBB, 0xAA, 0x88, 0x77, 0x55, 0x44, 0x22, 0x11];
    let mut p = [[0u8; 3]; 256];
    for (i, c) in p.iter_mut().enumerate().take(215) {
        let s = |n: usize| ((5 - n) * 0x33) as u8;
        *c = [s(i / 36), s((i / 6) % 6), s(i % 6)];
    }
    for (k, &v) in RAMP.iter().enumerate() {
        p[215 + k] = [v, 0, 0];
        p[225 + k] = [0, v, 0];
        p[235 + k] = [0, 0, v];
        p[245 + k] = [v, v, v];
    }
    p
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("data pack: {0}")]
    Corrupt(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn corrupt(s: impl Into<String>) -> Error {
    Error::Corrupt(s.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clut8_has_256_distinct_colours() {
        let p = clut8();
        let set: std::collections::HashSet<_> = p.iter().collect();
        assert_eq!(set.len(), 256);
        assert_eq!(p[0], [0xFF, 0xFF, 0xFF]);
        assert_eq!(p[214], [0, 0, 0x33]);
        assert_eq!(p[255], [0, 0, 0]);
    }
}
