//! Harry's look for the Ouro ports' shared screens (ouro-ui: the level
//! selection, the options and keys pages, the timeline, the box of the
//! port's entries in the main menu): black boxes framed like the
//! original's dialogs (black, white, black lines), the chosen row on the
//! dark red the inventory marks its slot with, text in Helvetica, the
//! accent in the menu's green.

use std::collections::HashMap;

use ouro_ui::{Ink, Rect, Role, Theme};

use crate::render::Canvas;
use crate::text::{Face, Fonts, Line};

const WHITE: [u8; 3] = [0xFF; 3];
const LIGHT: [u8; 3] = [0xDD; 3];
const GREY: [u8; 3] = [0x88; 3];
const GREEN: [u8; 3] = [0x00, 0xFF, 0x00];
const RED: [u8; 3] = [0x88, 0, 0];

pub struct Look<'a> {
    pub fonts: &'a Fonts,
}

fn size(role: Role) -> (Face, f32) {
    match role {
        Role::Heading => (Face::HelveticaBold, 18.0),
        Role::Row => (Face::HelveticaBold, 12.0),
        Role::Small => (Face::Helvetica, 11.0),
        Role::Hint => (Face::Helvetica, 10.0),
    }
}

fn colour(ink: Ink) -> [u8; 3] {
    match ink {
        Ink::Normal => LIGHT,
        Ink::Chosen => WHITE,
        Ink::Dim => GREY,
        Ink::Accent => GREEN,
    }
}

/// The palette index of a colour of the Mac's 8-bit palette (the nearest
/// for another).
pub fn index(rgb: [u8; 3]) -> u8 {
    let clut = harry_data::clut8();
    let d = |c: &[u8; 3]| (0..3).map(|k| (i32::from(c[k]) - i32::from(rgb[k])).pow(2)).sum::<i32>();
    clut.iter().enumerate().min_by_key(|(_, c)| d(c)).map_or(255, |(k, _)| k as u8)
}

const BLACK: u8 = 255;

fn fill(c: &mut Canvas, r: Rect, colour: u8) {
    c.fill(r.left, r.top, r.width(), r.height(), colour);
}

impl Theme for Look<'_> {
    type Surface = Canvas;
    type Text = Line;

    fn width(&self, role: Role, text: &str) -> f32 {
        let (face, size) = size(role);
        self.fonts.width(face, size, text)
    }

    fn backdrop(&self, c: &mut Canvas, screen: Rect) {
        fill(c, screen, BLACK);
    }

    /// The original's dialog frame (`dBoxProc`, as the port's message
    /// boxes draw it) inside the rectangle, black within.
    fn panel(&self, c: &mut Canvas, r: Rect) {
        fill(c, r, BLACK);
        c.frame_rect(r.left + 2, r.top + 2, r.right - 2, r.bottom - 2, index(WHITE));
    }

    fn highlight(&self, c: &mut Canvas, r: Rect) {
        fill(c, r, index(RED));
    }

    fn button(&self, c: &mut Canvas, r: Rect, _primary: bool, focused: bool) {
        fill(c, r, if focused { index(RED) } else { BLACK });
        c.frame_rect(r.left, r.top, r.right, r.bottom, index(WHITE));
    }

    fn rule(&self, c: &mut Canvas, r: Rect) {
        fill(c, r, index(GREY));
    }

    fn fill(&self, c: &mut Canvas, r: Rect, ink: Ink) {
        fill(c, r, index(colour(ink)));
    }

    fn text(&self, x: f32, baseline: f32, role: Role, ink: Ink, text: &str) -> Line {
        let (face, size) = size(role);
        Line { x, baseline, size, face, color: colour(ink), text: words_for_arrows(text) }
    }
}

/// The arrows the shared screens' hints use, in words: Helvetica has no
/// arrow glyphs.
fn words_for_arrows(text: &str) -> String {
    text.replace("↑ ↓", "Up / Down").replace("← →", "Left / Right").replace('↑', "Up").replace('↓', "Down").replace('←', "Left").replace('→', "Right")
}

/// The shared screens' key for a key of the keyboard.
pub fn ui_key(sc: sdl3::keyboard::Scancode) -> Option<ouro_ui::Key> {
    use ouro_ui::Key;
    use sdl3::keyboard::Scancode as S;
    Some(match sc {
        S::Up => Key::Up,
        S::Down | S::Tab => Key::Down,
        S::Left => Key::Left,
        S::Right => Key::Right,
        S::PageUp => Key::PageUp,
        S::PageDown => Key::PageDown,
        S::Home => Key::Home,
        S::End => Key::End,
        S::Return | S::KpEnter => Key::Accept,
        S::Escape => Key::Back,
        S::Space => Key::Toggle,
        _ => return None,
    })
}

/// `src` into `r`, as large as fits without changing its shape, centred;
/// the rest black.
pub fn scaled_into(c: &mut Canvas, r: Rect, src: &Canvas) {
    fill(c, r, BLACK);
    if src.width == 0 || src.height == 0 {
        return;
    }
    let scale = (r.width() as f32 / src.width as f32).min(r.height() as f32 / src.height as f32);
    let (w, h) = ((src.width as f32 * scale) as i32, (src.height as f32 * scale) as i32);
    let (left, top) = (r.left + (r.width() - w) / 2, r.top + (r.height() - h) / 2);
    for y in 0..h {
        let sy = ((y as f32 / scale) as usize).min(src.height - 1);
        for x in 0..w {
            let sx = ((x as f32 / scale) as usize).min(src.width - 1);
            c.set(left + x, top + y, src.pixels[sy * src.width + sx]);
        }
    }
}

/// A canvas at half its size (every other pixel), as RGB PNG.
pub fn half_png(c: &Canvas) -> Vec<u8> {
    let clut = harry_data::clut8();
    let (w, h) = (c.width / 2, c.height / 2);
    let mut p = ouro_play::picture::Picture::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let k = c.pixels[2 * y * c.width + 2 * x] as usize;
            p.rgb[(y * w + x) * 3..][..3].copy_from_slice(&clut[k]);
        }
    }
    p.png()
}

/// A PNG of [`half_png`] back as a canvas (its colours are the palette's).
pub fn from_png(png: &[u8]) -> Option<Canvas> {
    let p = ouro_play::picture::Picture::from_png(png)?;
    let clut = harry_data::clut8();
    let back: HashMap<[u8; 3], u8> = clut.iter().enumerate().rev().map(|(k, c)| (*c, k as u8)).collect();
    let mut c = Canvas::new(p.width, p.height, BLACK);
    for (k, px) in p.rgb.as_chunks::<3>().0.iter().enumerate() {
        c.pixels[k] = back.get(&[px[0], px[1], px[2]]).copied().unwrap_or(BLACK);
    }
    Some(c)
}
