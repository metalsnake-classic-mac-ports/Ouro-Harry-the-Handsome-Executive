//! Reading the original files (re/work/installed/*.bin, MacBinary). Without
//! them the tests are skipped.

mod common;

use common::{installed, work};
use harry_original::hlvl;

#[test]
fn levels_read_and_write_back_exactly() {
    let Some(res) = installed("Episode 1.bin") else { return };
    let ids = res.ids(b"Hlvl");
    assert_eq!(ids, [-1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    for id in ids {
        let raw = res.get(b"Hlvl", id).unwrap();
        let level = hlvl::read(raw).unwrap();
        assert_eq!(hlvl::write(&level), raw, "level {id}");
    }
    let l1 = hlvl::read(res.get(b"Hlvl", 1).unwrap()).unwrap();
    assert_eq!(l1.name, "Welcome to ScumCo");
    assert_eq!(l1.start, (2087, 2082));
    assert_eq!(l1.music, 601);
    assert!(!l1.physics_set);
    assert_eq!(l1.effective_physics(), harry_core::Physics::default());
    let l10 = hlvl::read(res.get(b"Hlvl", 10).unwrap()).unwrap();
    assert_eq!(l10.name, "It’s Cooler Here");
    assert_eq!((l10.fg_set, l10.bg_set), (137, 139));
    assert!(l10.own_sounds && !l1.own_sounds);
    assert_eq!((l1.number, l10.number), (1, 10));
}

#[test]
fn level_rest_is_mostly_empty() {
    // what is still undecoded: report it (not a failure)
    let Some(res) = installed("Episode 1.bin") else { return };
    for id in res.ids(b"Hlvl") {
        let l = hlvl::read(res.get(b"Hlvl", id).unwrap()).unwrap();
        let used: Vec<usize> = l.rest.iter().enumerate().filter(|(_, b)| **b != 0).map(|(i, _)| i).collect();
        if let (Some(a), Some(b)) = (used.first(), used.last()) {
            eprintln!("level {id}: {} undecoded bytes set, 0x{a:x}..0x{b:x}", used.len());
        }
    }
}

#[test]
fn masks_match_the_recorded_ones() {
    use harry_original::masks;
    let (Some(app), Some(gfx)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin")) else { return };
    let dir = work("diff/masks");
    let text = |name: &str| std::fs::read_to_string(dir.join(format!("{name}.txt"))).ok();
    let as_text = |m: &harry_core::mask::Mask| {
        let mut s = String::new();
        for y in 0..m.height {
            for x in 0..m.width {
                s.push(if m.get(x, y) { '#' } else { '.' });
            }
            s.push('\n');
        }
        s
    };
    let Some(body) = text("body") else { return };
    assert_eq!(as_text(&masks::from_cicn(&gfx, 299).unwrap()), body);
    assert_eq!(as_text(&masks::from_cicn(&gfx, 290).unwrap()), text("obstacle").unwrap());
    for (k, w) in masks::walls(&app).unwrap().iter().enumerate() {
        assert_eq!(as_text(w), text(&format!("wall{k}")).unwrap(), "wall {k}");
    }
}

#[test]
fn all_pictures_decode() {
    use harry_original::pict;
    let mut eight = 0;
    let mut one = 0;
    let mut direct = 0;
    for file in ["Harry 1.0.1.bin", "Episode 1.bin", "Harry Graphics.bin", "Harry Titles.bin"] {
        let Some(res) = installed(file) else { return };
        for id in res.ids(b"PICT") {
            let p = pict::decode(res.get(b"PICT", id).unwrap()).unwrap_or_else(|_| panic!("{file} PICT {id}"));
            if let Some(rgb) = &p.rgb {
                assert_eq!(rgb.len(), p.width * p.height);
                direct += 1;
                continue;
            }
            assert_eq!(p.pixels.len(), p.width * p.height);
            if p.palette.is_some() { eight += 1 } else { one += 1 }
        }
    }
    eprintln!("{eight} 8-bit, {one} 1-bit and {direct} direct-colour pictures");
    assert_eq!(eight + one + direct, 4 + 53 + 4 + 21);
}

#[test]
fn all_faces_decode_like_the_python_decoder() {
    use harry_original::hrsp;
    let Some(gfx) = installed("Harry Graphics.bin") else { return };
    let path = work("diff/hrsp_counts.txt");
    let Ok(counts) = std::fs::read_to_string(path) else { return };
    let mut n = 0;
    for line in counts.lines().filter(|l| !l.starts_with('#')) {
        let v: Vec<i64> = line.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let f = hrsp::decode(gfx.get(b"HrSp", v[0] as i16).unwrap()).unwrap();
        let px: Vec<u8> = f.pixels.iter().flatten().copied().collect();
        assert_eq!(
            [f.width as i64, f.height as i64, px.len() as i64, px.iter().map(|&c| i64::from(c)).sum()],
            [v[1], v[2], v[3], v[4]],
            "HrSp {}",
            v[0]
        );
        n += 1;
    }
    assert_eq!(n, 929);
}

/// The face masks the original's `BuildMaskRegion` makes (recorded in
/// Unicorn by `re/tools/emu_mask.py re/work faces`) - including the
/// mirrored bit order (BUGS.md).
#[test]
fn face_masks_match_the_original() {
    use harry_original::{hrsp, masks};
    let Some(gfx) = installed("Harry Graphics.bin") else { return };
    let path = work("diff/facemasks.txt");
    let Ok(text) = std::fs::read_to_string(path) else { return };
    // the first line is a comment ("# ..."); mask rows may start with '#'
    let mut lines = text.lines().filter(|l| !l.starts_with("# "));
    let mut n = 0;
    while let Some(head) = lines.next() {
        let v: Vec<i32> = head[2..].split_whitespace().map(|x| x.parse().unwrap()).collect();
        let rows: Vec<&str> = (0..v[2]).map(|_| lines.next().unwrap()).collect();
        let face = hrsp::decode(gfx.get(b"HrSp", v[0] as i16).unwrap()).unwrap();
        let mask = masks::from_face(&face);
        for (y, row) in rows.iter().enumerate() {
            let mine: String = (0..v[1]).map(|x| if mask.get(x, y as i32) { '#' } else { '.' }).collect();
            assert_eq!(&mine, row, "HrSp {} row {y}", v[0]);
        }
        n += 1;
    }
    assert_eq!(n, 929);
}
