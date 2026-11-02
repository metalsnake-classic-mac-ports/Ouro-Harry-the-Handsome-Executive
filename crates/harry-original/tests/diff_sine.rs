//! The ripple's sine table against the original's `BuildSineTable`,
//! recorded in Unicorn by `re/tools/emu_sine.py`.

use std::path::PathBuf;

use harry_core::Level;
use harry_core::ripple::{LENGTH, WIDTH, sine_table};
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_sine_table_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/sine.txt")) else {
        eprintln!("skipped: sine.txt not found (run re/tools/emu_sine.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Episode 1.bin")) else { return };
    let ep = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let level: Level = hlvl::read(ep.get(b"Hlvl", 10).unwrap()).unwrap();
    let (mut bad, mut n) = (0, 0);
    for line in text.lines() {
        let (a, b) = line.split_once(" -> ").unwrap();
        let (len, width) = a.split_once(' ').unwrap();
        let mut l = level.clone();
        l.info[LENGTH] = len.parse().unwrap();
        l.info[WIDTH] = width.parse().unwrap();
        let t = sine_table(&mut l);
        let entries: Vec<String> = t.iter().map(i16::to_string).collect();
        let got = format!("{} {}", l.info[LENGTH], if entries.is_empty() { "-".to_string() } else { entries.join(",") });
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 5 {
                eprintln!("{a}\n  want {b}\n   got {got}");
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
