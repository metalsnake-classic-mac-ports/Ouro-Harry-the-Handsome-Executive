//! Desks (301–319): solid furniture that does nothing itself
//! (`HandleDeskSprite` and `HitDeskSprite` are empty). Kinds below 310
//! stand across (64 × 32), the others along (32 × 64). The other sprites'
//! hit tasks treat a desk as an obstacle: Harry by a mask (HrSp 6021 /
//! 6022), barrels, chairs and plants bounce off (× −0.4, no mask test),
//! robots and reversors turn. Episode 1 has none; the editor's worlds may.

use crate::sprite::{HitTask, Sprite, Task};

/// The faces: HrSp 6001 (across) and 6011 (along); `InitFaces` loads five
/// of each (6001–6005, 6011–6015), but only these two exist.
pub const FACE_ACROSS: i16 = 6001;
pub const FACE_ALONG: i16 = 6011;
/// Harry's collision masks against a desk (`InitFaces`: HrSp 6021 and
/// 6022 with `BuildMaskRegion`).
pub const MASK_ACROSS: i16 = 6021;
pub const MASK_ALONG: i16 = 6022;

/// Does the desk stand across (64 wide, 32 high)? `kind < 310`.
pub fn across(kind: i16) -> bool {
    kind < 310
}

/// `SetupDeskSprite`: layer 1, hot rect (0, 0, 64, 32) across or
/// (0, 0, 32, 64) along. The face is entry kind − 301 (across) or
/// kind − 311 (along) of a table of five: 301 and 311 have their face;
/// for 302–305 and 312–315 the face does not exist (`GetResource` fails),
/// 306–309 and 316–319 read past the table and 310 before it (BUGS.md) -
/// the port gives all of those no face. The original never sets `fix`
/// (left from the record's last sprite); a plant touching the desk sets
/// the desk's position from it, so the desk jumped away (BUGS.md). The
/// port sets it from `at` (a garbage read, fixed).
pub fn setup_desk(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Desk);
    sp.hit_task = Some(HitTask::Desk);
    sp.hit_tile_task = None;
    sp.layer = 1;
    if across(sp.kind) {
        sp.hot_rect = [0, 0, 32, 64];
        if sp.kind == 301 {
            sp.face = FACE_ACROSS;
        }
    } else {
        sp.hot_rect = [0, 0, 64, 32];
        if sp.kind == 311 {
            sp.face = FACE_ALONG;
        }
    }
    sp.at_from_pos();
    sp.fix_from_at();
    sp.b8b = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desks_stand_across_below_310() {
        let mut sp = Sprite { kind: 301, pos: (100, 200), ..Sprite::default() };
        setup_desk(&mut sp);
        assert_eq!((sp.hot_rect, sp.face, sp.layer, sp.at), ([0, 0, 32, 64], FACE_ACROSS, 1, (100, 200)));
        let mut sp = Sprite { kind: 311, ..Sprite::default() };
        setup_desk(&mut sp);
        assert_eq!((sp.hot_rect, sp.face), ([0, 0, 64, 32], FACE_ALONG));
        let mut sp = Sprite { kind: 310, ..Sprite::default() };
        setup_desk(&mut sp);
        assert_eq!((sp.hot_rect, sp.face), ([0, 0, 64, 32], 0));
    }
}
