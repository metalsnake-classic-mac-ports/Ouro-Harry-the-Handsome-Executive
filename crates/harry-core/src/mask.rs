//! Pixel-exact collision masks: the original's QuickDraw regions made from
//! 1-bit masks (`BitMapToRegion`) are exactly the sets of their pixels, so
//! a mask here is a pixel set with the same operations the game uses
//! (offset, intersect, empty, bounding box).

/// A 1-bit mask; pixel (x, y) with x to the right, y down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mask {
    pub width: i32,
    pub height: i32,
    bits: Vec<bool>,
    /// The set pixels, for fast tests.
    pixels: Vec<(i32, i32)>,
}

impl Mask {
    pub fn new(width: i32, height: i32, bits: Vec<bool>) -> Mask {
        assert_eq!(bits.len(), (width * height) as usize);
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| bits[(y * width + x) as usize])
            .collect();
        Mask { width, height, bits, pixels }
    }

    pub fn get(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height && self.bits[(y * self.width + x) as usize]
    }

    /// The tight bounds (left, top, right, bottom) like a region's
    /// `rgnBBox`; `None` for an empty mask.
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let mut it = self.pixels.iter();
        let &(x0, y0) = it.next()?;
        Some(it.fold((x0, y0, x0 + 1, y0 + 1), |(l, t, r, b), &(x, y)| (l.min(x), t.min(y), r.max(x + 1), b.max(y + 1))))
    }

    /// Bounds of the overlap of `self` at `a` and `other` at `b` (both
    /// (h, v) offsets), `None` when they do not overlap.
    pub fn overlap(&self, a: (i32, i32), other: &Mask, b: (i32, i32)) -> Option<(i32, i32, i32, i32)> {
        let mut bounds: Option<(i32, i32, i32, i32)> = None;
        for &(x, y) in &self.pixels {
            let (gx, gy) = (x + a.0, y + a.1);
            if other.get(gx - b.0, gy - b.1) {
                bounds = Some(match bounds {
                    None => (gx, gy, gx + 1, gy + 1),
                    Some((l, t, r, bt)) => (l.min(gx), t.min(gy), r.max(gx + 1), bt.max(gy + 1)),
                });
            }
        }
        bounds
    }
}

fn sign(v: i16) -> i16 {
    v.signum()
}

/// Half of a 16-bit sum, rounded toward zero (C division by 2).
fn half(sum: i32) -> i16 {
    (sum as i16) / 2
}

/// The original's mask test (`FUN_10054124`): does `sprite` at `pos`
/// (v, h; the sprite's top-left) touch `tile_mask` at `tile` (v, h)? If so
/// the sprite is moved out (`FUN_10053eac`, 68k `REGIONSEPARATE`): one
/// pixel at a time along a Bresenham line pointing from the overlap's
/// centre to the sprite's centre, until they no longer overlap.
pub fn mask_hit(pos: &mut (i16, i16), sprite: &Mask, tile: (i16, i16), tile_mask: &Mask) -> bool {
    let t = (i32::from(tile.1), i32::from(tile.0));
    let at = |p: &(i16, i16)| (i32::from(p.1), i32::from(p.0));
    let Some(sect) = sprite.overlap(at(pos), tile_mask, t) else { return false };
    let Some(own) = sprite.bounds() else { return true };
    let (l, top, r, b) = (own.0 + i32::from(pos.1), own.1 + i32::from(pos.0), own.2 + i32::from(pos.1), own.3 + i32::from(pos.0));
    let dx = half(r + l).wrapping_sub(half(sect.2 + sect.0));
    let dy = half(b + top).wrapping_sub(half(sect.3 + sect.1));
    let (adx, ady) = (dx.wrapping_abs(), dy.wrapping_abs());
    let sx = sign(dx);
    let mut sy = sign(dy);
    if sx == 0 && sy == 0 {
        sy = 1;
    }
    let mut err: i16 = 0;
    loop {
        if ady < adx {
            pos.1 = pos.1.wrapping_add(sx);
            err = err.wrapping_add(ady);
            if adx < err {
                pos.0 = pos.0.wrapping_add(sy);
                err = err.wrapping_sub(adx);
            }
        } else {
            pos.0 = pos.0.wrapping_add(sy);
            err = err.wrapping_add(adx);
            if ady < err {
                pos.1 = pos.1.wrapping_add(sx);
                err = err.wrapping_sub(ady);
            }
        }
        if sprite.overlap(at(pos), tile_mask, t).is_none() {
            return true;
        }
    }
}
