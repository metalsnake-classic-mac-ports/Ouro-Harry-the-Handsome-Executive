//! Sprite tints: each face has a draw proc that calls `BlitEncFace(face,
//! …, mode, arg)`. The mode picks a colour table: 0 none, 1 a tint
//! (`_DAT_10065d00`, 16 tables), 2 translucent (the pixel below lightened
//! by the face's grey, `_DAT_10065d10` by `_DAT_10065d14`), 3 reddening
//! (`_DAT_10065d0c`, 8 steps), 4 heating (`_DAT_10065d08`, 16 steps); mode
//! 1 with arg −1 is mode 2. `arg` is the table's index, plus the sprite's
//! scale << 5 when it is scaled (`arg` ≥ 0x20: drawn scaled; untinted when
//! the index part is 0).
//!
//! The tables are built at launch (`BuildTintTable`, `BuildTranslucTable`,
//! `BuildPosterizationTable`, `BuildReddenTable`) from the screen's colour
//! table: an RGB per entry, mapped back with `Color2Index`.

use crate::boss::BossState;
use crate::frame::PlayerGlobals;
use crate::grate::FENCE;
use crate::level::Level;
use crate::random::Random;
use crate::sprite::{Face, Sprite};

/// One call of `BlitEncFace`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blit {
    pub face: Face,
    pub mode: i16,
    pub arg: i16,
}

/// The draw proc a face was given at launch (`BlitEncFaceProc` for the
/// faces no init routine changes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proc {
    Plain,
    NoScale,
    UserTintable,
    Harry,
    Blob,
    Button,
    Vending,
    Copier,
    Fan,
    Scientist,
    Dart,
    Door,
    Effect,
    Ghost,
    Grate,
    Weapon,
    Mail,
    Message,
    Robot,
    Reversor,
    Turret,
    Boss,
}

/// Which draw proc face `face` has (`InitFaces` and the sprites' init
/// routines).
pub fn proc_of(face: Face) -> Proc {
    use Proc::*;
    match face {
        1101..=1244 => Harry,
        1301..=1302 | 1501..=1507 | 1801..=1836 | 2001 | 3601..=3605 | 3607..=3609 | 3651..=3652 | 3661..=3668
        | 3901..=3904 | 4101..=4104 | 6034..=6037 | 6041..=6044 | 6051..=6054 => UserTintable,
        1401..=1424 => Blob,
        1741..=1745 => Button,
        1902..=1903 => Vending,
        1906 => Copier,
        1921..=1924 => Fan,
        2101..=2111 | 2302..=2308 | 4301..=4332 => Scientist,
        2501..=2528 => Dart,
        2601..=2636 => Door,
        2701..=2713 | 2801..=2832 | 2901..=2932 => Effect,
        3001..=3004 | 3021..=3027 | 3041..=3048 | 3061..=3068 | 3081..=3088 | 3101..=3108 | 3121..=3128 | 3141
        | 4001..=4007 | 4011..=4017 | 4401..=4408 | 4451..=4458 | 4801 | 4981..=4988 | 5001..=5008 => NoScale,
        3301..=3336 => Ghost,
        3401..=3404 => Grate,
        3501..=3536 | 3551..=3586 => Weapon,
        3701..=3704 => Mail,
        3801..=3805 => Message,
        4201..=4236 => Robot,
        4601..=4632 => Turret,
        4701..=4772 => Reversor,
        4902..=4937 => Boss,
        _ => Plain,
    }
}

/// What the draw procs read besides the sprite.
pub struct DrawWorld<'a> {
    pub level: &'a Level,
    pub g: &'a mut PlayerGlobals,
    pub rng: &'a mut Random,
    pub boss: &'a BossState,
    /// The drawn gun (`PTR_DAT_100660dc`).
    pub gun: u8,
    /// Harry's scale (`*_DAT_10065980` +0x52), for his gun.
    pub harry_scale: i16,
}

/// A prop's draw (the proc without a sprite): the level's tint for the
/// tintable faces (all props of Episode 1), else none.
pub fn prop_blit(face: Face, level: &Level) -> Blit {
    let tint = level.info[0];
    let (mode, arg) = if tint != 0 && proc_of(face) == Proc::UserTintable { (1, tint) } else { (0, 0) };
    Blit { face, mode, arg }
}

/// The fire extinguisher (Harry heats up while it sprays).
const EXTINGUISHER: i16 = 11999;

fn scaled(scale: i16) -> i32 {
    if scale != 0x100 { i32::from(scale) << 5 } else { 0 }
}

/// A level sprite entry's info `k` as a tint: none (`None`) for no entry
/// or 0, else 0 outside −1 … 15.
fn entry_tint(level: &Level, index: i16, k: usize) -> Option<i32> {
    let e = level.sprites.get(usize::try_from(index).ok()?)?;
    match e.info[k] {
        0 => None,
        t @ -1..=15 => Some(i32::from(t)),
        _ => Some(0),
    }
}

/// One call of the face's draw proc: the blit it makes (none for a few
/// kinds the proc leaves out) and what it changes. A hit robot, big robot,
/// turret, reversor or blob (layer 10) goes back to 7, a hit grate to 9;
/// Harry's face sets +0x8a while his comfort falls, he heats or is hit
/// (kind 10), the blob's when hit, the boss's while it is hurt, the vending
/// machine's when it is empty and drawn before; the vending machine sets
/// +0x78; the B.A.D.A.S.S. gun puts a fire hold over 66 back to 61; on
/// caffeine Harry, and the boss in phase 3, draw a random tint.
pub fn draw_proc(sp: &mut Sprite, w: &mut DrawWorld) -> Option<Blit> {
    let tint = i32::from(w.level.info[0]);
    let s = scaled(sp.scale[0]);
    // the level's tint or none
    let level_tint = |s: i32| if tint == 0 { (0, s) } else { (1, s + tint) };
    let (mode, arg) = match proc_of(sp.face) {
        Proc::Plain => (0, if sp.scale[0] == 0x100 && sp.scale[1] == 0x100 { 0 } else { s }),
        Proc::NoScale => level_tint(0),
        Proc::Copier => {
            if sp.kind == 9200 {
                (0, 0)
            } else {
                (1, 6)
            }
        }
        Proc::Fan => (0, 0),
        Proc::UserTintable => {
            if tint != 0 {
                (1, s + tint)
            } else {
                match entry_tint(w.level, sp.level_index, 0) {
                    Some(t) => (1, s + t),
                    None => (0, s),
                }
            }
        }
        Proc::Button | Proc::Mail => {
            if tint != 0 {
                (1, i32::from(w.level.info[3]))
            } else {
                entry_tint(w.level, sp.level_index, 3).map_or((0, 0), |t| (1, t))
            }
        }
        Proc::Message => {
            if tint != 0 {
                (1, tint)
            } else {
                entry_tint(w.level, sp.level_index, 2).map_or((0, 0), |t| (1, t))
            }
        }
        Proc::Harry => {
            let g = &mut *w.g;
            let item = usize::try_from(g.cur_slot).ok().and_then(|i| g.inventory.get(i)).map_or(-1, |e| e.0);
            if sp.kind == 10 {
                sp.b8a = true;
                (2, s)
            } else if g.caffeine >= 1 {
                (1, s + i32::from(w.rng.below(10)) + 1)
            } else if g.fire_hold >= 1 && item == EXTINGUISHER {
                sp.b8a = true;
                let mut i = g.fire_hold >> 1;
                if i > 15 {
                    i = [15, 14, 13, 12, 13, 14][(i % 6) as usize];
                }
                (4, s + i32::from(i))
            } else if g.comfort < g.comfort_shown {
                sp.b8a = true;
                let d = (i32::from(g.comfort_shown) - i32::from(g.comfort)).min(8);
                (3, s + d - 1)
            } else {
                level_tint(s)
            }
        }
        Proc::Weapon => {
            let s = scaled(w.harry_scale);
            match w.gun {
                0 => (0, s),
                2 => (1, s + 4),
                _ => match w.g.fire_hold {
                    7..=0x42 if w.g.fire_hold % 2 == 0 => (1, s + 8),
                    ..=0x42 => (1, s + 12),
                    _ => {
                        w.g.fire_hold = 0x3d;
                        (1, s + 12)
                    }
                },
            }
        }
        Proc::Blob => {
            if sp.layer == 10 {
                sp.layer = 7;
                sp.b8a = true;
                (1, s + 8)
            } else if tint != 0 {
                (1, s + tint)
            } else {
                match sp.kind {
                    1711 => (1, s + 10),
                    1701 => (0, s),
                    1712..=1799 => (1, s + 1),
                    _ => return None,
                }
            }
        }
        Proc::Robot => {
            if sp.layer == 10 {
                sp.layer = 7;
                (1, s + 4)
            } else if tint != 0 {
                (1, s + tint)
            } else {
                match sp.kind {
                    1811 => (1, s + 2),
                    1801 => (0, s),
                    1851..=1852 => (1, s + 4),
                    1861..=1862 => (1, s + 10),
                    1812..=1899 => (1, s + 1),
                    _ => return None,
                }
            }
        }
        Proc::Reversor => {
            if sp.layer == 10 {
                sp.layer = 7;
                (1, s + 4)
            } else {
                match sp.kind {
                    2021 => (1, s + 4),
                    2011 => (1, s + 1),
                    _ => (1, s),
                }
            }
        }
        Proc::Turret => {
            if sp.layer == 10 {
                sp.layer = 7;
                (1, 4)
            } else if tint != 0 {
                (1, tint)
            } else if sp.kind < 1910 {
                (0, 0)
            } else if sp.kind < 1920 {
                (1, 8)
            } else {
                (1, 12)
            }
        }
        Proc::Scientist => {
            if sp.layer == 10 {
                (1, 1)
            } else if tint != 0 {
                (1, tint)
            } else if (1200..=1300).contains(&sp.kind) {
                (1, 6)
            } else {
                (0, 0)
            }
        }
        Proc::Grate => {
            if sp.layer == 10 && sp.kind < FENCE {
                sp.layer = 9;
                (1, 4)
            } else if sp.kind < FENCE {
                (0, 0)
            } else {
                (1, 1)
            }
        }
        Proc::Ghost => {
            if tint != 0 {
                (1, tint)
            } else if sp.kind == 5001 {
                (0, 0)
            } else {
                (1, 1)
            }
        }
        Proc::Dart => {
            if sp.layer == 10 {
                (1, 1)
            } else {
                level_tint(0)
            }
        }
        Proc::Door => {
            if matches!(sp.kind, 903 | 913 | 923 | 933) {
                if sp.level_index == -1 {
                    (1, 4)
                } else {
                    let e = usize::try_from(sp.level_index).ok().and_then(|i| w.level.sprites.get(i));
                    match e.map_or(0, |e| e.info[0]) {
                        0 => (1, 4),
                        1 => (1, 2),
                        2 => (1, 10),
                        _ => return None,
                    }
                }
            } else {
                level_tint(0)
            }
        }
        Proc::Effect => {
            if tint != 0 {
                if sp.kind == -1 { (2, s) } else { (1, s + tint) }
            } else if sp.kind == 0 || sp.f7c > 0x1067 {
                (0, s)
            } else {
                match sp.kind {
                    5 => (1, s + 4),
                    -1 => (2, s),
                    k => (1, s + i32::from(k)),
                }
            }
        }
        Proc::Vending => {
            let empty = usize::try_from(sp.level_index)
                .ok()
                .and_then(|x| w.level.sprites.get(x))
                .is_some_and(|e| e.info[1] < 1);
            let lit = sp.f78 == 1 && empty;
            sp.f78 = 1;
            if lit {
                sp.b8a = true;
                (1, 6)
            } else {
                (0, 0)
            }
        }
        Proc::Boss => {
            let b = w.boss;
            if sp.f6e < 0 {
                sp.b8a = true;
                let i = if b.change < 0 {
                    7
                } else {
                    let mut t = sp.state;
                    while t > 50 {
                        t -= 50;
                    }
                    ((0.16 * f64::from(50 - i32::from(t))) as i32).clamp(2, 8) - 1
                };
                (3, s + i)
            } else if b.anim == 2 || b.anim == 6 || b.phase != 2 {
                if b.phase == 3 {
                    (1, s + i32::from(w.rng.below(15)))
                } else {
                    level_tint(s)
                }
            } else {
                match b.anim {
                    0 => (1, s + 7),
                    1 | 7 => (1, s + 6),
                    3 | 5 => (1, s + 8),
                    4 => (1, s + 9),
                    _ => return None,
                }
            }
        }
    };
    Some(Blit { face: sp.face, mode, arg: arg as i16 })
}

/// The screen's colour table as `ColorSpec`s: 16-bit components.
fn rgb16(clut: &[[u8; 3]; 256], i: usize) -> [u32; 3] {
    clut[i].map(|c| u32::from(c) * 0x101)
}

/// The colour tables of `BlitEncFace`'s modes, as palette indices.
pub struct Tables {
    /// Mode 1: 16 tints (0 and 15 the identity).
    pub tint: Vec<[u8; 256]>,
    /// Mode 2: by the face pixel's grey (`posterize`), the pixel below.
    pub transluc: Vec<[u8; 256]>,
    pub posterize: [u8; 256],
    /// Mode 3: toward red, 8 steps.
    pub redden: Vec<[u8; 256]>,
    /// Mode 4: toward a pale yellow, 16 steps.
    pub heat: Vec<[u8; 256]>,
}

impl Tables {
    /// Build the tables from the screen's colour table in the original's
    /// order; `color2index` maps an RGB (16-bit components) to an index,
    /// `rand` is `SATRand(5000)` (tint 10 is random colours).
    pub fn build(
        clut: &[[u8; 3]; 256],
        color2index: &mut dyn FnMut([u16; 3]) -> u8,
        rand: &mut dyn FnMut() -> u16,
    ) -> Tables {
        let identity: [u8; 256] = std::array::from_fn(|i| i as u8);
        let avg = |c: [u32; 3]| (c[0] + c[1] + c[2]) / 3;
        let cap = |v: u32| v.min(0xffff) as u16;
        let mut tint = vec![identity; 16];
        // BuildTintTable: tables 1-12 leave entry 255 (black) alone
        for (t, table) in tint.iter_mut().enumerate().take(15).skip(1) {
            let n = if t >= 13 { 256 } else { 255 };
            for (i, e) in table.iter_mut().enumerate().take(n) {
                let c = rgb16(clut, i);
                let a = avg(c);
                let rgb = match t {
                    1 => [cap(a + 10000), 0, 0],
                    2 => [cap(a + 10000), cap(a + 10000), 0],
                    3 => [0, 0, a as u16],
                    4 => [0, a as u16, 0xffff],
                    5 => [cap(a + 16000); 3],
                    6 => c.map(|x| (x >> 1) as u16),
                    7 => c.map(|x| (x >> 2) as u16),
                    8 => c.map(|x| cap((1.5 * f64::from(x)) as u32)),
                    9 => c.map(|x| cap((2.5 * f64::from(x)) as u32)),
                    10 => {
                        let b = rand();
                        let r = rand();
                        [r, a as u16, b]
                    }
                    11 => [a as u16; 3],
                    12 => [0xffff, a as u16, 0],
                    13 => [c[0] as u16, ((c[1] + 0xffff) >> 1) as u16, ((c[2] + 0xffff) >> 1) as u16],
                    _ => [((a + (a >> 1)) >> 1) as u16, (((a >> 1) + (a >> 2)) >> 1) as u16, 0],
                };
                *e = color2index(rgb);
            }
        }
        // BuildTranslucTable: lightened by half of s/16, then grey
        let mut transluc = vec![[0u8; 256]; 16];
        for (s, table) in transluc.iter_mut().enumerate() {
            let half = ((s as u32) << 12) >> 1;
            for (i, e) in table.iter_mut().enumerate() {
                let c = rgb16(clut, i).map(|x| u32::from(cap(x + half)));
                let a = avg(c) as u16;
                *e = color2index([a; 3]);
            }
        }
        let posterize = std::array::from_fn(|i| (avg(rgb16(clut, i)) >> 12).min(15) as u8);
        // BuildReddenTable: toward the grey's red, s + 1 eighths; toward a
        // pale yellow, s + 1 sixteenths; black stays
        let mut redden = vec![[0u8; 256]; 8];
        for (s, table) in redden.iter_mut().enumerate() {
            let s = s as u32;
            for (i, e) in table.iter_mut().enumerate() {
                let c = rgb16(clut, i);
                let t = (avg(c) + 0xffff) >> 1;
                let r = (((s + 1) * t) >> 3) + (((7 - s) * c[0]) >> 3);
                *e = color2index([r as u16, (((7 - s) * c[1]) >> 3) as u16, (((7 - s) * c[2]) >> 3) as u16]);
            }
            table[255] = 255;
        }
        let mut heat = vec![[0u8; 256]; 16];
        for (s, table) in heat.iter_mut().enumerate() {
            let s = s as u32;
            for (i, e) in table.iter_mut().enumerate() {
                let c = rgb16(clut, i);
                let a = avg(c);
                let to = [(a + 0xffff) >> 1, (a + 0xffff) >> 1, (a + 0xffff) >> 2];
                let rgb: [u16; 3] = std::array::from_fn(|k| ((((s + 1) * to[k]) >> 4) + (((15 - s) * c[k]) >> 4)) as u16);
                *e = color2index(rgb);
            }
            table[255] = 255;
        }
        Tables { tint, transluc, posterize, redden, heat }
    }

    /// A face pixel `src` drawn over `dst` in `mode` with table `index`.
    pub fn shade(&self, mode: i16, index: i16, src: u8, dst: u8) -> u8 {
        let get = |t: &[[u8; 256]]| usize::try_from(index).ok().and_then(|i| t.get(i)).map_or(src, |t| t[usize::from(src)]);
        match (mode, index) {
            (2, _) | (1, -1) => self.transluc[usize::from(self.posterize[usize::from(src)])][usize::from(dst)],
            (1, _) => get(&self.tint),
            (3, _) => get(&self.redden),
            (4, _) => get(&self.heat),
            _ => src,
        }
    }
}

/// The screen's 4-bit inverse table (`MakeITable`, resolution 4): for each
/// cell of the 16 × 16 × 16 colour cube the nearest colour of the table.
/// Approximation: the ROM's own construction is not reproduced (ties and
/// its distance may differ).
pub fn inverse_table(clut: &[[u8; 3]; 256]) -> Vec<u8> {
    (0..4096)
        .map(|n| {
            let cell = [n >> 8, (n >> 4) & 15, n & 15].map(|c: i32| c * 0x11);
            (0..256)
                .min_by_key(|&i| {
                    let d: [i32; 3] = std::array::from_fn(|k| i32::from(clut[i][k]) - cell[k]);
                    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
                })
                .unwrap() as u8
        })
        .collect()
}

/// `Color2Index` through the inverse table: the top 4 bits of each
/// component.
pub fn color2index(itable: &[u8], rgb: [u16; 3]) -> u8 {
    let [r, g, b] = rgb.map(|c| usize::from(c >> 12));
    itable[(r << 8) | (g << 4) | b]
}
