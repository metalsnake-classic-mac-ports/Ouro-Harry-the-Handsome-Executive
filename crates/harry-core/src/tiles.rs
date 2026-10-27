//! Which tiles a sprite touches (`CheckTileCollisions`) and the shapes the
//! test uses (`InitTileHotRects`, `InitPlayerKickRects`; captured from the
//! original by running them, see NOTES.md "Tiles and collisions").

use crate::bounce::Rect;
use crate::level::{Level, NO_TILE, TILE};

/// Collision rect of a foreground kind within its tile (top, left,
/// bottom, right): 0–3 half tiles (left, top, right, bottom), 4–7 quarters
/// (bottom-left, top-left, top-right, bottom-right), 51–53 shrinking
/// squares, all others the full tile.
pub fn fg_rect(kind: i16) -> Rect {
    match kind {
        0 => [0, 0, 32, 16],
        1 => [0, 0, 16, 32],
        2 => [0, 16, 32, 32],
        3 => [16, 0, 32, 32],
        4 => [16, 0, 32, 16],
        5 => [0, 0, 16, 16],
        6 => [0, 16, 16, 32],
        7 => [16, 16, 32, 32],
        51 => [4, 4, 28, 28],
        52 => [8, 8, 24, 24],
        53 => [12, 12, 20, 20],
        _ => [0, 0, 32, 32],
    }
}

/// Collision rect of a background kind within its tile.
pub fn bg_rect(kind: i16) -> Rect {
    if (3..=5).contains(&kind) { [6, 6, 26, 26] } else { [2, 2, 30, 30] }
}

/// Where Harry's foot is in each direction (relative to his 64 × 64 face),
/// for the power kick - on the side opposite to the direction of travel.
pub const KICK_RECTS: [Rect; 36] = [
    [27, 51, 37, 63], [21, 54, 33, 63], [16, 54, 28, 63], [11, 49, 25, 62], [6, 48, 20, 60], [4, 44, 16, 57],
    [1, 41, 13, 53], [0, 36, 10, 48], [0, 31, 8, 42], [0, 27, 6, 37], [0, 21, 7, 32], [0, 16, 9, 28],
    [1, 10, 11, 23], [2, 6, 14, 19], [6, 4, 18, 15], [10, 1, 22, 13], [15, 0, 27, 9], [21, 0, 32, 8],
    [26, 0, 36, 7], [31, 0, 42, 7], [35, 0, 47, 9], [38, 1, 53, 11], [43, 3, 57, 15], [47, 7, 59, 19],
    [51, 11, 63, 22], [53, 15, 63, 27], [55, 21, 63, 32], [55, 26, 63, 36], [56, 31, 63, 42], [54, 35, 63, 47],
    [51, 39, 62, 53], [48, 43, 61, 56], [44, 46, 57, 60], [40, 51, 52, 62], [36, 52, 48, 63], [31, 55, 42, 63],
];

/// `Rect` offset by (v, h).
pub fn offset(r: Rect, v: i16, h: i16) -> Rect {
    [r[0].wrapping_add(v), r[1].wrapping_add(h), r[2].wrapping_add(v), r[3].wrapping_add(h)]
}

/// QuickDraw `SectRect`: do the rects overlap (non-empty intersection)?
pub fn sect(a: Rect, b: Rect) -> bool {
    a[0].max(b[0]) < a[2].min(b[2]) && a[1].max(b[1]) < a[3].min(b[3])
}

/// A tile a sprite touches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileHit {
    /// The tile's top-left (v, h) in pixels.
    pub tile: (i16, i16),
    pub kind: i16,
    pub foreground: bool,
}

/// `CheckTileCollisions` for one sprite with a tile routine: the 3 × 3
/// tiles around the centre of its face, in the original's order (centre,
/// then the row above left to right, the two sides, the row below); per
/// tile the foreground first, then the background (kind 0 skipped). `at`
/// (v, h) and `hot` are taken once, before any reaction moves the sprite;
/// `face` is the face's bounds (top, left, bottom, right). `hit` is called
/// for each touched tile in order - it may move the sprite.
pub fn touched_tiles(level: &Level, at: (i16, i16), hot: Rect, face: Rect, mut hit: impl FnMut(TileHit)) {
    let me = offset(hot, at.0, at.1);
    let ch = i32::from(at.1) + ((i32::from(face[3]) - i32::from(face[1])) >> 1);
    let cv = i32::from(at.0) + ((i32::from(face[2]) - i32::from(face[0])) >> 1);
    let (x, y) = (i32::from(ch as i16) >> 5, i32::from(cv as i16) >> 5);
    const ORDER: [(i32, i32); 9] = [(0, 0), (-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];
    for (dx, dy) in ORDER {
        let (tx, ty) = (x + dx, y + dy);
        let tile = ((ty * TILE) as i16, (tx * TILE) as i16);
        let fg = level.fg_kind(level.fg_tile(tx, ty));
        let bg = level.bg_kind(level.bg_tile(tx, ty));
        if fg != NO_TILE as i16 && sect(me, offset(fg_rect(fg), tile.0, tile.1)) {
            hit(TileHit { tile, kind: fg, foreground: true });
        }
        if bg != 0 && sect(me, offset(bg_rect(bg), tile.0, tile.1)) {
            hit(TileHit { tile, kind: bg, foreground: false });
        }
    }
}
