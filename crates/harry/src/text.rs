//! Text in the original's Mac fonts as far as the system has them: Geneva
//! (balloons) and Helvetica (the panel), 9 point - on the Mac's 72 dpi
//! screen an em of 9 pixels of the 640 × 480 picture. Text is drawn at the
//! window's resolution on top of the picture.

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Geneva,
    Helvetica,
    HelveticaBold,
}

/// A line of text on the 640 × 480 screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub x: f32,
    /// The baseline (QuickDraw's `MoveTo` before `DrawString`).
    pub baseline: f32,
    /// Point size (= em in screen pixels).
    pub size: f32,
    pub face: Face,
    pub color: [u8; 3],
    pub text: String,
}

pub struct Fonts {
    geneva: Option<FontVec>,
    helvetica: Option<FontVec>,
    helvetica_bold: Option<FontVec>,
}

fn find(families: &[&str], weight: fontdb::Weight) -> Option<FontVec> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut fam: Vec<fontdb::Family> = families.iter().map(|&f| fontdb::Family::Name(f)).collect();
    fam.push(fontdb::Family::SansSerif);
    let id = db.query(&fontdb::Query { families: &fam, weight, ..Default::default() })?;
    db.with_face_data(id, |data, index| FontVec::try_from_vec_and_index(data.to_vec(), index).ok()).flatten()
}

/// The scale that makes the em `size` pixels.
fn scale(font: &FontVec, size: f32) -> PxScale {
    let em = font.units_per_em().unwrap_or(1000.0);
    PxScale::from(size * font.height_unscaled() / em)
}

impl Fonts {
    /// Find the fonts; text in a missing one is not shown.
    pub fn load() -> Fonts {
        let fonts = Fonts {
            geneva: find(&["Geneva", "Verdana", "DejaVu Sans"], fontdb::Weight::NORMAL),
            helvetica: find(&["Helvetica", "Arial", "Liberation Sans"], fontdb::Weight::NORMAL),
            helvetica_bold: find(&["Helvetica", "Arial", "Liberation Sans"], fontdb::Weight::BOLD),
        };
        if fonts.geneva.is_none() || fonts.helvetica.is_none() {
            eprintln!("some fonts are missing: texts may not be shown");
        }
        fonts
    }

    fn font(&self, face: Face) -> Option<&FontVec> {
        match face {
            Face::Geneva => self.geneva.as_ref(),
            Face::Helvetica => self.helvetica.as_ref(),
            Face::HelveticaBold => self.helvetica_bold.as_ref().or(self.helvetica.as_ref()),
        }
    }

    /// The width of a text in screen pixels.
    pub fn width(&self, face: Face, size: f32, text: &str) -> f32 {
        let Some(font) = self.font(face) else { return 0.0 };
        let s = font.as_scaled(scale(font, size));
        let mut w = 0.0;
        let mut last = None;
        for c in text.chars() {
            let id = font.glyph_id(c);
            if let Some(p) = last {
                w += s.kern(p, id);
            }
            w += s.h_advance(id);
            last = Some(id);
        }
        w
    }

    /// Draw the lines into an RGBA image of `width` × `height` pixels that
    /// covers the 640 × 480 screen.
    pub fn draw(&self, lines: &[Line], width: usize, height: usize) -> Vec<u8> {
        let mut rgba = vec![0u8; width * height * 4];
        let k = width as f32 / 640.0;
        for line in lines {
            let Some(font) = self.font(line.face) else { continue };
            let sc = scale(font, line.size * k);
            let scaled = font.as_scaled(sc);
            let mut caret = line.x * k;
            let baseline = line.baseline * k;
            let mut last = None;
            for c in line.text.chars() {
                let id = font.glyph_id(c);
                if let Some(p) = last {
                    caret += scaled.kern(p, id);
                }
                let glyph = id.with_scale_and_position(sc, ab_glyph::point(caret, baseline));
                caret += scaled.h_advance(id);
                last = Some(id);
                let Some(outline) = font.outline_glyph(glyph) else { continue };
                let b = outline.px_bounds();
                outline.draw(|gx, gy, cov| {
                    let x = b.min.x as i32 + gx as i32;
                    let y = b.min.y as i32 + gy as i32;
                    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                        return;
                    }
                    let at = (y as usize * width + x as usize) * 4;
                    let a = (cov.clamp(0.0, 1.0) * 255.0) as u8;
                    if a > rgba[at + 3] {
                        rgba[at..at + 4].copy_from_slice(&[line.color[0], line.color[1], line.color[2], a]);
                    }
                });
            }
        }
        rgba
    }
}
