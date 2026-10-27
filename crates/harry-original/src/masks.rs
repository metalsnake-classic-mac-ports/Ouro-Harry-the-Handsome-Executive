//! The collision masks of the game, made from the original's resources as
//! `InitFaces` / `LoadEncFaceSetFromPICT` make them (NOTES.md "Collision
//! masks").

use harry_core::mask::Mask;

use crate::resfork::{Error, Resources};
use crate::{cicn, hrsp, pict};

/// A `cicn`'s mask (Harry's body 299, the background obstacle 290, ...).
pub fn from_cicn(res: &Resources, id: i16) -> Result<Mask, Error> {
    let c = cicn::decode(res.get(b"cicn", id)?);
    Ok(Mask::new(c.width as i32, c.height as i32, c.mask))
}

/// The 32 wall shapes: application PICT 160, 8 tiles of 32 × 32 per row.
pub fn walls(app: &Resources) -> Result<Vec<Mask>, Error> {
    let p = pict::decode(app.get(b"PICT", 160)?).map_err(|_| Error::BadFork("PICT 160 is not a bitmap"))?;
    Ok((0..32)
        .map(|k| {
            let (ox, oy) = ((k % 8) * 32, (k / 8) * 32);
            let bits = (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).map(|(x, y)| p.pixels[(oy + y) * p.width + ox + x] != 0).collect();
            Mask::new(32, 32, bits)
        })
        .collect())
}

/// `BuildMaskRegion` of a face: pixels other than index 0 (white) count,
/// but `Threshold255Blit` packs each group of 8 pixels least significant
/// bit first while `BitMapToRegion` reads the most significant bit as the
/// leftmost pixel - so every 8-pixel group comes out mirrored (BUGS.md).
pub fn from_face(face: &hrsp::Face) -> Mask {
    let (w, h) = (face.width, face.height);
    let mut bits = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            if face.pixels[y * w + x].is_some_and(|c| c != 0) {
                let mirrored = (x & !7) + 7 - (x & 7);
                if mirrored < w {
                    bits[y * w + mirrored] = true;
                }
            }
        }
    }
    Mask::new(w as i32, h as i32, bits)
}
