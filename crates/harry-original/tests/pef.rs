//! The PEF data section unpacked by `harry_original::pef` equals the one
//! Ghidra unpacked (`re/work/ppc/data.bin`, from `re/ghidra/run.sh`).

use std::path::PathBuf;

use harry_original::pef;
use harry_original::resfork::MacBinary;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_data_section_unpacks_as_ghidra_does() {
    let (Ok(app), Ok(ghidra)) =
        (std::fs::read(root().join("re/work/installed/Harry 1.0.1.bin")), std::fs::read(root().join("re/work/ppc/data.bin")))
    else {
        eprintln!("skipped: the application or data.bin not found");
        return;
    };
    let data = pef::data_section(&MacBinary::parse(&app).unwrap().data).unwrap();
    // data.bin: the start address, then the section with the loader's
    // relocations applied: every word that differs is a pointer into the
    // code (+ 0x10000000) or the data (+ 0x10065390), or an import
    assert_eq!(data.len(), ghidra.len() - 4);
    let word = |d: &[u8], o: usize| u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]);
    let mut relocated = 0;
    for o in (0..data.len() - 3).step_by(4) {
        let (ours, theirs) = (word(&data, o), word(&ghidra[4..], o));
        if ours != theirs {
            let d = theirs.wrapping_sub(ours);
            assert!(d == 0x1000_0000 || d == 0x1006_5390 || (0x1008_20d0..0x1008_2600).contains(&theirs), "word at {o:#x}");
            relocated += 1;
        }
    }
    assert_eq!(relocated, 3003);
    assert_eq!(data[0x3718..0x3718 + 52 * 256 + 256], ghidra[4 + 0x3718..4 + 0x3718 + 52 * 256 + 256]);
    let texts = pef::strings(&data, 0x3818, 52);
    assert_eq!(texts.len(), 52);
    assert_eq!(texts[6], b"Agouti Fur Coat.");
}
