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
//! Second faces of draw procs (a fan's grille, a dart enemy's loaded dart)
//! are drawn over the sprite whole; the original clips them to the part
//! of the sprite being drawn, which differs only at the ring buffer's
//! wrap.

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
#[derive(Clone)]
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
    pub fn draw_image(&mut self, img: &Image, src: (usize, usize, usize, usize), x: usize, y: usize) {
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

    /// Draw an image with transparency at screen (x, y), clipped.
    pub fn draw_icon(&mut self, img: &Image, x: i32, y: i32) {
        for yy in 0..img.height {
            for xx in 0..img.width {
                if let Some(c) = img.get(xx, yy) {
                    self.set(x + xx as i32, y + yy as i32, c);
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

    /// A whole image at screen (x, y) (see `draw_image`).
    pub fn draw_picture(&mut self, img: &Image, x: usize, y: usize) {
        self.draw_image(img, (0, 0, img.width, img.height), x, y);
    }

    /// Fill the rectangle at (x, y), `w` × `h`, clipped to the screen.
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        let (x0, y0) = (x.max(0), y.max(0));
        let (x1, y1) = ((x + w).min(self.width as i32), (y + h).min(self.height as i32));
        for yy in y0..y1 {
            for xx in x0..x1 {
                self.pixels[yy as usize * self.width + xx as usize] = c;
            }
        }
    }

    /// `FrameRect`: the 1-px border of (l, t) – (r, b), right and bottom
    /// exclusive, clipped.
    pub fn frame_rect(&mut self, l: i32, t: i32, r: i32, b: i32, c: u8) {
        for x in l..r {
            self.set(x, t, c);
            self.set(x, b - 1, c);
        }
        for y in t..b {
            self.set(l, y, c);
            self.set(r - 1, y, c);
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

fn draw_sprite(c: &mut Canvas, data: &GameData, level: &harry_core::Level, view: (i16, i16), s: &Sprite, blit: Option<&Blit>) {
    let Some(face) = data.faces.get(&s.face) else { return };
    // a fan's grille, a dart enemy's loaded dart
    if let Some(second) = tint::second_face(s, level)
        && let Some(f) = data.faces.get(&second.blit.face)
    {
        draw_face(c, view, face, s, blit);
        let at = (i32::from(s.at.1) + i32::from(second.offset.1), i32::from(s.at.0) + i32::from(second.offset.0));
        c.blit(view, &f.image, (0, 0, f.image.width, f.image.height), at, shade_of(Some(&second.blit)));
        return;
    }
    draw_face(c, view, face, s, blit);
}

fn draw_face(c: &mut Canvas, view: (i16, i16), face: &harry_data::Face, s: &Sprite, blit: Option<&Blit>) {
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
        c.draw_picture(screen, 0, 0);
    }
    c.fill(AREA, AREA, AREA_SIZE, AREA_SIZE, BLACK);
    let Some(ring) = draw_area(data, game) else { return c };
    copy_area(&mut c, &ring, game);
    draw_inventory(&mut c, data, game);
    // the meters as `UpdateMeters` last drew them (`Game::meters`): the
    // empty strip of PICT 150, the value's strip and the rounded end
    if let Some(m) = data.pictures.get(&150) {
        for (k, (y, d)) in [386, 416, 446].into_iter().zip(game.meters).enumerate() {
            c.draw_image(m, (0, 0, 147, 11), 473, y);
            if d.len > 0 {
                c.draw_image(m, (0, (k + 1) * 11, d.len as usize, 11), 473, y);
            }
            if d.end
                && let Some(end) = data.icons.get(&(10135 + k as i16))
            {
                c.draw_icon(end, 473 + i32::from(d.len), y as i32 + 1);
            }
        }
    }
    c
}

/// An inventory slot's top left (`FindCol` / `FindRow`): 3 columns, 4
/// rows, 49 px apart from (476, 167).
pub fn slot_at(slot: usize) -> (i32, i32) {
    (476 + (slot % 3) as i32 * 49, 167 + (slot / 3) as i32 * 49)
}

/// The face of an inventory item (`FindInvItemFace`): the debug items
/// 11398 → HrSp 1031, 11397 → 1032, else 1010 + (id − 11199) / 100.
fn item_face(id: i16) -> i16 {
    match id {
        11398 => 1031,
        11397 => 1032,
        _ => 1010 + (id - 11199) / 100,
    }
}

/// The palette index of a grey (clut 8 has them all as exact colours).
fn grey(v: u8) -> u8 {
    harry_data::clut8().iter().position(|c| *c == [v, v, v]).unwrap_or(BLACK as usize) as u8
}

/// The inventory as `UpdateInventory` draws it: per item its slot (the
/// selected one on the highlight, HrSp 1002 at + 4, the others on black)
/// with the frame (HrSp 1001) and the item's face (at + 4); an empty
/// slot is HrSp 1040. Under a gun its ammunition (`UpdateAmmo`): a bar
/// 33 px long at (+ 5, + 34), the part up to ammo / 3 light, the rest
/// dark, lit from the top left.
fn draw_inventory(c: &mut Canvas, data: &GameData, game: &Game) {
    let g = &game.g;
    let face = |c: &mut Canvas, id: i16, x: i32, y: i32| {
        if let Some(f) = data.faces.get(&id) {
            c.draw_icon(&f.image, x, y);
        }
    };
    for (slot, &(id, _)) in g.inventory.iter().enumerate() {
        let (x, y) = slot_at(slot);
        if id == -1 {
            face(c, 1040, x, y);
            continue;
        }
        if g.cur_slot == slot as i16 {
            face(c, 1002, x + 4, y + 4);
        } else {
            c.fill(x, y, 44, 44, BLACK);
        }
        face(c, 1001, x, y);
        face(c, item_face(id), x + 4, y + 4);
    }
    // the ammunition of the staple gun, B.A.D.A.S.S. and shrapnel gun
    let (fill_l, fill_r, top_l, bottom_l, top_r, bottom_r) = (grey(0xaa), grey(0x55), grey(0xff), grey(0x77), grey(0x11), grey(0x99));
    for (gun, ammo) in [(11299, g.ammo.staple), (11599, g.ammo.badass), (11799, g.ammo.shrapnel)] {
        let Some(slot) = g.inventory.iter().rposition(|e| e.0 == gun) else { continue };
        let (x, y) = slot_at(slot);
        let (x, y) = (x + 5, y + 34);
        let a = ammo.clamp(0, 100);
        let w = i32::from(a / 3);
        let line = |c: &mut Canvas, x0: i32, x1: i32, y: i32, col: u8| (x0..=x1).for_each(|xx| c.set(xx, y, col));
        line(c, x, x + w, y, top_l);
        line(c, x + w, x + 33, y, top_r);
        for yy in y + 1..y + 4 {
            line(c, x, x + w - 1, yy, fill_l);
            line(c, x + w, x + 32, yy, fill_r);
        }
        line(c, x, x + w, y + 4, bottom_l);
        line(c, x + w, x + 33, y + 4, bottom_r);
        let column = |c: &mut Canvas, xx: i32, col: u8| (y..=y + 4).for_each(|yy| c.set(xx, yy, col));
        column(c, x, if a < 1 { top_r } else { top_l });
        if a < 100 {
            column(c, x + 33, bottom_r);
        }
        if a > 0 {
            column(c, x + w, bottom_l);
        }
    }
}

/// The count of an inventory item over its face (`DrawNumItems`): "x" and
/// the number in white, bold 9 pt, when there is more than one.
pub fn inventory_counts(game: &Game) -> Vec<(f32, f32, String)> {
    game.g
        .inventory
        .iter()
        .enumerate()
        .filter(|(_, e)| e.0 != -1 && e.1 > 1)
        .flat_map(|(slot, e)| {
            let (x, y) = slot_at(slot);
            let (h, v) = (x + 4, y + 4);
            [((h + 2) as f32, (v + 34) as f32, "x".to_string()), ((h + 9) as f32, (v + 34) as f32, e.1.to_string())]
        })
        .collect()
}

/// The map (`ShowMap`) over the game area: 112 × 112 tiles of 4 × 4 px.
/// Which part: the middle (from tile 8, 8) while Harry (`gPlayerH/V`)
/// is within 0x500 … 0xb40 on both axes, else the quarter he is in
/// (from 0 or 16 on each axis). The area grey (0x451e); a tile Harry has
/// not come near (the track map) black; a seen one with a foreground tile
/// below 32 its map tile (PICT 170), from 56 on black, else nothing. Harry:
/// cicn 200 at ((H + 32) / 8 − 4 · from, …), its middle on his tile.
pub fn draw_map(c: &mut Canvas, data: &GameData, game: &Game) {
    let grey = harry_data::clut8().iter().position(|p| *p == [0x44, 0x44, 0x44]).unwrap_or(BLACK as usize) as u8;
    c.fill(AREA, AREA, AREA_SIZE, AREA_SIZE, grey);
    let (ph, pv) = (i32::from(game.g.player_pos.0), i32::from(game.g.player_pos.1));
    let middle = (0x500..0xb40).contains(&ph) && (0x500..0xb40).contains(&pv);
    let (oh, ov) = if middle { (8, 8) } else { (if ph < 0x800 { 0 } else { 16 }, if pv < 0x800 { 0 } else { 16 }) };
    let tiles = data.pictures.get(&170);
    for h in oh..oh + 112 {
        for v in ov..ov + 112 {
            let (x, y) = ((h - oh) * 4 + AREA, (v - ov) * 4 + AREA);
            let seen = game.track.cells[(h * 128 + v) as usize] != 0;
            let fg = game.level.fg_tile(h, v) as i16;
            if !seen || fg >= 56 {
                c.fill(x, y, 4, 4, BLACK);
            } else if (0..32).contains(&fg)
                && let Some(t) = tiles
            {
                let (sx, sy) = ((fg % 8) as usize * 4, (fg / 8) as usize * 4);
                for yy in 0..4 {
                    for xx in 0..4 {
                        // white is transparent in an encoded face
                        if let Some(p) = t.get(sx + xx, sy + yy).filter(|&p| p != 0) {
                            c.set(x + xx as i32, y + yy as i32, p);
                        }
                    }
                }
            }
        }
    }
    if let Some(me) = data.icons.get(&200) {
        let (x, y) = ((ph + 32) / 8 - 4 * oh, (pv + 32) / 8 - 4 * ov);
        c.draw_icon(me, x, y);
    }
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
        draw_sprite(&mut c, data, level, view, s, *b);
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
        draw_sprite(&mut c, data, level, view, s, *b);
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
