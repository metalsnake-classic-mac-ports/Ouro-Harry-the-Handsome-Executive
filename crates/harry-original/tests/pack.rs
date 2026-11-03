//! The data pack made by `ouro-harry-extract` holds the original's data
//! unchanged (run `cargo run -p harry-original --bin ouro-harry-extract --
//! re/work/installed re/work/harry-data.zip` first).

use std::path::PathBuf;

use harry_original::resfork::{MacBinary, Resources};
use harry_original::{hlvl, hrsp, masks};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn installed(name: &str) -> Option<Resources> {
    let bytes = std::fs::read(root().join("re/work/installed").join(name)).ok()?;
    Some(Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap())
}

#[test]
fn pack_holds_the_original_data() {
    let Ok(bytes) = std::fs::read(root().join("re/work/harry-data.zip")) else {
        eprintln!("skipped: re/work/harry-data.zip not found");
        return;
    };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let data = harry_data::pack::read(&bytes).unwrap();
    for (i, l) in data.levels.iter().enumerate() {
        let mut orig = hlvl::read(ep.get(b"Hlvl", i as i16 + 1).unwrap()).unwrap();
        orig.rest = Vec::new();
        assert_eq!(*l, orig, "level {}", i + 1);
    }
    assert_eq!(data.faces.len(), gfx.ids(b"HrSp").len());
    for (id, f) in &data.faces {
        let o = hrsp::decode(gfx.get(b"HrSp", *id).unwrap()).unwrap();
        assert_eq!((f.bounds, f.rect2, f.image.width, f.image.height), (o.bounds, o.rect2, o.width, o.height));
        assert_eq!(f.image.pixels, o.pixels, "face {id}");
        assert_eq!(data.face_mask(*id).unwrap(), masks::from_face(&o));
    }
    for (id, m) in &data.masks.cicns {
        assert_eq!(*m, masks::from_cicn(&gfx, *id).unwrap(), "cicn {id}");
    }
    assert_eq!(data.masks.walls, masks::walls(&app).unwrap());
    let pal = harry_data::clut8();
    for id in harry_data::ICONS {
        let c = harry_original::cicn::decode(gfx.get(b"cicn", id).unwrap());
        let icon = &data.icons[&id];
        assert_eq!((icon.width, icon.height), (c.width, c.height), "cicn {id}");
        let want: Vec<Option<[u8; 3]>> = c.rgb.iter().zip(&c.mask).map(|(&rgb, &m)| m.then_some(rgb)).collect();
        let got: Vec<Option<[u8; 3]>> = icon.pixels.iter().map(|p| p.map(|i| pal[usize::from(i)])).collect();
        assert_eq!(got, want, "cicn {id}");
    }
    for l in &data.levels {
        assert!(data.tile_set(l).is_some());
    }
    let world = ep.get(b"Hwld", 0).unwrap();
    assert_eq!(data.world, harry_data::WorldInfo::from_hwld(world));
    assert_eq!(data.world_pictures.len(), 26);
    for (id, p) in &data.world_pictures {
        let o = harry_original::pict::decode(ep.get(b"PICT", *id).unwrap()).unwrap();
        let pal = o.palette.unwrap();
        let want: Vec<[u8; 3]> = o.pixels.iter().map(|&i| pal[usize::from(i)]).collect();
        let got: Vec<[u8; 3]> = p.pixels.iter().map(|c| harry_data::clut8()[usize::from(c.unwrap())]).collect();
        assert_eq!((p.width, p.height), (o.width, o.height), "PICT {id}");
        assert!(got == want, "PICT {id}");
    }
    for n in 0..272 {
        let o = 0x3c9 + n * 256;
        let t = &world[o + 1..o + 1 + world[o] as usize];
        assert_eq!(data.texts.get(&(n as i16)).map_or(&[][..], |v| v.as_slice()), t, "text {n}");
    }
    assert_eq!(data.words, harry_original::words::words(&app).unwrap());
    let Some(sounds) = installed("Harry Sounds.bin") else { return };
    for id in sounds.ids(b"snd ") {
        let s = harry_original::snd::decode(sounds.get(b"snd ", id).unwrap()).unwrap();
        assert_eq!((data.sounds[&id].rate, &data.sounds[&id].samples), (s.rate, &s.samples), "snd {id}");
    }
    assert_eq!(data.world_sounds.keys().copied().collect::<Vec<_>>(), [200, 201, 203]);
    let screen = &data.pictures[&129];
    assert_eq!((screen.width, screen.height), (640, 480));
    let meters = &data.pictures[&150];
    assert_eq!((meters.width, meters.height), (147, 44));
}
