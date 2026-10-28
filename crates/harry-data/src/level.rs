//! Levels as text, in the data pack (`levels/NN.txt`):
//!
//! ```text
//! name Lobby
//! number 1
//! own_sounds 0
//! info <16 numbers>
//! bg_kinds <96 numbers>
//! fg_kinds <24 numbers>
//! start <h> <v> <dir>
//! music <id>
//! tiles <foreground PICT> <background PICT>
//! physics <set> <scoot> <kick> <friction> <slippery> <bounce>
//! bg
//! <128 rows (y) of 128 hex tiles (x)>
//! fg
//! <128 rows>
//! sprite <index> <present> <flags> <id> <info 0-3> <v> <h>
//! ```
//!
//! Lines starting with `#` are comments. Only used sprite entries are
//! listed. The bytes of the original level that are not decoded (empty
//! custom sprite tables in Episode 1) are not kept.

use harry_core::Level;
use harry_core::level::{LevelSprite, SIZE, SPRITES};
use harry_core::physics::Physics;

use crate::{Result, corrupt};

fn join<T: ToString>(v: impl IntoIterator<Item = T>) -> String {
    v.into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
}

/// A level as text.
pub fn write(l: &Level) -> String {
    let mut s = String::new();
    s.push_str("# Harry the Handsome Executive level (ouro-harry data pack)\n");
    s.push_str(&format!("name {}\n", l.name));
    s.push_str(&format!("number {}\n", l.number));
    s.push_str(&format!("own_sounds {}\n", u8::from(l.own_sounds)));
    s.push_str(&format!("info {}\n", join(l.info)));
    s.push_str(&format!("bg_kinds {}\n", join(l.bg_kinds)));
    s.push_str(&format!("fg_kinds {}\n", join(l.fg_kinds)));
    s.push_str(&format!("start {} {} {}\n", l.start.0, l.start.1, l.start_dir));
    s.push_str(&format!("music {}\n", l.music));
    s.push_str(&format!("tiles {} {}\n", l.fg_set, l.bg_set));
    let p = l.physics;
    s.push_str(&format!("physics {} {} {} {} {} {}\n", u8::from(l.physics_set), p.scoot, p.kick, p.friction, p.slippery, p.bounce));
    for (name, grid) in [("bg", &l.bg), ("fg", &l.fg)] {
        s.push_str(name);
        s.push('\n');
        for y in 0..SIZE {
            let row: Vec<String> = (0..SIZE).map(|x| format!("{:04x}", grid[x * SIZE + y])).collect();
            s.push_str(&row.join(" "));
            s.push('\n');
        }
    }
    for (i, e) in l.sprites.iter().enumerate() {
        if *e != LevelSprite::default() {
            s.push_str(&format!(
                "sprite {i} {} {} {} {} {} {}\n",
                u8::from(e.present),
                e.flags,
                e.id,
                join(e.info),
                e.v,
                e.h
            ));
        }
    }
    s
}

fn nums<T: std::str::FromStr>(words: &[&str], n: usize, what: &str) -> Result<Vec<T>> {
    if words.len() != n {
        return Err(corrupt(format!("level: {what} needs {n} values")));
    }
    words.iter().map(|w| w.parse().map_err(|_| corrupt(format!("level: bad {what} value {w}")))).collect()
}

/// A level from its text.
pub fn parse(text: &str) -> Result<Level> {
    let mut l = Level {
        name: String::new(),
        number: 0,
        own_sounds: false,
        bg: vec![0; SIZE * SIZE],
        fg: vec![0xffff; SIZE * SIZE],
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
    let mut lines = text.lines().filter(|s| !s.starts_with('#'));
    while let Some(line) = lines.next() {
        let (key, rest) = line.split_once(' ').unwrap_or((line, ""));
        let w: Vec<&str> = rest.split_whitespace().collect();
        match key {
            "name" => l.name = rest.to_string(),
            "number" => l.number = nums(&w, 1, key)?[0],
            "own_sounds" => l.own_sounds = nums::<u8>(&w, 1, key)?[0] != 0,
            "info" => l.info.copy_from_slice(&nums(&w, 16, key)?),
            "bg_kinds" => l.bg_kinds.copy_from_slice(&nums(&w, 96, key)?),
            "fg_kinds" => l.fg_kinds.copy_from_slice(&nums(&w, 24, key)?),
            "start" => {
                let v: Vec<i16> = nums(&w, 3, key)?;
                (l.start, l.start_dir) = ((v[0], v[1]), v[2]);
            }
            "music" => l.music = nums(&w, 1, key)?[0],
            "tiles" => {
                let v: Vec<i16> = nums(&w, 2, key)?;
                (l.fg_set, l.bg_set) = (v[0], v[1]);
            }
            "physics" => {
                let v: Vec<i16> = nums(&w, 6, key)?;
                l.physics_set = v[0] != 0;
                l.physics = Physics { scoot: v[1], kick: v[2], friction: v[3], slippery: v[4], bounce: v[5] };
            }
            "bg" | "fg" => {
                let grid = if key == "bg" { &mut l.bg } else { &mut l.fg };
                for y in 0..SIZE {
                    let row = lines.next().ok_or_else(|| corrupt("level: grid too short"))?;
                    let cells: Vec<&str> = row.split_whitespace().collect();
                    if cells.len() != SIZE {
                        return Err(corrupt("level: grid row needs 128 tiles"));
                    }
                    for (x, c) in cells.iter().enumerate() {
                        grid[x * SIZE + y] = u16::from_str_radix(c, 16).map_err(|_| corrupt(format!("level: bad tile {c}")))?;
                    }
                }
            }
            "sprite" => {
                let v: Vec<i16> = nums(&w, 10, key)?;
                let i = usize::try_from(v[0]).ok().filter(|&i| i < SPRITES).ok_or_else(|| corrupt("level: bad sprite index"))?;
                l.sprites[i] = LevelSprite {
                    present: v[1] != 0,
                    flags: v[2] as u8,
                    id: v[3],
                    info: [v[4], v[5], v[6], v[7]],
                    v: v[8],
                    h: v[9],
                };
            }
            "" => {}
            _ => return Err(corrupt(format!("level: unknown line {key}"))),
        }
    }
    Ok(l)
}
