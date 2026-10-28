//! Props: pictures drawn behind (background) or over (foreground) the
//! sprites, without a sprite of their own (`AddBGProp`, `AddFGProp`; two
//! tables of 64 slots at 0x100723ac and 0x100720ac, cleared at the level
//! start). `GenerateSprite` makes them from object ids 1019 / 1029 /
//! 1039 / 1049 (background) and 1609 / 1619 / 1629 / 1639 (foreground);
//! the drawing code draws each one whose point lies in the view.

use crate::sprite::Face;

/// Slots per table; a full table drops new props.
pub const SLOTS: usize = 64;

/// One prop: its point (v, h) - the object's position - and face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Prop {
    pub at: (i16, i16),
    pub face: Face,
}

/// One table, in slot order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Props {
    pub slots: Vec<Prop>,
}

impl Props {
    /// `AddBGProp` / `AddFGProp`: into the first free slot.
    pub fn add(&mut self, at: (i16, i16), face: Face) {
        if self.slots.len() < SLOTS {
            self.slots.push(Prop { at, face });
        }
    }
}

/// The prop of an object id (HrSp ids 6034–6037, 6051–6054): (in the
/// foreground, face), or None if the id is no prop.
pub fn prop_face(id: i16) -> Option<(bool, Face)> {
    match id {
        1019 | 1029 | 1039 | 1049 => Some((false, 6034 + (id - 1019) / 10)),
        1609 | 1619 | 1629 | 1639 => Some((true, 6051 + (id - 1609) / 10)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_table_drops_new_props() {
        let mut p = Props::default();
        for i in 0..70 {
            p.add((i, i), 6034);
        }
        assert_eq!(p.slots.len(), SLOTS);
        assert_eq!(p.slots[63].at, (63, 63));
        assert_eq!(prop_face(1049), Some((false, 6037)));
        assert_eq!(prop_face(1629), Some((true, 6053)));
        assert_eq!(prop_face(1021), None);
    }
}
