//! Reading the original files (re/work/installed/*.bin, MacBinary). Without
//! them the tests are skipped.

use std::path::PathBuf;

use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn installed(name: &str) -> Option<Resources> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../re/work/installed").join(name);
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("skipped: {} not found", path.display());
        return None;
    };
    let mb = MacBinary::parse(&bytes).expect("MacBinary");
    Some(Resources::parse(&mb.rsrc).expect("resource fork"))
}

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
    assert_eq!(l1.start, (2082, 2087));
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
