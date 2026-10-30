//! The track map: which 32-px tiles of the level Harry has come near, for
//! the map screen (`ShowMap`). 128 × 128 bytes at 0x10072730, indexed
//! `[h * 128 + v]`; a copier save keeps it.

/// Tiles per side.
pub const SIZE: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackMap {
    pub cells: Vec<u8>,
    /// The tile (h, v) of Harry's last update (`sRam1007272e` /
    /// `sRam1007272c`; never reset, 0 at launch).
    pub last: (i16, i16),
}

impl Default for TrackMap {
    fn default() -> Self {
        TrackMap { cells: vec![0; SIZE * SIZE], last: (0, 0) }
    }
}

impl TrackMap {
    /// `InitTrackMap`: nothing seen.
    pub fn clear(&mut self) {
        self.cells.fill(0);
    }

    /// `SetTrackMap(h, v, value)`, outside the map ignored.
    pub fn set(&mut self, h: i16, v: i16, value: u8) {
        if (0..SIZE as i16).contains(&h) && (0..SIZE as i16).contains(&v) {
            self.cells[h as usize * SIZE + v as usize] = value;
        }
    }

    /// `UpdateTrackMap`, the last thing of a frame: when the tile under
    /// Harry's centre (his position + 32) changed, the 14 × 14 tiles from
    /// 7 before to 6 after it are seen.
    pub fn update(&mut self, player_pos: (i16, i16)) {
        let h = ((i32::from(player_pos.0) + 32) >> 5) as i16;
        let v = ((i32::from(player_pos.1) + 32) >> 5) as i16;
        if self.last == (h, v) {
            return;
        }
        for x in h.wrapping_sub(7)..h.wrapping_add(7) {
            for y in v.wrapping_sub(7)..v.wrapping_add(7) {
                self.set(x, y, 1);
            }
        }
        self.last = (h, v);
    }
}
