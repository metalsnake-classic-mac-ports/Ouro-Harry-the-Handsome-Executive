//! Big-endian numbers, as the Mac stores them. The plain reads panic past
//! the end (for data whose size was checked); the `try_` ones give none.

/// The u16 at `o`.
pub fn be16(d: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([d[o], d[o + 1]])
}

/// The i16 at `o`.
pub fn be16s(d: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([d[o], d[o + 1]])
}

/// The u32 at `o`.
pub fn be32(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// The u16 at `o`, none past the end.
pub fn try_be16(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes(d.get(o..o.checked_add(2)?)?.try_into().ok()?))
}

/// The u32 at `o`, none past the end.
pub fn try_be32(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes(d.get(o..o.checked_add(4)?)?.try_into().ok()?))
}
