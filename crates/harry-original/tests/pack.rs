//! The game's data pack (`data/ouro-harry-data.zip`, made by
//! `ouro-harry-extract re/work/installed data/ouro-harry-data.zip`) holds
//! the original's data unchanged; it is checked when the original's files
//! are in `re/work/installed`.

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
    let bytes = std::fs::read(root().join("data/ouro-harry-data.zip")).unwrap();
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
    let app_bin = std::fs::read(root().join("re/work/installed/Harry 1.0.1.bin")).unwrap();
    let section = harry_original::pef::data_section(&MacBinary::parse(&app_bin).unwrap().data).unwrap();
    assert_eq!(data.messages.len(), 53);
    for (n, t) in harry_original::pef::strings(&section, 0x3818, 52).into_iter().enumerate() {
        assert_eq!(data.messages[&(n as i16)], t, "message {n}");
    }
    assert!(data.messages[&-1].starts_with(b"Ben? You mean Ben Spees?"));
    let Some(sounds) = installed("Harry Sounds.bin") else { return };
    for id in sounds.ids(b"snd ") {
        let s = harry_original::snd::decode(sounds.get(b"snd ", id).unwrap()).unwrap();
        assert_eq!((data.sounds[&id].rate, &data.sounds[&id].samples), (s.rate, &s.samples), "snd {id}");
    }
    assert_eq!(data.world_sounds.keys().copied().collect::<Vec<_>>(), [200, 201, 203]);
    // the music: every tune with all its parts played by an instrument
    // whose samples are there, the events as the original's
    let m = &data.music;
    let tunes = installed("Harry Music.bin").unwrap();
    assert_eq!(m.tunes.len(), 17);
    for (id, t) in &m.tunes {
        let (parts, events) = harry_original::qtma::tune(tunes.get(b"Tune", *id).unwrap()).unwrap();
        assert_eq!(t.events, events, "Tune {id}");
        assert_eq!(t.parts.len(), parts.len(), "Tune {id}: a part without an instrument");
        for key in t.parts.values() {
            for z in &m.instruments[key].zones {
                assert!(m.samples.contains_key(&z.sample), "Tune {id}: {key}: no sample {}", z.sample);
            }
        }
    }
    let screen = &data.pictures[&129];
    assert_eq!((screen.width, screen.height), (640, 480));
    let meters = &data.pictures[&150];
    assert_eq!((meters.width, meters.height), (147, 44));
}
