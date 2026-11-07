//! Bouncing off walls (`WallBounce` in the original's Sources).
//!
//! The sprite's position exists twice in the original's record: `pos`
//! (SAT's `position`, +0x02/+0x04) and a working copy (+0x5a/+0x5c, here
//! `at`) that the collision code moves; `fix` (+0x5e/+0x62) is the 24.8
//! position the movement integrates. All are the top-left corner of the
//! sprite's 64 × 64 face; `half` (32) reaches its centre.

use crate::physics::Speed;

/// The parts of a sprite record the collision code reads and writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Body {
    /// SAT position (v, h).
    pub pos: (i16, i16),
    /// Working position (v, h) (+0x5a, +0x5c).
    pub at: (i16, i16),
    /// 24.8 fixed position (h, v).
    pub fix: (i32, i32),
    pub speed: Speed,
}

/// A position (v, h) as a 24.8 fixed position (h, v).
pub fn fix_of(p: (i16, i16)) -> (i32, i32) {
    (i32::from(p.1) << 8, i32::from(p.0) << 8)
}

impl Body {
    /// `fix` = `at` << 8.
    pub fn fix_from_at(&mut self) {
        self.fix = fix_of(self.at);
    }

    /// `fix` = `pos` << 8.
    pub fn fix_from_pos(&mut self) {
        self.fix = fix_of(self.pos);
    }
}

/// A rect as the Toolbox stores it: top, left, bottom, right.
pub type Rect = [i16; 4];

fn sect(a: Rect, b: Rect) -> bool {
    a[0].max(b[0]) < a[2].min(b[2]) && a[1].max(b[1]) < a[3].min(b[3])
}

/// `(b · s) >> 8` with the original's 16-bit result.
fn scale(b: i16, s: i16) -> i16 {
    ((i32::from(b) * i32::from(s)) >> 8) as i16
}

/// Reflect the horizontal component and damp both.
fn reflect_h(s: &mut Speed, b: i16) {
    s.h = scale(b, s.h).wrapping_neg();
    s.v = scale(b, s.v);
}

/// The offset from `at` to a sprite's centre, (v, h); square sprites
/// have the same on both axes (the `…2` routines of the ramps take a
/// point, the others one number).
pub type Half = (i16, i16);

/// Reflect the vertical component and damp both.
fn reflect_v(s: &mut Speed, b: i16) {
    s.h = scale(b, s.h);
    s.v = scale(b, s.v).wrapping_neg();
}

/// `WallBounce` (PPC 0x10019dac) for foreground wall kinds 0–31.
///
/// `tile` is the tile's top-left (v, h) in pixels, `hot` the sprite's hot
/// rect relative to `at`, `bounce` the level's bounce factor, `first`
/// whether this is the first wall contact of the frame (only kind 0 cares).
/// `mask` is the pixel-exact test of the sprite's and the tile's masks
/// (`FUN_10054124`): it gets the body with `pos` set and may move `pos` out
/// of the wall; it returns whether they touched.
///
/// Returns the original's result: 1 when the sprite was moved, 0 when not,
/// and for kinds below 0 or above 55 the kind itself (low byte).
pub fn wall_bounce(
    body: &mut Body,
    kind: i16,
    tile: (i16, i16),
    half: Half,
    bounce: i16,
    hot: Rect,
    first: bool,
    mask: &mut dyn FnMut(&mut Body) -> bool,
) -> i32 {
    if !(0..=0x37).contains(&kind) {
        return i32::from(kind as i8);
    }
    let (tv, th) = tile;
    let mid_h = th.wrapping_add(16);
    let mid_v = tv.wrapping_add(16);
    // the sprite's centre before anything moved
    let ch = body.at.1.wrapping_add(half.1);
    let cv = body.at.0.wrapping_add(half.0);
    let s = &mut body.speed;
    // the result of a diagonal case: 1 reflect h, 2 reflect v, 3 corner
    let mut how = 3;
    let mut corner = kind;
    match kind {
        0 => {
            if s.h < 0 && first {
                reflect_h(s, bounce);
            }
            body.at.1 = mid_h - hot[1];
        }
        1 => {
            if s.v < 0 {
                reflect_v(s, bounce);
            }
            body.at.0 = mid_v - hot[0];
        }
        2 => {
            if s.h > 0 {
                reflect_h(s, bounce);
            }
            body.at.1 = mid_h - hot[3];
        }
        3 => {
            if s.v > 0 {
                reflect_v(s, bounce);
            }
            body.at.0 = mid_v - hot[2];
        }
        4..=7 => {
            if !mask(body) {
                return 0;
            }
            body.at = body.pos;
            let s = &mut body.speed;
            // distances into the quarter tile, from the old centre
            let (d_v, d_h) = match kind {
                4 => (tv.wrapping_add(32).wrapping_sub(cv), ch.wrapping_sub(th)),
                5 => (cv.wrapping_sub(tv), ch.wrapping_sub(th)),
                6 => (cv.wrapping_sub(tv), th.wrapping_add(32).wrapping_sub(ch)),
                _ => (tv.wrapping_add(32).wrapping_sub(cv), th.wrapping_add(32).wrapping_sub(ch)),
            };
            // which side of the quarter: left (4, 5) or right (6, 7); top (5, 6) or bottom (4, 7)
            let left = kind == 4 || kind == 5;
            let top = kind == 5 || kind == 6;
            if d_v < d_h {
                let into = if left { s.h < 0 } else { s.h > 0 };
                if d_v < 9 || !into {
                    if into {
                        reflect_h(s, bounce);
                    }
                    body.at.1 = mid_h - if left { hot[1] } else { hot[3] };
                    body.fix_from_at();
                    return 1;
                }
            } else {
                let into = if top { s.v < 0 } else { s.v > 0 };
                if d_h < 9 || !into {
                    if into {
                        reflect_v(s, bounce);
                    }
                    body.at.0 = mid_v - if top { hot[0] } else { hot[2] };
                    body.fix_from_at();
                    return 1;
                }
            }
            corner = kind + 8;
        }
        8..=11 => {
            // L-shaped tiles: two rects (top, left, bottom, right)
            let (a, b): (Rect, Rect) = match kind {
                8 => ([tv, th, tv + 16, th + 32], [tv, th + 16, tv + 32, th + 32]),
                9 => ([tv + 16, th, tv + 32, th + 32], [tv, th + 16, tv + 32, th + 32]),
                10 => ([tv, th, tv + 32, th + 16], [tv + 16, th, tv + 32, th + 32]),
                _ => ([tv, th, tv + 16, th + 32], [tv, th, tv + 32, th + 16]),
            };
            let (av, ah) = body.at;
            let r = [av + hot[0], ah + hot[1], av + hot[2], ah + hot[3]];
            if !(sect(r, a) || sect(r, b)) {
                return 0;
            }
            let right = kind == 8 || kind == 9;
            let bottom = kind == 9 || kind == 10;
            let d_h = if right { th.wrapping_add(32).wrapping_sub(ch) } else { ch.wrapping_sub(th) };
            let d_v = if bottom { tv.wrapping_add(32).wrapping_sub(cv) } else { cv.wrapping_sub(tv) };
            let edge_h = mid_h - if right { hot[3] } else { hot[1] };
            let edge_v = mid_v - if bottom { hot[2] } else { hot[0] };
            if d_h < d_v {
                if if right { s.h > 0 } else { s.h < 0 } {
                    reflect_h(s, bounce);
                }
                body.at.1 = edge_h;
            } else {
                if if bottom { s.v > 0 } else { s.v < 0 } {
                    reflect_v(s, bounce);
                }
                body.at.0 = edge_v;
            }
            if right && i32::from(edge_h) < i32::from(body.at.1) || !right && i32::from(body.at.1) < i32::from(edge_h) {
                body.at.1 = edge_h;
            }
            if bottom && i32::from(edge_v) < i32::from(body.at.0) || !bottom && i32::from(body.at.0) < i32::from(edge_v) {
                body.at.0 = edge_v;
            }
        }
        12..=31 => {
            body.pos = body.at;
            if !mask(body) {
                return 0;
            }
            body.at = body.pos;
            let ch = body.at.1.wrapping_add(half.1);
            let cv = body.at.0.wrapping_add(half.0);
            let left_half = ch < mid_h;
            let top_half = cv < mid_v;
            // (condition, result when true, result when false); corner kinds as 3 + kind
            let pick = |cond: bool, t: (i32, i16), f: (i32, i16)| if cond { t } else { f };
            let (h2, c2) = match kind {
                12..=15 => (3, kind),
                16 => pick(left_half, (2, 0), (3, 12)),
                17 => pick(top_half, (1, 0), (3, 13)),
                18 => pick(left_half, (3, 14), (2, 0)),
                19 => pick(top_half, (3, 15), (1, 0)),
                20 => pick(top_half, (3, 12), (1, 0)),
                21 => pick(left_half, (2, 0), (3, 13)),
                22 => pick(top_half, (1, 14), (3, 14)),
                23 => pick(left_half, (3, 15), (2, 0)),
                24 => pick(left_half, (3, 12), (2, 0)),
                25 => pick(top_half, (1, 0), (3, 13)),
                26 => pick(left_half, (2, 0), (3, 14)),
                27 => pick(top_half, (1, 0), (3, 15)),
                28 => pick(top_half, (1, 0), (3, 12)),
                29 => pick(left_half, (3, 13), (2, 0)),
                30 => pick(top_half, (3, 14), (1, 0)),
                _ => pick(left_half, (2, 0), (3, 15)),
            };
            how = h2;
            corner = c2;
        }
        _ => return 0,
    }
    if kind >= 4 && !(8..=11).contains(&kind) {
        let s = &mut body.speed;
        match how {
            2 => reflect_v(s, bounce),
            1 => reflect_h(s, bounce),
            _ => {
                if (12..=15).contains(&corner) {
                    if corner == 13 || corner == 15 {
                        std::mem::swap(&mut s.h, &mut s.v);
                    } else {
                        let (h, v) = (s.h, s.v);
                        s.h = v.wrapping_neg();
                        s.v = h.wrapping_neg();
                    }
                }
                s.h = scale(bounce, s.h).wrapping_neg();
                s.v = scale(bounce, s.v).wrapping_neg();
            }
        }
    }
    body.fix_from_at();
    1
}

/// `RectBounceFake` (PPC 0x1001e954): bounce off a rectangular obstacle
/// (`rect` relative to the tile at `tile` (v, h)); used for foreground kinds
/// 32–55 with the background kind rects. The axis is chosen by the
/// distance of the sprite's centre to the rect's centre.
///
/// Its reflection rounds differently from `wall_bounce`:
/// `(−(b · s)) >> 8` instead of `−((b · s) >> 8)`.
pub fn rect_bounce_fake(body: &mut Body, tile: (i16, i16), rect: Rect, half: Half, bounce: i32, hot: Rect) {
    let (tv, th) = tile;
    let top = tv.wrapping_add(rect[0]);
    let left = th.wrapping_add(rect[1]);
    let bottom = tv.wrapping_add(rect[2]);
    let right = th.wrapping_add(rect[3]);
    let (av, ah) = body.at;
    let me = [av.wrapping_add(hot[0]), ah.wrapping_add(hot[1]), av.wrapping_add(hot[2]), ah.wrapping_add(hot[3])];
    if !sect(me, [top, left, bottom, right]) {
        return;
    }
    let ch = ah.wrapping_add(half.1);
    let cv = av.wrapping_add(half.0);
    let mid_h = th
        .wrapping_add(rect[1])
        .wrapping_add(((i32::from(rect[3]) - i32::from(rect[1])) >> 1) as i16);
    let mid_v = tv
        .wrapping_add(rect[0])
        .wrapping_add(((i32::from(rect[2]) - i32::from(rect[0])) >> 1) as i16);
    let s = &mut body.speed;
    let rh = |s: &mut Speed| {
        s.h = ((-(bounce * i32::from(s.h))) >> 8) as i16;
        s.v = ((bounce * i32::from(s.v)) >> 8) as i16;
    };
    let rv = |s: &mut Speed| {
        s.h = ((bounce * i32::from(s.h)) >> 8) as i16;
        s.v = ((-(bounce * i32::from(s.v))) >> 8) as i16;
    };
    // (sprite left of the centre, above it): which side to push out to
    let left_of = ch < mid_h;
    let above = cv < mid_v;
    let d_v = if above { mid_v.wrapping_sub(cv) } else { cv.wrapping_sub(mid_v) };
    let d_h = if left_of { mid_h.wrapping_sub(ch) } else { ch.wrapping_sub(mid_h) };
    if d_v < d_h {
        // push out horizontally
        if left_of {
            if s.h > 0 {
                rh(s);
            }
            body.at.1 = left.wrapping_sub(hot[3]);
        } else {
            if s.h < 0 {
                rh(s);
            }
            body.at.1 = right.wrapping_sub(hot[1]);
        }
    } else if above {
        if s.v > 0 {
            rv(s);
        }
        body.at.0 = top.wrapping_sub(hot[2]);
    } else {
        if s.v < 0 {
            rv(s);
        }
        body.at.0 = bottom.wrapping_sub(hot[0]);
    }
    if body.at.1 != ah {
        body.fix.0 = i32::from(body.at.1) << 8;
    }
    if body.at.0 != av {
        body.fix.1 = i32::from(body.at.0) << 8;
    }
}

/// `BGTileBounce` (PPC 0x1001c8a4): bounce off a background obstacle
/// (background kind 1) at `tile` (v, h). `mask` is the region test as in
/// [`wall_bounce`]; `fg_kind(x, y)` gives the foreground kind of a tile
/// (`LookupFGTileKind(GetFGTile(x, y))`, −1 for none) - a side already
/// covered by a neighbouring wall is not used.
///
/// The speed is damped twice: once when the side is chosen, and again
/// (with the sign forced away from the tile) on the frame's first contact.
/// The position is only clamped to 14 px from the tile's centre.
pub fn bg_tile_bounce(
    body: &mut Body,
    tile: (i16, i16),
    half: Half,
    bounce: i16,
    hot: Rect,
    first: bool,
    mask: &mut dyn FnMut(&mut Body) -> bool,
    fg_kind: &dyn Fn(i32, i32) -> i16,
) -> i32 {
    tile_bounce(body, tile, half, bounce, hot, first, mask, fg_kind, true)
}

/// `BGTileBounce2` (PPC 0x1001cf2c, the ramps'): as [`bg_tile_bounce`],
/// but only a straight neighbouring wall (kinds 0–3) closes a side, not
/// the diagonal ones.
#[allow(clippy::too_many_arguments)]
pub fn bg_tile_bounce2(
    body: &mut Body,
    tile: (i16, i16),
    half: Half,
    bounce: i16,
    hot: Rect,
    first: bool,
    mask: &mut dyn FnMut(&mut Body) -> bool,
    fg_kind: &dyn Fn(i32, i32) -> i16,
) -> i32 {
    tile_bounce(body, tile, half, bounce, hot, first, mask, fg_kind, false)
}

#[allow(clippy::too_many_arguments)]
fn tile_bounce(
    body: &mut Body,
    tile: (i16, i16),
    half: Half,
    bounce: i16,
    hot: Rect,
    first: bool,
    mask: &mut dyn FnMut(&mut Body) -> bool,
    fg_kind: &dyn Fn(i32, i32) -> i16,
    diagonals: bool,
) -> i32 {
    let (tv, th) = tile;
    let mid_h = th.wrapping_add(16);
    let mid_v = tv.wrapping_add(16);
    if !mask(body) {
        return 0;
    }
    let ch = body.at.1.wrapping_add(half.1);
    let cv = body.at.0.wrapping_add(half.0);
    let dh = mid_h.wrapping_sub(ch).wrapping_abs();
    let dv = mid_v.wrapping_sub(cv).wrapping_abs();
    let s = &mut body.speed;
    // side: 1 right of the tile, 2 left, 3 above, 4 below
    let mut side;
    if dv < dh {
        s.h = scale(bounce, s.h).wrapping_neg();
        s.v = scale(bounce, s.v);
        side = if mid_h < ch { 1 } else { 2 };
    } else {
        s.h = scale(bounce, s.h);
        s.v = scale(bounce, s.v);
        side = if mid_v < cv { 4 } else { 3 };
    }
    let x = i32::from(th) >> 5;
    let y = i32::from(tv) >> 5;
    let h_side = if mid_h < ch { 1 } else { 2 };
    let v_side = if mid_v < cv { 4 } else { 3 };
    match side {
        3 if matches!(fg_kind(x, y - 1), 1) || diagonals && matches!(fg_kind(x, y - 1), 6 | 5) => side = h_side,
        1 if matches!(fg_kind(x + 1, y), 2) || diagonals && matches!(fg_kind(x + 1, y), 7 | 6) => side = v_side,
        2 if matches!(fg_kind(x - 1, y), 0) || diagonals && matches!(fg_kind(x - 1, y), 4 | 5) => side = v_side,
        4 if matches!(fg_kind(x, y + 1), 3) || diagonals && matches!(fg_kind(x, y + 1), 7 | 4) => side = h_side,
        _ => {}
    }
    let damp = |s: &mut Speed| {
        s.h = scale(bounce, s.h);
        s.v = scale(bounce, s.v);
    };
    let (av, ah) = (i32::from(body.at.0), i32::from(body.at.1));
    match side {
        3 => {
            if first {
                damp(s);
                if s.v > 0 {
                    s.v = s.v.wrapping_neg();
                }
            }
            let edge = i32::from(mid_v) - 14 - i32::from(hot[2]);
            if edge < av {
                body.at.0 = edge as i16;
            }
        }
        1 => {
            if first {
                damp(s);
                if s.h < 0 {
                    s.h = s.h.wrapping_neg();
                }
            }
            let edge = i32::from(mid_h) + 14 - i32::from(hot[1]);
            if ah < edge {
                body.at.1 = edge as i16;
            }
        }
        2 => {
            if first {
                damp(s);
                if s.h > 0 {
                    s.h = s.h.wrapping_neg();
                }
            }
            let edge = i32::from(mid_h) - 14 - i32::from(hot[3]);
            if edge < ah {
                body.at.1 = edge as i16;
            }
        }
        _ => {
            if first {
                s.h = scale(bounce, s.h);
                s.v = scale(bounce, s.v).wrapping_neg();
                if s.v < 0 {
                    s.v = s.v.wrapping_neg();
                }
            }
            let edge = i32::from(mid_v) + 14 - i32::from(hot[0]);
            if av < edge {
                body.at.0 = edge as i16;
            }
        }
    }
    body.fix_from_at();
    1
}

/// `RectBounce` (PPC 0x1001e4bc): bounce a sprite (centre at + `half`,
/// hot rect `hot`) off another sprite's hot rect (at its `at`) when they
/// overlap: pushed out on the side its centre is nearest to (comparing
/// the distances to the other's centre), the speed across reflected and
/// both damped × bounce / 256 if it moves into the rect.
pub fn rect_bounce(body: &mut Body, half: i16, bounce: i32, hot: Rect, other_at: (i16, i16), other_hot: Rect) {
    let o = [
        other_at.0.wrapping_add(other_hot[0]),
        other_at.1.wrapping_add(other_hot[1]),
        other_at.0.wrapping_add(other_hot[2]),
        other_at.1.wrapping_add(other_hot[3]),
    ];
    let me = [body.at.0.wrapping_add(hot[0]), body.at.1.wrapping_add(hot[1]), body.at.0.wrapping_add(hot[2]), body.at.1.wrapping_add(hot[3])];
    if !crate::tiles::sect(me, o) {
        return;
    }
    let ch = body.at.1.wrapping_add(half);
    let cv = body.at.0.wrapping_add(half);
    let half_of = |a: i16, b: i16| ((i32::from(b) - i32::from(a)) >> 1) as i16;
    let oh = other_at.1.wrapping_add(other_hot[1]).wrapping_add(half_of(other_hot[1], other_hot[3]));
    let ov = other_at.0.wrapping_add(other_hot[0]).wrapping_add(half_of(other_hot[0], other_hot[2]));
    let old = body.at;
    let s = &mut body.speed;
    let damp = |x: i16| ((bounce * i32::from(x)) >> 8) as i16;
    let flip = |x: i16| ((-(bounce * i32::from(x))) >> 8) as i16;
    // which side: 0 left, 1 top, 2 bottom, 3 right
    let side = if ch < oh {
        let dh = oh.wrapping_sub(ch);
        if cv < ov {
            if ov.wrapping_sub(cv) < dh { 0 } else { 1 }
        } else if cv.wrapping_sub(ov) < dh {
            0
        } else {
            2
        }
    } else {
        let dh = ch.wrapping_sub(oh);
        if cv < ov {
            if ov.wrapping_sub(cv) < dh { 3 } else { 1 }
        } else if cv.wrapping_sub(ov) < dh {
            3
        } else {
            2
        }
    };
    match side {
        0 => {
            if s.h > 0 {
                (s.h, s.v) = (flip(s.h), damp(s.v));
            }
            body.at.1 = o[1].wrapping_sub(hot[3]);
        }
        3 => {
            if s.h < 0 {
                (s.h, s.v) = (flip(s.h), damp(s.v));
            }
            body.at.1 = o[3].wrapping_sub(hot[1]);
        }
        1 => {
            if s.v > 0 {
                (s.h, s.v) = (damp(s.h), flip(s.v));
            }
            body.at.0 = o[0].wrapping_sub(hot[2]);
        }
        _ => {
            if s.v < 0 {
                (s.h, s.v) = (damp(s.h), flip(s.v));
            }
            body.at.0 = o[2].wrapping_sub(hot[0]);
        }
    }
    if old.1 != body.at.1 {
        body.fix.0 = i32::from(body.at.1) << 8;
    }
    if old.0 != body.at.0 {
        body.fix.1 = i32::from(body.at.0) << 8;
    }
}
