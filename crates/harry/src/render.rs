//! Software rendering into screen palette indices (`clut` 8), as the
//! original draws a frame: the background tiles, the background props,
//! the sprites with +0x8b set (`DrawSpritesBehindTiles`, which also draws
//! the foreground props), the foreground tiles (`RedrawFGTiles`), then the
//! other sprites (`DrawSpritesAboveTiles`) - each in SAT's list order, at
//! their `at`, scaled when their scale is not 1.0.
//!
//! Not drawn yet: the sprites' tints (draw procs), the ripple, the panel's
//! texts (level name, score) and the meters' rounded ends.

use harry_core::game::Game;
use harry_core::level::{NO_TILE, SIZE};
use harry_core::shots::scaled_pos;
use harry_core::sprite::Sprite;
use harry_data::{GameData, Image};

pub const SCREEN_WIDTH: usize = 640;
pub const SCREEN_HEIGHT: usize = 480;
/// The game area: 448 × 448 at (16, 16).
const AREA: i32 = 16;
const AREA_SIZE: i32 = 448;
const BLACK: u8 = 255;

/// An image of palette indices.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Canvas {
    pub fn new(width: usize, height: usize, fill: u8) -> Canvas {
        Canvas { width, height, pixels: vec![fill; width * height] }
    }

    /// Set a pixel of the game area at level position (h, v).
    fn plot(&mut self, view: (i16, i16), h: i32, v: i32, c: u8) {
        let x = h - i32::from(view.1);
        let y = v - i32::from(view.0);
        if (0..AREA_SIZE).contains(&x) && (0..AREA_SIZE).contains(&y) {
            self.pixels[(AREA + y) as usize * self.width + (AREA + x) as usize] = c;
        }
    }

    /// Draw part of an image (`src` = (x, y, w, h)) at level position (h, v).
    fn blit(&mut self, view: (i16, i16), img: &Image, src: (usize, usize, usize, usize), at: (i32, i32)) {
        let (sx, sy, w, h) = src;
        for y in 0..h {
            for x in 0..w {
                if let Some(c) = img.get(sx + x, sy + y) {
                    self.plot(view, at.0 + x as i32, at.1 + y as i32, c);
                }
            }
        }
    }

    /// Draw an image scaled to `size` (w, h) at level position (h, v).
    fn blit_scaled(&mut self, view: (i16, i16), img: &Image, size: (usize, usize), at: (i32, i32)) {
        if size.0 == 0 || size.1 == 0 {
            return;
        }
        for y in 0..size.1 {
            for x in 0..size.0 {
                if let Some(c) = img.get(x * img.width / size.0, y * img.height / size.1) {
                    self.plot(view, at.0 + x as i32, at.1 + y as i32, c);
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

fn draw_sprite(c: &mut Canvas, data: &GameData, view: (i16, i16), s: &Sprite) {
    let Some(face) = data.faces.get(&s.face) else { return };
    let img = &face.image;
    if s.scale[0] == 0x100 && s.scale[1] == 0x100 {
        c.blit(view, img, (0, 0, img.width, img.height), (i32::from(s.at.1), i32::from(s.at.0)));
    } else {
        let (v, h) = scaled_pos(s.at, face.bounds, s.scale[0], s.scale[1]);
        let w = (i32::from(s.scale[0]) * img.width as i32) >> 8;
        let hh = (i32::from(s.scale[1]) * img.height as i32) >> 8;
        c.blit_scaled(view, img, (w.max(0) as usize, hh.max(0) as usize), (i32::from(h), i32::from(v)));
    }
}

/// The frame of a game in progress.
pub fn draw(data: &GameData, game: &Game) -> Canvas {
    let mut c = Canvas::new(SCREEN_WIDTH, SCREEN_HEIGHT, BLACK);
    if let Some(screen) = data.pictures.get(&129) {
        c.draw_image(screen, (0, 0, screen.width, screen.height), 0, 0);
    }
    c.fill(AREA as usize, AREA as usize, AREA_SIZE as usize, AREA_SIZE as usize, BLACK);
    let view = game.view.corner;
    let level = &game.level;
    let Some(tiles) = data.tile_set(level) else { return c };
    let (x0, y0) = ((i32::from(view.1) >> 5).max(0), (i32::from(view.0) >> 5).max(0));
    let (x1, y1) = ((x0 + AREA_SIZE / 32 + 1).min(SIZE as i32 - 1), (y0 + AREA_SIZE / 32 + 1).min(SIZE as i32 - 1));
    for x in x0..=x1 {
        for y in y0..=y1 {
            let b = level.bg_tile(x, y) as usize;
            if b < 96 {
                c.blit(view, &tiles.background, (b % 8 * 32, b / 8 * 32, 32, 32), (x * 32, y * 32));
            }
        }
    }
    let w = &game.world;
    let prop = |c: &mut Canvas, p: &harry_core::props::Prop| {
        if let Some(f) = data.faces.get(&p.face) {
            c.blit(view, &f.image, (0, 0, f.image.width, f.image.height), (i32::from(p.at.1), i32::from(p.at.0)));
        }
    };
    for p in &w.bg_props.slots {
        prop(&mut c, p);
    }
    let sprites: Vec<&Sprite> = w.sprites.list.iter().map(|&i| w.rec(i)).filter(|s| s.task.is_some()).collect();
    for s in sprites.iter().filter(|s| s.b8b) {
        draw_sprite(&mut c, data, view, s);
    }
    for p in &w.fg_props.slots {
        prop(&mut c, p);
    }
    for x in x0..=x1 {
        for y in y0..=y1 {
            let f = level.fg_tile(x, y);
            if f != NO_TILE && f < 64 {
                let f = f as usize;
                c.blit(view, &tiles.foreground, (f % 8 * 32, f / 8 * 32, 32, 32), (x * 32, y * 32));
            }
        }
    }
    for s in sprites.iter().filter(|s| !s.b8b) {
        draw_sprite(&mut c, data, view, s);
    }
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
