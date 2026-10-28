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
    for l in &data.levels {
        assert!(data.tile_set(l).is_some());
    }
    let screen = &data.pictures[&129];
    assert_eq!((screen.width, screen.height), (640, 480));
    let meters = &data.pictures[&150];
    assert_eq!((meters.width, meters.height), (147, 44));
}
