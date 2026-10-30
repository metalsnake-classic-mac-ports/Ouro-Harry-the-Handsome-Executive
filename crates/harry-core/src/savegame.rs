//! Saved games: the record behind `_DAT_10065b60` (0x95d8 bytes), written
//! to a file of type `HrSG` (creator `Hary`) as resource `HrSG` 0. The
//! game keeps one record for the whole session: a save at a level's start
//! (`DoSaveGame`) and a copier save (`DoCopierSaveGame`) each write only
//! their part of it, the rest stays as it was.
//!
//! Layout (big endian): +0 the world's id (Hwld +0x100), +4 copier save,
//! +6 level, +8 score, +0xc comfort, +0xe surface, +0x10 favor, +0x12 the
//! inventory (12 × id, count), +0x42 / +0x44 / +0x46 Harry's position
//! (`gPlayerH`, `gPlayerV`) and direction, +0x48 / +0x4a / +0x4c staple,
//! B.A.D.A.S.S. and shrapnel ammunition, +0x4e the track map (128 × 128),
//! +0x4050 the date (Mac seconds), +0x4058 the level's sprite list (256 ×
//! 16 bytes, positions − 6), +0x5058 / +0x5398 its two custom tables (16
//! × 0x34, 16 × 0x18), +0x5518 the world file's name (Pascal string).

use crate::frame::SLOTS;
use crate::keys::Ammo;
use crate::level::{LevelSprite, SPRITES};
use crate::track;

/// Size of the record.
pub const LEN: usize = 0x95d8;
/// Hwld +0x100 of the default world (Episode 1).
pub const DEFAULT_WORLD: u32 = 0x0b07_cf71;
/// The level's two custom tables (0x11000 and 0x11340 in `Hlvl`).
pub const CUSTOM: std::ops::Range<usize> = 0x11000..0x114c0;

const TRACK: usize = 0x4e;
const DATE: usize = 0x4050;
const ENTRIES: usize = 0x4058;
const CUSTOM_AT: usize = 0x5058;
const NAME: usize = 0x5518;

/// The record, decoded; `rest` keeps every byte so a record read from a
/// file is written back unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveGame {
    pub world: u32,
    /// Made at a copier: position, track map and level come with it.
    pub copier: bool,
    pub level: i16,
    pub score: i32,
    pub comfort: i16,
    pub surface: i16,
    pub favor: i16,
    pub inventory: [(i16, i16); SLOTS],
    /// `gPlayerH`, `gPlayerV` and Harry's direction.
    pub pos: (i16, i16),
    pub dir: i16,
    pub ammo: Ammo,
    pub track: Vec<u8>,
    pub date: u32,
    /// The level's sprite list with the positions as in the level (the
    /// record keeps them 6 smaller).
    pub entries: Vec<LevelSprite>,
    /// The level's custom tables (`CUSTOM` of the `Hlvl`).
    pub custom: Vec<u8>,
    /// The world file's name (Mac Roman).
    pub world_name: Vec<u8>,
    pub rest: Vec<u8>,
}

impl Default for SaveGame {
    /// The record at launch (`NewHandleClear`).
    fn default() -> Self {
        SaveGame::from_bytes(&[0; LEN])
    }
}

fn r16(d: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([d[o], d[o + 1]])
}

fn r32(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn w16(d: &mut [u8], o: usize, v: i16) {
    d[o..o + 2].copy_from_slice(&v.to_be_bytes());
}

impl SaveGame {
    /// Decode a record (shorter ones are padded with zeros).
    pub fn from_bytes(bytes: &[u8]) -> SaveGame {
        let mut d = bytes.to_vec();
        d.resize(LEN, 0);
        let entries = (0..SPRITES)
            .map(|k| {
                let o = ENTRIES + k * 16;
                LevelSprite {
                    present: d[o] != 0,
                    flags: d[o + 1],
                    id: r16(&d, o + 2),
                    info: [r16(&d, o + 4), r16(&d, o + 6), r16(&d, o + 8), r16(&d, o + 10)],
                    v: r16(&d, o + 12).wrapping_add(6),
                    h: r16(&d, o + 14).wrapping_add(6),
                }
            })
            .collect();
        let name_len = usize::from(d[NAME]).min(255);
        SaveGame {
            world: r32(&d, 0),
            copier: d[4] != 0,
            level: r16(&d, 6),
            score: r32(&d, 8) as i32,
            comfort: r16(&d, 0xc),
            surface: r16(&d, 0xe),
            favor: r16(&d, 0x10),
            inventory: std::array::from_fn(|i| (r16(&d, 0x12 + 4 * i), r16(&d, 0x14 + 4 * i))),
            pos: (r16(&d, 0x42), r16(&d, 0x44)),
            dir: r16(&d, 0x46),
            ammo: Ammo { staple: r16(&d, 0x48), badass: r16(&d, 0x4a), shrapnel: r16(&d, 0x4c) },
            track: d[TRACK..TRACK + track::SIZE * track::SIZE].to_vec(),
            date: r32(&d, DATE),
            entries,
            custom: d[CUSTOM_AT..CUSTOM_AT + CUSTOM.len()].to_vec(),
            world_name: d[NAME + 1..NAME + 1 + name_len].to_vec(),
            rest: d,
        }
    }

    /// Encode the record.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut d = self.rest.clone();
        d.resize(LEN, 0);
        d[0..4].copy_from_slice(&self.world.to_be_bytes());
        d[4] = u8::from(self.copier);
        w16(&mut d, 6, self.level);
        d[8..12].copy_from_slice(&self.score.to_be_bytes());
        w16(&mut d, 0xc, self.comfort);
        w16(&mut d, 0xe, self.surface);
        w16(&mut d, 0x10, self.favor);
        for (i, &(id, n)) in self.inventory.iter().enumerate() {
            w16(&mut d, 0x12 + 4 * i, id);
            w16(&mut d, 0x14 + 4 * i, n);
        }
        w16(&mut d, 0x42, self.pos.0);
        w16(&mut d, 0x44, self.pos.1);
        w16(&mut d, 0x46, self.dir);
        w16(&mut d, 0x48, self.ammo.staple);
        w16(&mut d, 0x4a, self.ammo.badass);
        w16(&mut d, 0x4c, self.ammo.shrapnel);
        d[TRACK..TRACK + self.track.len()].copy_from_slice(&self.track);
        d[DATE..DATE + 4].copy_from_slice(&self.date.to_be_bytes());
        for (k, e) in self.entries.iter().enumerate().take(SPRITES) {
            let o = ENTRIES + k * 16;
            d[o] = u8::from(e.present);
            d[o + 1] = e.flags;
            w16(&mut d, o + 2, e.id);
            for (j, &x) in e.info.iter().enumerate() {
                w16(&mut d, o + 4 + 2 * j, x);
            }
            w16(&mut d, o + 12, e.v.wrapping_sub(6));
            w16(&mut d, o + 14, e.h.wrapping_sub(6));
        }
        d[CUSTOM_AT..CUSTOM_AT + self.custom.len()].copy_from_slice(&self.custom);
        let n = self.world_name.len().min(255);
        d[NAME] = n as u8;
        d[NAME + 1..NAME + 1 + n].copy_from_slice(&self.world_name[..n]);
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_round_trips() {
        let mut d = vec![0u8; LEN];
        for (i, b) in d.iter_mut().enumerate() {
            *b = (i * 7 + i / 251) as u8;
        }
        d[NAME] = 9;
        d[4] = 1;
        for k in 0..SPRITES {
            d[ENTRIES + k * 16] = 1;
        }
        assert_eq!(SaveGame::from_bytes(&d).to_bytes(), d);
    }
}
