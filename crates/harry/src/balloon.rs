//! Speech balloons (`ShowBalloon`'s frontend part): the text laid out as
//! TextEdit does in Geneva 9 within 195 px, the balloon (210 px wide, its
//! tail under it) placed over the sprite - over Harry in mode 2 - and shown
//! only if it lies inside the game area with a 32 px margin. While it is
//! shown the game waits: Return or Enter closes it at once, any key after
//! a second.
//!
//! The original measures with the Mac's bitmap Geneva 9; the port with the
//! system's Geneva (or a substitute) at the same size, so a text may break
//! into a different number of lines - and a balloon at the edge of fitting
//! may then be shown where the original did not, or the other way round.

use harry_core::game::Balloon;
use harry_core::kinds::BalloonText;
use harry_core::text::mac_roman;
use harry_data::GameData;

use crate::render::Canvas;
use crate::text::{Face, Fonts, Line};

const WIDTH: i32 = 210;
const TEXT_WIDTH: f32 = 195.0;
/// Geneva 9's line height (12 px, measured on the emulator's balloons) and ascent.
const LINE_HEIGHT: i32 = 12;
const ASCENT: i32 = 9;
const SIZE: f32 = 9.0;
const WHITE: u8 = 0;
const BLACK: u8 = 255;

/// A balloon on the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    /// Top-left on the 640 × 480 screen and the height without the tail.
    pub x: i32,
    pub y: i32,
    pub height: i32,
    pub lines: Vec<String>,
}

/// The text of a balloon, Mac Roman.
pub fn text_of(b: &Balloon, data: &GameData) -> Vec<u8> {
    match &b.text {
        BalloonText::World(n) => data.texts.get(n).cloned().unwrap_or_default(),
        BalloonText::Buzzwords(t) | BalloonText::Phonewords(t) => t.clone(),
        BalloonText::Message(n) => data.messages.get(n).cloned().unwrap_or_default(),
    }
}

/// TextEdit's line breaking: at returns, and before a word that would
/// pass the width (spaces stay at the end of the line); a word longer than
/// a line is broken where it reaches the edge.
fn wrap(text: &str, fonts: &Fonts) -> Vec<String> {
    let fits = |s: &str| fonts.width(Face::Geneva, SIZE, s.trim_end()) <= TEXT_WIDTH;
    let mut lines = Vec::new();
    for para in text.split('\r') {
        let mut line = String::new();
        for word in para.split_inclusive(' ') {
            let candidate = format!("{line}{word}");
            if fits(&candidate) {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            for c in word.chars() {
                let candidate = format!("{line}{c}");
                if !fits(&candidate) && !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(c);
            }
        }
        lines.push(line);
    }
    lines
}

/// Place a balloon; `None` if it does not fit (the game then goes on, see
/// `harry_core::game` for what it does then).
pub fn place(b: &Balloon, data: &GameData, fonts: &Fonts) -> Option<Shown> {
    let lines = wrap(&mac_roman(&text_of(b, data)), fonts);
    let height = (lines.len() as i32 + 1) * LINE_HEIGHT;
    let (view_v, view_h) = (i32::from(b.view.0), i32::from(b.view.1));
    let (x, y) = if b.mode == 2 {
        (i32::from(b.player.0) - 49 - view_h, i32::from(b.player.1) + 13 - view_v - height)
    } else {
        (i32::from(b.at.1) - view_h - 57, i32::from(b.at.0) - view_v + 17 - height)
    };
    let fits = x >= 32 && y >= 32 && x + WIDTH <= 448 && y + 8 + height <= 448;
    fits.then_some(Shown { x, y, height, lines })
}

/// Draw the balloon: the game area dimmed with a 50 % pattern, the white
/// round rect and tail; returns the text lines to draw on top.
pub fn draw(c: &mut Canvas, s: &Shown) -> Vec<Line> {
    for y in 16..464usize {
        for x in 16..464usize {
            if (x + y) % 2 == 0 {
                c.pixels[y * c.width + x] = BLACK;
            }
        }
    }
    let (l, t, r, b) = (s.x, s.y, s.x + WIDTH, s.y + s.height);
    let radius = 8;
    for y in t..b {
        for x in l..r {
            // the corners of PaintRoundRect(…, 16, 16)
            let cx = if x < l + radius { l + radius } else if x >= r - radius { r - radius - 1 } else { x };
            let cy = if y < t + radius { t + radius } else if y >= b - radius { b - radius - 1 } else { y };
            let (dx, dy) = (x - cx, y - cy);
            if dx * dx + dy * dy <= radius * radius {
                c.set(x, y, WHITE);
            }
        }
    }
    // the tail: from the base (l + 96 … l + 104, b) down to (l + 88, b + 8)
    for k in 0..8 {
        for x in (l + 96 - k)..(l + 104 - 2 * k) {
            c.set(x, b + k, WHITE);
        }
    }
    s.lines
        .iter()
        .enumerate()
        .map(|(i, text)| Line {
            x: (l + 5) as f32,
            baseline: (t + 5 + ASCENT + i as i32 * LINE_HEIGHT) as f32,
            size: SIZE,
            face: Face::Geneva,
            color: [0, 0, 0],
            text: text.clone(),
        })
        .collect()
}
