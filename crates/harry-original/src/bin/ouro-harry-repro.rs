//! Writes copier saves that put Harry where a bug of the original shows
//! (BUGS.md, "How to see it"): in the original, Continue Game opens one
//! and Harry stands at the spot with the meters, ammunition and items the
//! bug needs. Each is written twice: `<name>` as the original's file
//! (resource `HrSG` 0, type `HrSG`, creator `Hary`; macOS only, for an
//! emulator's shared folder) and `<name>.hrsg` (the bare record, for the
//! port: `HARRY_SAVE=<file>` and Continue).
//!
//! `ouro-harry-repro <pack.zip> <folder>`

use std::path::Path;
use std::process::ExitCode;

use harry_core::inventory::{EMPTY, item};
use harry_core::keys::Ammo;
use harry_core::savegame::{DEFAULT_WORLD, SaveGame};

/// One save: the level, where Harry's centre is, his direction (None:
/// the level's start direction), comfort / surface / favor, ammunition
/// (staple, B.A.D.A.S.S., shrapnel) and items (the first is selected).
struct Repro {
    name: &'static str,
    level: i16,
    at: (i16, i16),
    dir: Option<i16>,
    meters: (i16, i16, i16),
    ammo: (i16, i16, i16),
    items: &'static [i16],
}

const fn r(name: &'static str, level: i16, at: (i16, i16)) -> Repro {
    Repro { name, level, at, dir: None, meters: (72, 72, 72), ammo: (0, 0, 0), items: &[] }
}

/// Level 1's start room, level 11's corridor between two holes.
const L1_START: (i16, i16) = (2087, 2082);
const L11_HOLES: (i16, i16) = (1760, 1072);

fn repros() -> Vec<Repro> {
    vec![
        Repro { dir: Some(35), ..r("dir35", 1, L1_START) },
        // the surface meter is drawn inverted: 142 is a 2-point strip
        Repro { meters: (72, 142, 72), ..r("meter-end", 1, L1_START) },
        Repro { ammo: (0, 5, 0), items: &[item::BADASS], ..r("badass-backfire", 1, L1_START) },
        Repro { ammo: (10, 10, 10), items: &[item::MYSTERY_VIAL, item::STAPLE_GUN], ..r("vial-negative", 1, L1_START) },
        Repro { ammo: (99, 0, 0), items: &[item::STAPLE_GUN], ..r("ammo-over-100", 1, L1_START) },
        Repro { ammo: (100, 0, 0), items: &[item::STAPLE_GUN], ..r("shots-walls", 1, L1_START) },
        r("locked-door", 1, (2096, 3120)),
        r("copier-cancel", 6, (3760, 3600)),
        r("turret-shots", 7, (700, 1460)),
        r("homing", 8, (608, 1200)),
        r("hole-scale", 11, L11_HOLES),
        Repro { meters: (19, 72, 72), ..r("death-delay", 11, L11_HOLES) },
        // above the chute down to the magnets (they pull only with ammunition)
        Repro { ammo: (50, 0, 0), items: &[item::STAPLE_GUN], ..r("magnets", 11, (1760, 1296)) },
        r("mg-bolts", 13, (1744, 1500)),
        r("halfpipe-left", 14, (1840, 2224)),
    ]
}

fn save(rp: &Repro, level: &harry_core::Level) -> SaveGame {
    let mut s = SaveGame { world: DEFAULT_WORLD, copier: true, level: rp.level, ..SaveGame::default() };
    (s.comfort, s.surface, s.favor) = rp.meters;
    // gPlayerH / gPlayerV: the top left of Harry's 64 × 64 face
    s.pos = (rp.at.0 - 32, rp.at.1 - 32);
    s.dir = rp.dir.unwrap_or(level.start_dir);
    s.ammo = Ammo { staple: rp.ammo.0, badass: rp.ammo.1, shrapnel: rp.ammo.2 };
    s.inventory = [(EMPTY, 0); harry_core::frame::SLOTS];
    for (slot, &id) in s.inventory.iter_mut().zip(rp.items) {
        *slot = (id, 1);
    }
    s.entries.clone_from(&level.sprites);
    s
}

/// The original's file: the record as resource `HrSG` 0 in the resource
/// fork, type and creator in the Finder info (as the emulators' shared
/// folders on macOS keep them).
#[cfg(target_os = "macos")]
fn write_mac(path: &Path, record: &[u8]) -> Result<(), String> {
    std::fs::write(path, []).map_err(|e| e.to_string())?;
    let fork = harry_original::resfork::single(b"HrSG", 0, record);
    std::fs::write(path.join("..namedfork/rsrc"), fork).map_err(|e| format!("{}: resource fork: {e}", path.display()))?;
    let mut info = [0u8; 32];
    info[..8].copy_from_slice(b"HrSGHary");
    let hex: String = info.iter().map(|b| format!("{b:02x}")).collect();
    let ok = std::process::Command::new("xattr")
        .args(["-wx", "com.apple.FinderInfo", &hex])
        .arg(path)
        .status()
        .map_err(|e| e.to_string())?
        .success();
    if ok { Ok(()) } else { Err(format!("{}: xattr failed", path.display())) }
}

#[cfg(not(target_os = "macos"))]
fn write_mac(_: &Path, _: &[u8]) -> Result<(), String> {
    Ok(())
}

fn run(pack: &Path, out: &Path) -> Result<(), String> {
    let data = harry_data::pack::read(&std::fs::read(pack).map_err(|e| format!("{}: {e}", pack.display()))?).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for rp in repros() {
        let level = data.levels.get(rp.level as usize - 1).ok_or(format!("{}: no level {}", rp.name, rp.level))?;
        let record = save(&rp, level).to_bytes();
        std::fs::write(out.join(format!("{}.hrsg", rp.name)), &record).map_err(|e| e.to_string())?;
        write_mac(&out.join(rp.name), &record)?;
        println!("{:16} level {:2} at {:?}", rp.name, rp.level, rp.at);
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (Some(pack), Some(out)) = (args.get(1), args.get(2)) else {
        eprintln!("usage: ouro-harry-repro <pack.zip> <folder>");
        return ExitCode::FAILURE;
    };
    match run(Path::new(pack), Path::new(out)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ouro-harry-repro: {e}");
            ExitCode::FAILURE
        }
    }
}
