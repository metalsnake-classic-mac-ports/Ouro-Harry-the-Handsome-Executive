//! Software rendering into screen palette indices (`clut` 8), as the
//! original draws a frame: the background tiles, the background props,
//! the sprites with +0x8b set (`DrawSpritesBehindTiles`, which also draws
//! the foreground props), the foreground tiles (`RedrawFGTiles`), then the
//! other sprites (`DrawSpritesAboveTiles`) - each in SAT's list order, at
//! their `at`, scaled when their scale is not 1.0.
//!
//! The game area is drawn 480 px wide, like the original's ring buffer,
//! and copied to the screen row by row - shifted sideways where the level
//! ripples (`DoubleBlitSine`).
//!
//! Each sprite is drawn in the colours its draw proc chose in the game's
//! draw pass (`Game::blits`, `harry_core::tint`): tinted, translucent,
//! reddened or heated through the tables `BuildTintTable` makes.
//!
//! Not drawn yet: the panel's texts (level name, score), the meters'
//! rounded ends, the fans' grilles and the dart enemies' loaded darts.

use harry_core::game::Game;
use harry_core::ripple;
use harry_core::level::{NO_TILE, SIZE};
use harry_core::shots::scaled_pos;
use harry_core::random::Random;
use harry_core::sprite::Sprite;
use harry_core::tint::{self, Blit, Tables};
use harry_data::{GameData, Image, clut8};

pub const SCREEN_WIDTH: usize = 640;
pub const SCREEN_HEIGHT: usize = 480;
/// The game area: 448 × 448 at (16, 16).
const AREA: i32 = 16;
const AREA_SIZE: i32 = 448;
/// The width the game area is drawn in (the ring buffer's).
const RING: i32 = 480;
const BLACK: u8 = 255;

/// An image of palette indices.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
    /// Where level positions are drawn: (x, y, width, height).
    area: (i32, i32, i32, i32),
}

impl Canvas {
    pub fn new(width: usize, height: usize, fill: u8) -> Canvas {
        Canvas { width, height, pixels: vec![fill; width * height], area: (AREA, AREA, AREA_SIZE, AREA_SIZE) }
    }

    /// Set a pixel of the game area at level position (h, v), in a
    /// colour mode (`Shade`).
    fn plot(&mut self, view: (i16, i16), h: i32, v: i32, c: u8, shade: Shade) {
        let x = h - i32::from(view.1);
        let y = v - i32::from(view.0);
        let (ax, ay, aw, ah) = self.area;
        if (0..aw).contains(&x) && (0..ah).contains(&y) {
            let p = &mut self.pixels[(ay + y) as usize * self.width + (ax + x) as usize];
            *p = match shade {
                Some((mode, index)) => tables().shade(mode, index, c, *p),
                None => c,
            };
        }
    }

    /// Draw part of an image (`src` = (x, y, w, h)) at level position (h, v).
    fn blit(&mut self, view: (i16, i16), img: &Image, src: (usize, usize, usize, usize), at: (i32, i32), shade: Shade) {
        let (sx, sy, w, h) = src;
        for y in 0..h {
            for x in 0..w {
                if let Some(c) = img.get(sx + x, sy + y) {
                    self.plot(view, at.0 + x as i32, at.1 + y as i32, c, shade);
                }
            }
        }
    }

    /// Draw an image scaled to `size` (w, h) at level position (h, v).
    fn blit_scaled(&mut self, view: (i16, i16), img: &Image, size: (usize, usize), at: (i32, i32), shade: Shade) {
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        for y in 0..size.1 {
            for x in 0..size.0 {
                if let Some(c) = img.get(x * img.width / size.0, y * img.height / size.1) {
                    self.plot(view, at.0 + x as i32, at.1 + y as i32, c, shade);
                }
            }
        }
    }

    /// Draw part of an image (`src` = (x, y, w, h)) at screen (x, y).
    fn draw_image(&mut self, img: &Image, src: (usize, usize, usize, usize), x: usize, y: usize) {
        let (sx, sy, w, h) = src;
        for yy in 0..h.min(img.height.saturating_sub(sy)) {
            for xx in 0..w.min(img.width.saturating_sub(sx)) {
                if let Some(c) = img.get(sx + xx, sy + yy)
                    && x + xx < self.width
                    && y + yy < self.height
                {
                    self.pixels[(y + yy) * self.width + x + xx] = c;
                }
            }
        }
    }

    /// Set a screen pixel (outside the screen: nothing).
    pub fn set(&mut self, x: i32, y: i32, c: u8) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.pixels[y as usize * self.width + x as usize] = c;
        }
    }

    fn fill(&mut self, x: usize, y: usize, w: usize, h: usize, c: u8) {
        for yy in y..(y + h).min(self.height) {
            for xx in x..(x + w).min(self.width) {
                self.pixels[yy * self.width + xx] = c;
            }
        }
    }

    pub fn to_rgb24(&self, palette: &[[u8; 3]; 256], out: &mut [u8]) {
        for (o, &i) in out.as_chunks_mut::<3>().0.iter_mut().zip(&self.pixels) {
            o.copy_from_slice(&palette[i as usize]);
        }
    }
}

/// The game area onto the screen, each row shifted by the ripple's table
/// where the level ripples. A row shifted right leaves its first pixels
/// as they were on the screen (the port: unshifted); one shifted left
/// reads past the area's right edge, in the ring buffer.
fn copy_area(c: &mut Canvas, ring: &Canvas, game: &Game) {
    let mut level = game.level.clone();
    let table = if ripple::active(&level, game.prefs.scroll, false) { ripple::sine_table(&mut level) } else { Vec::new() };
    for y in 0..AREA_SIZE {
        let s = i32::from(ripple::row_shift(&table, game.view.corner.0, game.session.ripple_phase, y));
        let src = &ring.pixels[y as usize * ring.width..];
        let dst = &mut c.pixels[(AREA + y) as usize * c.width + AREA as usize..][..AREA_SIZE as usize];
        for (x, d) in dst.iter_mut().enumerate() {
            let sx = x as i32 - s;
            *d = src[if (0..RING).contains(&sx) { sx as usize } else { x }];
        }
    }
}

/// A blit's colour mode and table index (`None`: the face's own colours).
type Shade = Option<(i16, i16)>;

/// The colour tables, built once from the screen's colour table (tint 10's
/// random colours from a fixed seed; the original's come from the clock).
fn tables() -> &'static Tables {
    static TABLES: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let clut = clut8();
        let itable = tint::inverse_table(&clut);
        let mut rng = Random::new(1);
        Tables::build(&clut, &mut |rgb| tint::color2index(&itable, rgb), &mut || rng.below(5000) as u16)
    })
}

/// How `BlitEncFace` draws in `mode` with `arg`: from 0x20 on, scaled
/// (arg >> 5) with the table index in the low 5 bits, untinted when that
/// is 0; mode 1 with −1 is translucent.
fn shade_of(blit: Option<&Blit>) -> Shade {
    let b = blit?;
    let index = if b.arg >= 0x20 { b.arg & 0x1f } else { b.arg };
    match b.mode {
        0 => None,
        _ if b.arg >= 0x20 && index == 0 => None,
        1 if index == -1 => Some((2, 0)),
        m => Some((m, index)),
    }
}

fn draw_sprite(c: &mut Canvas, data: &GameData, view: (i16, i16), s: &Sprite, blit: Option<&Blit>) {
    let Some(face) = data.faces.get(&s.face) else { return };
    let img = &face.image;
    let shade = shade_of(blit);
    if s.scale[0] == 0x100 && s.scale[1] == 0x100 {
        c.blit(view, img, (0, 0, img.width, img.height), (i32::from(s.at.1), i32::from(s.at.0)), shade);
    } else {
        let (v, h) = scaled_pos(s.at, face.bounds, s.scale[0], s.scale[1]);
        let w = (i32::from(s.scale[0]) * img.width as i32) >> 8;
        let hh = (i32::from(s.scale[1]) * img.height as i32) >> 8;
        c.blit_scaled(view, img, (w.max(0) as usize, hh.max(0) as usize), (i32::from(h), i32::from(v)), shade);
    }
}

/// The frame of a game in progress.
pub fn draw(data: &GameData, game: &Game) -> Canvas {
    let mut c = Canvas::new(SCREEN_WIDTH, SCREEN_HEIGHT, BLACK);
    if let Some(screen) = data.pictures.get(&129) {
        c.draw_image(screen, (0, 0, screen.width, screen.height), 0, 0);
    }
    c.fill(AREA as usize, AREA as usize, AREA_SIZE as usize, AREA_SIZE as usize, BLACK);
    let Some(ring) = draw_area(data, game) else { return c };
    copy_area(&mut c, &ring, game);
    // the meters (`UpdateMeters`): the empty strip of PICT 150, then the
    // value's strip up to the shown value (3 px shorter above 8, where a
    // rounded end - cicn 10135-10137, not drawn yet - closes it).
    // Surface resistance is drawn inverted.
    if let Some(m) = data.pictures.get(&150) {
        let g = &game.g;
        for (row, y, v) in [(1, 386, g.comfort_shown), (2, 416, 144 - g.surface_shown), (3, 446, g.favor_shown)] {
            c.draw_image(m, (0, 0, 147, 11), 473, y);
            let v = v.clamp(0, 144) as usize;
            let len = if v > 8 { v - 3 } else { v };
            c.draw_image(m, (0, row * 11, len, 11), 473, y);
        }
    }
    c
}

/// The game area as the ring buffer holds it: tiles, props and sprites,
/// 480 px wide from the view's corner. None without the level's tile set.
fn draw_area(data: &GameData, game: &Game) -> Option<Canvas> {
    let view = game.view.corner;
    let level = &game.level;
    let tiles = data.tile_set(level)?;
    let mut c = Canvas::new(RING as usize, AREA_SIZE as usize, BLACK);
    c.area = (0, 0, RING, AREA_SIZE);
    let (x0, y0) = ((i32::from(view.1) >> 5).max(0), (i32::from(view.0) >> 5).max(0));
    let (x1, y1) = ((x0 + AREA_SIZE / 32 + 1).min(SIZE as i32 - 1), (y0 + AREA_SIZE / 32 + 1).min(SIZE as i32 - 1));
    for x in x0..=x1 {
        for y in y0..=y1 {
            let b = level.bg_tile(x, y) as usize;
            if b < 96 {
                c.blit(view, &tiles.background, (b % 8 * 32, b / 8 * 32, 32, 32), (x * 32, y * 32), None);
            }
        }
    }
    let w = &game.world;
    let prop = |c: &mut Canvas, p: &harry_core::props::Prop| {
        if let Some(f) = data.faces.get(&p.face) {
            let b = tint::prop_blit(p.face, level);
            c.blit(view, &f.image, (0, 0, f.image.width, f.image.height), (i32::from(p.at.1), i32::from(p.at.0)), shade_of(Some(&b)));
        }
    };
    for p in &w.bg_props.slots {
        prop(&mut c, p);
    }
    // each sprite in the colours of its draw proc's (first) blit
    let sprites: Vec<(&Sprite, Option<&Blit>)> = w
        .sprites
        .list
        .iter()
        .filter(|&&i| w.rec(i).task.is_some())
        .map(|&i| (w.rec(i), game.blits.iter().find(|b| b.0 == Some(i)).map(|b| &b.1)))
        .collect();
    for (s, b) in sprites.iter().filter(|s| s.0.b8b) {
        draw_sprite(&mut c, data, view, s, *b);
    }
    for p in &w.fg_props.slots {
        prop(&mut c, p);
    }
    for x in x0..=x1 {
        for y in y0..=y1 {
            let f = level.fg_tile(x, y);
            if f != NO_TILE && f < 64 {
                let f = f as usize;
                c.blit(view, &tiles.foreground, (f % 8 * 32, f / 8 * 32, 32, 32), (x * 32, y * 32), None);
            }
        }
    }
    for (s, b) in sprites.iter().filter(|s| !s.0.b8b) {
        draw_sprite(&mut c, data, view, s, *b);
    }
    Some(c)
}

/// A 24-bit BMP of RGB pixels (for screenshots).
pub fn write_bmp(path: &std::path::Path, width: usize, height: usize, rgb: &[u8]) -> std::io::Result<()> {
    let row = (width * 3).div_ceil(4) * 4;
    let size = 54 + row * height;
    let mut b = Vec::with_capacity(size);
    b.extend_from_slice(b"BM");
    b.extend_from_slice(&(size as u32).to_le_bytes());
    b.extend_from_slice(&[0; 4]);
    b.extend_from_slice(&54u32.to_le_bytes());
    b.extend_from_slice(&40u32.to_le_bytes());
    b.extend_from_slice(&(width as i32).to_le_bytes());
    b.extend_from_slice(&(height as i32).to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&24u16.to_le_bytes());
    b.extend_from_slice(&[0; 24]);
    for y in (0..height).rev() {
        for x in 0..width {
            let p = &rgb[(y * width + x) * 3..][..3];
            b.extend_from_slice(&[p[2], p[1], p[0]]);
        }
        b.resize(b.len() + row - width * 3, 0);
    }
    std::fs::write(path, b)
}
