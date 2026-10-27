//! Colour icons (`cicn`): PixMap, mask and icon BitMaps, data. The game
//! uses their masks as collision regions.

fn be16(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}

pub struct ColorIcon {
    pub width: usize,
    pub height: usize,
    /// The mask, 1 = set, row major.
    pub mask: Vec<bool>,
    pub depth: u16,
}

/// Decode a `cicn` resource (Inside Macintosh: Imaging, "Color Icon").
pub fn decode(d: &[u8]) -> ColorIcon {
    let top = be16(d, 6) as i16;
    let left = be16(d, 8) as i16;
    let bottom = be16(d, 10) as i16;
    let right = be16(d, 12) as i16;
    let depth = be16(d, 32);
    let (width, height) = ((right - left) as usize, (bottom - top) as usize);
    let m_row = be16(d, 54) as usize;
    let bits = &d[82..82 + m_row * height];
    let mask = (0..height)
        .flat_map(|y| (0..width).map(move |x| bits[y * m_row + x / 8] & (0x80 >> (x % 8)) != 0))
        .collect();
    ColorIcon { width, height, mask, depth }
}
