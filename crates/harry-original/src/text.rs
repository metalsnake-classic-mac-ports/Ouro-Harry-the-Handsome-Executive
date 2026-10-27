//! Mac Roman text.

/// Characters 0x80–0xff of Mac Roman.
const HIGH: &str = "ÄÅÇÉÑÖÜáàâäãåçéèêëíìîïñóòôöõúùûü†°¢£§•¶ß®©™´¨≠ÆØ∞±≤≥¥µ∂∑∏π∫ªºΩæø¿¡¬√ƒ≈∆«»… ÀÃÕŒœ–—“”‘’÷◊ÿŸ⁄€‹›ﬁﬂ‡·‚„‰ÂÊÁËÈÍÎÏÌÓÔ\u{f8ff}ÒÚÛÙıˆ˜¯˘˙˚¸˝˛ˇ";

pub fn mac_roman(b: &[u8]) -> String {
    let high: Vec<char> = HIGH.chars().collect();
    b.iter().map(|&c| if c < 0x80 { c as char } else { high[(c - 0x80) as usize] }).collect()
}

pub fn to_mac_roman(s: &str) -> Vec<u8> {
    let high: Vec<char> = HIGH.chars().collect();
    s.chars()
        .map(|c| if (c as u32) < 0x80 { c as u8 } else { high.iter().position(|&h| h == c).map_or(b'?', |p| 0x80 + p as u8) })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn table_is_complete() {
        assert_eq!(super::HIGH.chars().count(), 128);
        assert_eq!(super::mac_roman(&[0xd5]), "’");
    }
}
