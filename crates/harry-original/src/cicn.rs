//! Colour icons (`cicn`): PixMap, mask and icon BitMaps, data. The game
//! uses their masks as collision regions, and a few as pictures (the
//! meters' rounded ends).

use crate::bytes::be16;

pub struct ColorIcon {
    pub width: usize,
    pub height: usize,
    /// The mask, 1 = set, row major.
    pub mask: Vec<bool>,
    pub depth: u16,
    /// The colour pixels (RGB, 8 bits per component), row major.
    pub rgb: Vec<[u8; 3]>,
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
    // after the mask and the 1-bit icon: the colour table, then the pixels
    let i_row = be16(d, 68) as usize;
    let ct = 82 + (m_row + i_row) * height;
    let entries = usize::from(be16(d, ct + 6)) + 1;
    // ColorSpecs: the pixel value, then red, green, blue
    let mut colours = vec![[0u8; 3]; 1 << depth.clamp(1, 8)];
    for k in 0..entries {
        let o = ct + 8 + 8 * k;
        if let Some(c) = colours.get_mut(usize::from(be16(d, o))) {
            *c = [0, 1, 2].map(|n| (be16(d, o + 2 + 2 * n) >> 8) as u8);
        }
    }
    let row = (be16(d, 4) & 0x3fff) as usize;
    let px = ct + 8 + 8 * entries;
    let depth_bits = usize::from(depth.max(1));
    let rgb = (0..height)
        .flat_map(|y| (0..width).map(move |x| (y, x)))
        .map(|(y, x)| {
            let bit = x * depth_bits;
            let byte = d.get(px + y * row + bit / 8).copied().unwrap_or(0);
            let v = (byte >> (8 - depth_bits - bit % 8)) & ((1u16 << depth_bits) - 1) as u8;
            colours.get(usize::from(v)).copied().unwrap_or([0, 0, 0])
        })
        .collect();
    ColorIcon { width, height, mask, depth, rgb }
}
