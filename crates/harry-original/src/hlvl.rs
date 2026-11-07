//! The original's level resource `Hlvl` (76 168 bytes, see NOTES.md
//! "Level"). `read` decodes the known fields and keeps all other bytes, so
//! `write(read(x)) == x`.

use harry_core::level::{Level, LevelSprite, SIZE, SPRITES};
use harry_core::physics::Physics;

use crate::bytes::be16s;

/// Size of an `Hlvl` resource.
pub const LEN: usize = 76_168;

const FG: usize = 0x8000;
const SPRITE_LIST: usize = 0x10000;
const NAME: usize = 0x114c0;
const NUMBER: usize = 0x12032;
const OWN_SOUNDS: usize = 0x12034;
const INFO: usize = 0x12074;
const BG_KINDS: usize = 0x12094;
const FG_KINDS: usize = 0x120c4;
const START_V: usize = 0x120f4;
const START_H: usize = 0x120f6;
const START_DIR: usize = 0x120f8;
const MUSIC: usize = 0x120fa;
const FG_SET: usize = 0x120fc;
const BG_SET: usize = 0x120fe;
const PHYSICS_SET: usize = 0x12100;
const PHYSICS: usize = 0x12102;

/// Read a word and clear it in `rest` (it is decoded).
fn take(d: &[u8], rest: &mut [u8], o: usize) -> i16 {
    rest[o..o + 2].fill(0);
    be16s(d, o)
}

fn w16(d: &mut [u8], o: usize, v: i16) {
    d[o..o + 2].copy_from_slice(&v.to_be_bytes());
}

#[derive(Debug, thiserror::Error)]
#[error("Hlvl has {0} bytes, expected {LEN}")]
pub struct BadSize(usize);

pub fn read(d: &[u8]) -> Result<Level, BadSize> {
    if d.len() != LEN {
        return Err(BadSize(d.len()));
    }
    let mut rest = d.to_vec();
    let bg = (0..SIZE * SIZE).map(|i| take(d, &mut rest, i * 2) as u16).collect();
    let fg = (0..SIZE * SIZE).map(|i| take(d, &mut rest, FG + i * 2) as u16).collect();
    let mut sprites = Vec::with_capacity(SPRITES);
    for k in 0..SPRITES {
        let o = SPRITE_LIST + k * 16;
        let present = d[o] != 0;
        let flags = d[o + 1];
        let s = LevelSprite {
            present,
            flags,
            id: be16s(d, o + 2),
            info: [be16s(d, o + 4), be16s(d, o + 6), be16s(d, o + 8), be16s(d, o + 10)],
            v: be16s(d, o + 12),
            h: be16s(d, o + 14),
        };
        // keep the raw entry unless it is exactly what we decode
        if d[o] <= 1 {
            rest[o..o + 16].fill(0);
        }
        sprites.push(s);
    }
    let name_len = d[NAME] as usize;
    let name = crate::text::mac_roman(&d[NAME + 1..NAME + 1 + name_len]);
    rest[NAME..NAME + 1 + name_len].fill(0);
    let number = take(d, &mut rest, NUMBER);
    let own_sounds = d[OWN_SOUNDS] == 1;
    if d[OWN_SOUNDS] <= 1 {
        rest[OWN_SOUNDS] = 0;
    }
    let mut info = [0; 16];
    for (i, v) in info.iter_mut().enumerate() {
        *v = take(d, &mut rest, INFO + i * 2);
    }
    let mut bg_kinds = [0u8; 96];
    for (i, k) in bg_kinds.iter_mut().enumerate() {
        *k = (d[BG_KINDS + i / 2] >> if i % 2 == 0 { 4 } else { 0 }) & 15;
    }
    rest[BG_KINDS..BG_KINDS + 48].fill(0);
    let mut fg_kinds = [0i16; 24];
    for (i, k) in fg_kinds.iter_mut().enumerate() {
        *k = i16::from(d[FG_KINDS + i]) - 1;
    }
    rest[FG_KINDS..FG_KINDS + 24].fill(0);
    let start = (take(d, &mut rest, START_H), take(d, &mut rest, START_V));
    let start_dir = take(d, &mut rest, START_DIR);
    let music = take(d, &mut rest, MUSIC);
    let fg_set = take(d, &mut rest, FG_SET);
    let bg_set = take(d, &mut rest, BG_SET);
    let flag = take(d, &mut rest, PHYSICS_SET);
    let physics = Physics {
        scoot: take(d, &mut rest, PHYSICS),
        kick: take(d, &mut rest, PHYSICS + 2),
        friction: take(d, &mut rest, PHYSICS + 4),
        slippery: take(d, &mut rest, PHYSICS + 6),
        bounce: take(d, &mut rest, PHYSICS + 8),
    };
    // a flag other than 0/1 stays in `rest`
    if flag > 1 {
        w16(&mut rest, PHYSICS_SET, flag);
    }
    Ok(Level {
        name,
        number,
        own_sounds,
        bg,
        fg,
        sprites,
        info,
        bg_kinds,
        fg_kinds,
        start,
        start_dir,
        music,
        fg_set,
        bg_set,
        physics_set: flag != 0,
        physics,
        rest,
    })
}

pub fn write(l: &Level) -> Vec<u8> {
    let mut d = l.rest.clone();
    d.resize(LEN, 0);
    for (i, &t) in l.bg.iter().enumerate() {
        w16(&mut d, i * 2, t as i16);
    }
    for (i, &t) in l.fg.iter().enumerate() {
        w16(&mut d, FG + i * 2, t as i16);
    }
    for (k, s) in l.sprites.iter().enumerate() {
        let o = SPRITE_LIST + k * 16;
        if d[o] > 1 {
            continue; // kept raw
        }
        d[o] = s.present as u8;
        d[o + 1] = s.flags;
        w16(&mut d, o + 2, s.id);
        for (i, &v) in s.info.iter().enumerate() {
            w16(&mut d, o + 4 + i * 2, v);
        }
        w16(&mut d, o + 12, s.v);
        w16(&mut d, o + 14, s.h);
    }
    let name = crate::text::to_mac_roman(&l.name);
    d[NAME] = name.len() as u8;
    d[NAME + 1..NAME + 1 + name.len()].copy_from_slice(&name);
    w16(&mut d, NUMBER, l.number);
    if d[OWN_SOUNDS] == 0 {
        d[OWN_SOUNDS] = l.own_sounds as u8;
    }
    for (i, &v) in l.info.iter().enumerate() {
        w16(&mut d, INFO + i * 2, v);
    }
    for (i, &k) in l.bg_kinds.iter().enumerate() {
        d[BG_KINDS + i / 2] |= k << if i % 2 == 0 { 4 } else { 0 };
    }
    for (i, &k) in l.fg_kinds.iter().enumerate() {
        d[FG_KINDS + i] = (k + 1) as u8;
    }
    w16(&mut d, START_H, l.start.0);
    w16(&mut d, START_V, l.start.1);
    w16(&mut d, START_DIR, l.start_dir);
    w16(&mut d, MUSIC, l.music);
    w16(&mut d, FG_SET, l.fg_set);
    w16(&mut d, BG_SET, l.bg_set);
    if d[PHYSICS_SET] == 0 && d[PHYSICS_SET + 1] == 0 {
        w16(&mut d, PHYSICS_SET, l.physics_set as i16);
    }
    let p = &l.physics;
    for (i, v) in [p.scoot, p.kick, p.friction, p.slippery, p.bounce].into_iter().enumerate() {
        w16(&mut d, PHYSICS + i * 2, v);
    }
    d
}
