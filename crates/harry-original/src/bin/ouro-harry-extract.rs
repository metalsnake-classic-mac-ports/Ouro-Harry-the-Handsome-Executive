//! Make the data pack from the original files: `ouro-harry-extract
//! <folder with the installed game> [pack.zip]`. The folder holds the
//! game's files as MacBinary (`Harry 1.0.1.bin`, `Harry Graphics.bin`,
//! `Harry Titles.bin`, `Harry Sounds.bin`, `Episode 1.bin`).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use harry_core::mask::Mask;
use harry_data::{Face, GameData, Image, MaskImages, TileSet, clut8};
use harry_original::resfork::{MacBinary, Resources};
use harry_original::{hlvl, hrsp, masks, pict, snd};

fn open(dir: &Path, name: &str) -> Result<Resources, String> {
    let bytes = std::fs::read(dir.join(name)).map_err(|e| format!("{name}: {e}"))?;
    let bin = MacBinary::parse(&bytes).map_err(|e| format!("{name}: {e}"))?;
    Resources::parse(&bin.rsrc).map_err(|e| format!("{name}: {e}"))
}

/// An 8-bit PICT as screen palette indices (its own colour table mapped
/// exactly - every colour of the game's pictures is in clut 8).
fn picture(files: &[&Resources], id: i16) -> Result<pict::Picture, String> {
    let data = files.iter().find_map(|r| r.get(b"PICT", id).ok()).ok_or(format!("PICT {id} missing"))?;
    pict::decode(data).map_err(|e| format!("PICT {id}: {e}"))
}

fn indexed(p: &pict::Picture, clut: &HashMap<[u8; 3], u8>, mask: Option<&pict::Picture>) -> Result<Image, String> {
    let pal = p.palette.as_ref().ok_or("not an 8-bit picture")?;
    let pixels = p
        .pixels
        .iter()
        .enumerate()
        .map(|(n, &i)| {
            if mask.is_some_and(|m| m.pixels[n] == 0) {
                return Ok(None);
            }
            clut.get(&pal[i as usize]).map(|&c| Some(c)).ok_or(format!("colour {:?} is not in clut 8", pal[i as usize]))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Image { width: p.width, height: p.height, pixels })
}

fn run(dir: &Path, out: &Path) -> Result<(), String> {
    let app = open(dir, "Harry 1.0.1.bin")?;
    let gfx = open(dir, "Harry Graphics.bin")?;
    let ep = open(dir, "Episode 1.bin")?;
    let titles = open(dir, "Harry Titles.bin")?;
    let sound_file = open(dir, "Harry Sounds.bin")?;
    let clut: HashMap<[u8; 3], u8> = clut8().iter().enumerate().map(|(i, c)| (*c, i as u8)).collect();
    let mut faces = BTreeMap::new();
    for id in gfx.ids(b"HrSp") {
        let f = hrsp::decode(gfx.get(b"HrSp", id).map_err(|e| e.to_string())?).map_err(|e| format!("HrSp {id}: {e}"))?;
        faces.insert(id, Face { bounds: f.bounds, rect2: f.rect2, image: Image { width: f.width, height: f.height, pixels: f.pixels } });
    }
    let mut levels = Vec::new();
    for id in 1..=15 {
        levels.push(hlvl::read(ep.get(b"Hlvl", id).map_err(|e| e.to_string())?).map_err(|e| format!("Hlvl {id}: {e}"))?);
    }
    let mut tile_sets = BTreeMap::new();
    for l in &levels {
        let (fg, bg) = GameData::tile_set_ids(l);
        if tile_sets.contains_key(&(fg, bg)) {
            continue;
        }
        let files = [&ep, &gfx];
        let mask = picture(&files, fg + 1)?;
        let foreground = indexed(&picture(&files, fg)?, &clut, Some(&mask))?;
        let background = indexed(&picture(&files, bg)?, &clut, None)?;
        tile_sets.insert((fg, bg), TileSet { foreground, background });
    }
    let mut cicns = BTreeMap::new();
    for id in [290, 299, 571, 572, 573, 574, 598, 599, 1299] {
        cicns.insert(id, masks::from_cicn(&gfx, id).map_err(|e| format!("cicn {id}: {e}"))?);
    }
    let walls: Vec<Mask> = masks::walls(&app).map_err(|e| e.to_string())?;
    let mut pictures = BTreeMap::new();
    for id in harry_data::PICTURES {
        pictures.insert(id, indexed(&picture(&[&titles], id)?, &clut, None)?);
    }
    // the icons through the screen's inverse table, as QuickDraw draws them
    let itable = harry_core::tint::inverse_table(&clut8());
    let mut icons = BTreeMap::new();
    for id in harry_data::ICONS {
        let c = harry_original::cicn::decode(gfx.get(b"cicn", id).map_err(|e| format!("cicn {id}: {e}"))?);
        let pixels = c
            .rgb
            .iter()
            .zip(&c.mask)
            .map(|(rgb, &m)| {
                m.then(|| clut.get(rgb).copied().unwrap_or_else(|| harry_core::tint::color2index(&itable, rgb.map(|v| u16::from(v) * 0x101))))
            })
            .collect();
        icons.insert(id, Image { width: c.width, height: c.height, pixels });
    }
    let world = ep.get(b"Hwld", 0).map_err(|e| format!("Hwld 0: {e}"))?;
    let mut texts = BTreeMap::new();
    for n in 0..(world.len().saturating_sub(0x3c9) / 256) {
        let o = 0x3c9 + n * 256;
        let len = world[o] as usize;
        if len > 0 {
            texts.insert(n as i16, world[o + 1..o + 1 + len].to_vec());
        }
    }
    let words = harry_original::words::words(&app).map_err(|e| format!("word lists: {e}"))?;
    let sound_set = |r: &Resources| -> BTreeMap<i16, harry_data::Sound> {
        r.ids(b"snd ")
            .into_iter()
            .filter_map(|id| {
                let s = r.get(b"snd ", id).ok().and_then(snd::decode);
                if s.is_none() {
                    eprintln!("snd {id}: not a sampled sound, left out");
                }
                s.map(|s| (id, harry_data::Sound { rate: s.rate, samples: s.samples }))
            })
            .collect()
    };
    let (sounds, world_sounds) = (sound_set(&sound_file), sound_set(&ep));
    let data =
        GameData { faces, tile_sets, masks: MaskImages { cicns, walls }, pictures, icons, levels, texts, words, sounds, world_sounds };
    let bytes = harry_data::pack::write(&data).map_err(|e| e.to_string())?;
    std::fs::write(out, &bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    println!(
        "{}: {} faces, {} tile sets, {} levels, {} texts, {} sounds, {} KB",
        out.display(),
        data.faces.len(),
        data.tile_sets.len(),
        data.levels.len(),
        data.texts.len(),
        data.sounds.len() + data.world_sounds.len(),
        bytes.len() / 1024
    );
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(dir) = args.get(1) else {
        eprintln!("usage: ouro-harry-extract <folder with the installed game> [pack.zip]");
        return ExitCode::FAILURE;
    };
    let out = args.get(2).map_or_else(|| PathBuf::from("harry-data.zip"), PathBuf::from);
    match run(Path::new(dir), &out) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ouro-harry-extract: {e}");
            ExitCode::FAILURE
        }
    }
}
