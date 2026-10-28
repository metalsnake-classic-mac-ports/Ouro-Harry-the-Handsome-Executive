//! The view: the top-left corner of the 448 × 448 px game area in the level
//! (`FindUpperLeftCorner`, PPC 0x1000c734, once per frame after the
//! sprites moved). How it follows Harry is a preference (prefs +8).

/// The scroll preference (prefs +8).
pub mod mode {
    /// The view stays (only the edge rule moves it).
    pub const STILL: i16 = 0;
    /// Follows a target that eases toward Harry by 1/16 of the distance.
    pub const FOLLOW: i16 = 1;
    /// As FOLLOW, but leading: the view runs ahead of the target.
    pub const LEAD: i16 = 2;
    /// Pages: when Harry is more than 144 px from the middle, the view
    /// scrolls 176 px in 12 steps (8, 16 × 10, 8) ...
    pub const PAGE: i16 = 3;
    /// ... or 160 px in 6 steps (16, 32 × 4, 16).
    pub const PAGE_FAST: i16 = 4;
}

/// The view and its scroll state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// Top-left (v, h) (`_DAT_100659bc`).
    pub corner: (i16, i16),
    /// The eased target of FOLLOW / LEAD (`_DAT_10065984`), (v, h).
    pub target: (i16, i16),
    /// Steps of a page scroll left (`_DAT_1007271e`).
    pub steps: i16,
    /// Set to 2 while paging, counted down after (`_DAT_1007271c`).
    pub settle: i16,
    /// Direction of the page scroll (`_DAT_10072720`): 1 right, 2 left,
    /// 3 down, 4 up, 5 down-right, 6 up-right, 7 down-left, 8 up-left.
    pub page_dir: i16,
    /// Frames of shaking left (`_DAT_10065ad0`): odd frames move the view
    /// left, even right, by half of it (not while paging).
    pub shake: i16,
}

/// Which drawing work the view asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewRedraw {
    /// A page scroll of mode 4 ended: redraw the game area
    /// (`uRam10072710`).
    pub area: bool,
    /// The last step of a page scroll with prefs +2 and +3 set
    /// (`*_DAT_10065974`).
    pub full: bool,
    /// The same with only prefs +2 set: `DrawBlackLines`.
    pub black_lines: bool,
}

/// `(x / 16)`, at least 1 (x > 0).
fn ease(x: i32) -> i16 {
    ((x / 16) as i16).max(1)
}

/// `FindUpperLeftCorner`. `player` is Harry's top-left (h, v)
/// (`gPlayerH`, `gPlayerV`); `even_v` (prefs +2) rounds the corner's v
/// down to even, `blit` is prefs +3.
pub fn find_upper_left_corner(view: &mut View, player: (i16, i16), scroll: i16, even_v: bool, blit: bool) -> ViewRedraw {
    let mut redraw = ViewRedraw::default();
    let (ph, pv) = (i32::from(player.0), i32::from(player.1));
    let ch = player.0.wrapping_add(32);
    let cv = player.1.wrapping_add(32);
    let (old_v, old_h) = view.corner;
    let mid_h = old_h.wrapping_add(224);
    let mid_v = old_v.wrapping_add(224);
    let i = i32::from;
    if scroll < 3 {
        let t = &mut view.target;
        if i(t.1) < ph - 208 {
            t.1 = t.1.wrapping_add(ease(ph - 208 - i(t.1)));
        } else if ph - 208 < i(t.1) {
            t.1 = t.1.wrapping_sub(ease(i(t.1) - (ph - 208)));
        }
        if i(t.0) < pv - 208 {
            t.0 = t.0.wrapping_add(ease(pv - 208 - i(t.0)));
        } else if pv - 208 < i(t.0) {
            t.0 = t.0.wrapping_sub(ease(i(t.0) - (pv - 208)));
        }
        if scroll == mode::LEAD {
            let lead = |t: i16, p: i16| t.wrapping_add(t.wrapping_sub(p.wrapping_sub(208)).wrapping_mul(-2));
            view.corner = (lead(t.0, player.1), lead(t.1, player.0));
        } else if scroll == mode::FOLLOW {
            view.corner = *t;
        }
    } else if view.steps == 0 {
        if view.settle > 0 {
            view.settle -= 1;
        }
        let dh = i(ch) - i(mid_h);
        let dv = i(cv) - i(mid_v);
        let start = |view: &mut View| {
            if scroll == mode::PAGE {
                view.steps = 12;
            } else if scroll == mode::PAGE_FAST {
                view.steps = 6;
            }
        };
        if dh.abs() < 145 {
            if dv.abs() > 144 {
                start(view);
                view.page_dir = if dh.abs() < 73 {
                    if dv < 1 { 4 } else { 3 }
                } else if dv < 1 {
                    if dh < 1 { 8 } else { 6 }
                } else if dh < 1 {
                    7
                } else {
                    5
                };
            }
        } else {
            start(view);
            view.page_dir = if dv.abs() < 73 {
                if dh < 1 { 2 } else { 1 }
            } else if dh < 1 {
                if dv < 1 { 8 } else { 7 }
            } else if dv < 1 {
                6
            } else {
                5
            };
        }
    } else {
        let step = if scroll == mode::PAGE {
            if view.steps == 12 || view.steps == 1 { 8 } else { 16 }
        } else if view.steps == 6 || view.steps == 1 {
            16
        } else {
            32
        };
        view.settle = 2;
        view.steps -= 1;
        let c = &mut view.corner;
        let (dv, dh) = match view.page_dir {
            1 => (0, step),
            2 => (0, -step),
            3 => (step, 0),
            4 => (-step, 0),
            5 => (step, step),
            6 => (-step, step),
            7 => (step, -step),
            8 => (-step, -step),
            _ => (0, 0),
        };
        c.0 = c.0.wrapping_add(dv);
        c.1 = c.1.wrapping_add(dh);
        if view.steps == 0 && scroll == mode::PAGE_FAST {
            redraw.area = true;
        }
        if even_v && view.steps == 0 {
            if blit {
                redraw.full = true;
            } else {
                redraw.black_lines = true;
            }
        }
    }
    let c = &mut view.corner;
    if even_v {
        c.0 = c.0 >> 1 << 1;
    }
    if view.shake > 0 {
        if scroll < 3 {
            if view.shake & 1 == 0 {
                c.1 = c.1.wrapping_add(view.shake >> 1);
            } else {
                c.1 = c.1.wrapping_sub(view.shake >> 1);
            }
        }
        view.shake -= 1;
    }
    // keep Harry within 196 px of the middle
    if i(ch) < i(mid_h) - 196 {
        c.1 = c.1.wrapping_add(old_h.wrapping_add(28).wrapping_sub(ch));
    }
    if i(mid_h) + 196 < i(ch) {
        c.1 = c.1.wrapping_sub(ch.wrapping_sub(old_h.wrapping_add(420)));
    }
    if i(cv) < i(mid_v) - 196 {
        c.0 = c.0.wrapping_add(old_v.wrapping_add(28).wrapping_sub(cv));
    }
    if i(mid_v) + 196 < i(cv) {
        c.0 = c.0.wrapping_sub(cv.wrapping_sub(old_v.wrapping_add(420)));
    }
    c.1 = c.1.clamp(0, 0xe40);
    c.0 = c.0.clamp(0, 0xe40);
    redraw
}
