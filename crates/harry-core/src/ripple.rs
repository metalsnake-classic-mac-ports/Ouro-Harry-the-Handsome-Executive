//! The ripple: the game area drawn with its rows shifted sideways along a
//! sine wave that runs down the screen (`DoubleBlitSine`). Its length,
//! width and speed are the level's info #10–#12 (level 10: 40, 3, 384);
//! W-A-V-Y-G-R-A-V-Y switches the cheat's (100, 16, 0x300) on and off.

use crate::level::Level;

/// Level info fields of the ripple.
pub const LENGTH: usize = 10;
pub const WIDTH: usize = 11;
pub const SPEED: usize = 12;

/// The original's 2π (`_DAT_10066720`), short of the real one.
#[allow(clippy::approx_constant)]
const TWO_PI: f64 = 6.28318;

/// `BuildSineTable`: the length is cut to 100 (in the level itself); entry
/// i is `width · sin(6.28318 / length · i)`, the product in single
/// precision, cut toward zero.
pub fn sine_table(level: &mut Level) -> Vec<i16> {
    if level.info[LENGTH] > 100 {
        level.info[LENGTH] = 100;
    }
    let (len, width) = (level.info[LENGTH], level.info[WIDTH]);
    let step = (TWO_PI / f64::from(len)) as f32;
    (0..len.max(0))
        .map(|i| {
            let s = (f64::from(step) * f64::from(i)) as f32;
            let s = f64::from(s).sin() as f32;
            (s * f32::from(width)) as i32 as i16
        })
        .collect()
}

/// The cheat's switch: on, the level gets length 100, width 16, speed
/// 0x300 (and the table is built); off, all three 0. The phase starts over.
pub fn toggle(level: &mut Level, phase: &mut i32, on: bool) {
    *phase = 0;
    if on {
        level.info[LENGTH] = 100;
        level.info[WIDTH] = 16;
        level.info[SPEED] = 0x300;
        sine_table(level);
    } else {
        level.info[LENGTH] = 0;
        level.info[WIDTH] = 0;
        level.info[SPEED] = 0;
    }
}

/// The shift of screen row `y` of the game area (0 at its top): the table
/// at (the view's top + the phase's whole part + y) mod the length. The
/// remainder keeps the sign of C's `%`, so rows above the level's top
/// would read before the table; the view never goes there.
pub fn row_shift(table: &[i16], view_v: i16, phase: i32, y: i32) -> i16 {
    let len = table.len() as i32;
    if len == 0 {
        return 0;
    }
    let i = i32::from(view_v) + (phase >> 8) + y;
    table.get((i % len) as usize).copied().unwrap_or(0)
}

/// Whether the game area is drawn rippled: a width of at least 1, scroll
/// modes 0–2 (above, the view jumps and is drawn whole each time), and not
/// a frame that redraws everything (the first after a level start).
pub fn active(level: &Level, scroll: i16, full_redraw: bool) -> bool {
    level.info[WIDTH] >= 1 && !full_redraw && scroll <= 2
}
