//! The game's encoded sprite faces (`HrSp`, "Harry Sprite Face"): two
//! Rects, then 32-bit commands (high byte opcode, low 24 bits n) as
//! `BlitEncFaceNoClip` reads them: 0 end, 1 next row, 2 copy n pixels
//! (padded to 4 bytes), 3 skip n pixels.

pub struct Face {
    /// Bounds (top, left, bottom, right) and the second rect of the header.
    pub bounds: [i16; 4],
    pub rect2: [i16; 4],
    pub width: usize,
    pub height: usize,
    /// Row major; `None` = transparent, else a clut 8 index.
    pub pixels: Vec<Option<u8>>,
}

#[derive(Debug, thiserror::Error)]
#[error("bad HrSp face: {0}")]
pub struct BadFace(&'static str);

fn be16(d: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([d[o], d[o + 1]])
}

pub fn decode(d: &[u8]) -> Result<Face, BadFace> {
    if d.len() < 16 {
        return Err(BadFace("too short"));
    }
    let bounds = [be16(d, 0), be16(d, 2), be16(d, 4), be16(d, 6)];
    let rect2 = [be16(d, 8), be16(d, 10), be16(d, 12), be16(d, 14)];
    let width = (bounds[3] - bounds[1]) as usize;
    let height = (bounds[2] - bounds[0]) as usize;
    let mut pixels = vec![None; width * height];
    let mut row: Option<usize> = None;
    let mut x = 0;
    let mut p = 16;
    while p + 4 <= d.len() {
        let word = u32::from_be_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]]);
        p += 4;
        let (op, n) = (word >> 24, (word & 0xff_ffff) as usize);
        match op {
            0 => break,
            1 => {
                row = Some(row.map_or(0, |r| r + 1));
                x = 0;
            }
            2 => {
                let r = row.ok_or(BadFace("pixels before the first row"))?;
                if p + n > d.len() {
                    return Err(BadFace("pixels beyond the data"));
                }
                for (i, &c) in d[p..p + n].iter().enumerate() {
                    if r < height && x + i < width {
                        pixels[r * width + x + i] = Some(c);
                    }
                }
                x += n;
                p += (n + 3) & !3;
            }
            3 => x += n,
            _ => return Err(BadFace("unknown opcode")),
        }
    }
    Ok(Face { bounds, rect2, width, height, pixels })
}
