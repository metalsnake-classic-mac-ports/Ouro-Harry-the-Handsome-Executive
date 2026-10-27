//! QuickDraw pictures (PICT version 2) with one packed pixmap or bitmap -
//! all the game's pictures are of that kind.

/// A decoded picture.
pub struct Picture {
    pub width: usize,
    pub height: usize,
    /// 8-bit: palette indices; 1-bit: 1 = set (black) pixel.
    pub pixels: Vec<u8>,
    /// 8-bit pictures: their colour table (index → RGB); `None` for 1-bit.
    pub palette: Option<Vec<[u8; 3]>>,
    /// Direct-colour pictures (`DirectBits`): the pixels as RGB, `pixels`
    /// is empty.
    pub rgb: Option<Vec<[u8; 3]>>,
}

#[derive(Debug, thiserror::Error)]
#[error("no packed pixmap or bitmap found in the picture")]
pub struct NoImage;

fn be16(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}

/// PackBits (Inside Macintosh: Imaging, `UnpackBits`).
pub fn unpack_bits(src: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while out.len() < len && i < src.len() {
        let n = src[i] as i8;
        i += 1;
        if n >= 0 {
            let k = n as usize + 1;
            out.extend_from_slice(&src[i..(i + k).min(src.len())]);
            i += k;
        } else if n != -128 {
            let k = 1 - n as isize;
            out.extend(std::iter::repeat_n(src[i], k as usize));
            i += 1;
        }
    }
    out.resize(len, 0);
    out
}

/// Decode the first `PackBitsRect` / `PackBitsRgn` (opcode 0x98 / 0x99) of
/// a picture - an 8-bit pixmap with its colour table or a 1-bit bitmap -
/// or `DirectBitsRect` (0x9a, 32-bit, packType 4).
pub fn decode(p: &[u8]) -> Result<Picture, NoImage> {
    let mut op = 10;
    while op + 4 < p.len() {
        if p[op] == 0 && (p[op + 1] == 0x98 || p[op + 1] == 0x99) {
            if let Some(pic) = decode_at(p, op) {
                return Ok(pic);
            }
        }
        if p[op] == 0 && p[op + 1] == 0x9a {
            if let Some(pic) = decode_direct(p, op) {
                return Ok(pic);
            }
        }
        op += 2;
    }
    Err(NoImage)
}

fn decode_at(p: &[u8], op: usize) -> Option<Picture> {
    let rgn = p[op + 1] == 0x99;
    let pm = op + 2;
    let raw_rb = be16(p, pm);
    let row_bytes = (raw_rb & 0x3fff) as usize;
    let bounds = |o: usize| [be16(p, o) as i16, be16(p, o + 2) as i16, be16(p, o + 4) as i16, be16(p, o + 6) as i16];
    let b = bounds(pm + 2);
    let (width, height) = ((b[3] - b[1]) as usize, (b[2] - b[0]) as usize);
    if width == 0 || height == 0 || row_bytes == 0 {
        return None;
    }
    let mut pos;
    let palette = if raw_rb & 0x8000 != 0 {
        if pm + 54 > p.len() || be16(p, pm + 28) != 8 {
            return None;
        }
        let ct = pm + 46;
        let flags = be16(p, ct + 4);
        let n = be16(p, ct + 6) as usize + 1;
        let mut pal = vec![[0u8; 3]; 256];
        for k in 0..n {
            let e = ct + 8 + k * 8;
            let idx = if flags & 0x8000 != 0 { k } else { be16(p, e) as usize & 0xff };
            pal[idx] = [p[e + 2], p[e + 4], p[e + 6]];
        }
        pos = ct + 8 + n * 8;
        Some(pal)
    } else {
        if row_bytes * 8 < width {
            return None;
        }
        pos = pm + 10;
        None
    };
    pos += 8 + 8 + 2; // srcRect, dstRect, mode
    if rgn {
        pos += be16(p, pos) as usize;
    }
    let mut rows = Vec::with_capacity(width * height);
    for _ in 0..height {
        let line = if row_bytes < 8 {
            // rows shorter than 8 bytes are stored unpacked, without a count
            let l = p[pos..pos + row_bytes].to_vec();
            pos += row_bytes;
            l
        } else {
            let n = if row_bytes > 250 {
                let n = be16(p, pos) as usize;
                pos += 2;
                n
            } else {
                let n = p[pos] as usize;
                pos += 1;
                n
            };
            let l = unpack_bits(&p[pos..pos + n], row_bytes);
            pos += n;
            l
        };
        if palette.is_some() {
            rows.extend_from_slice(&line[..width]);
        } else {
            rows.extend((0..width).map(|x| (line[x / 8] >> (7 - x % 8)) & 1));
        }
    }
    Some(Picture { width, height, pixels: rows, palette, rgb: None })
}

/// `DirectBitsRect` with 32-bit pixels, packType 4 (each row: the
/// component planes one after the other, PackBits-compressed).
fn decode_direct(p: &[u8], op: usize) -> Option<Picture> {
    let pm = op + 2 + 4; // after the fake baseAddr
    let row_bytes = (be16(p, pm) & 0x3fff) as usize;
    let b = [be16(p, pm + 2) as i16, be16(p, pm + 4) as i16, be16(p, pm + 6) as i16, be16(p, pm + 8) as i16];
    let (width, height) = ((b[3] - b[1]) as usize, (b[2] - b[0]) as usize);
    let pack_type = be16(p, pm + 12);
    let pixel_size = be16(p, pm + 28);
    let cmp_count = be16(p, pm + 30) as usize;
    if pixel_size != 32 || pack_type != 4 || !(3..=4).contains(&cmp_count) || width == 0 {
        return None;
    }
    let mut pos = pm + 46 + 8 + 8 + 2; // pixmap, srcRect, dstRect, mode
    let mut rgb = Vec::with_capacity(width * height);
    for _ in 0..height {
        let n = if row_bytes > 250 {
            let n = be16(p, pos) as usize;
            pos += 2;
            n
        } else {
            let n = p[pos] as usize;
            pos += 1;
            n
        };
        let line = unpack_bits(p.get(pos..pos + n)?, width * cmp_count);
        pos += n;
        let plane = |c: usize, x: usize| line[(c + cmp_count - 3) * width + x];
        rgb.extend((0..width).map(|x| [plane(0, x), plane(1, x), plane(2, x)]));
    }
    Some(Picture { width, height, pixels: Vec::new(), palette: None, rgb: Some(rgb) })
}
