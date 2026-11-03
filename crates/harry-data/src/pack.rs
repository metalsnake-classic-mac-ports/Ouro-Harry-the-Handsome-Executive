//! The data pack: a zip file of ordinary files, made by
//! `ouro-harry-extract` from the original and read by the game. It holds
//! no original resource data, only what the game uses, converted:
//!
//! - `pack.txt`: [`FORMAT`] on the first line
//! - `faces.txt`: per face `id top left bottom right` and the second rect;
//!   `faces/ID.png` its picture (RGBA, colours of the screen palette `clut`
//!   8, transparent pixels with alpha 0)
//! - `tiles.txt`: per tile set `foreground background` (PICT ids);
//!   `tiles/fg_ID.png` (256 × 256, transparent where the mask is clear) and
//!   `tiles/bg_ID.png` (256 × 384)
//! - `pictures/ID.png`: pictures (RGBA, opaque)
//! - `icons/ID.png`: colour icons (RGBA, transparent outside the mask)
//! - `world/ID.png`: the world's pictures (RGBA, opaque);
//!   `world.txt`: the world's fields (`WorldInfo`), `name value` per line
//! - `masks/cicn_ID.png`, `masks/walls.png`: collision masks (grey, white
//!   = set)
//! - `levels/NN.txt`: the levels of Episode 1 (`level`)
//! - `words/ID.txt`: the word lists of the memo and phone sentences
//!   (`STR#` 6900–6905, 6800–6801), one string per line (UTF-8)
//! - `sounds/ID.wav`, `world_sounds/ID.wav`: the sounds (8-bit mono WAV)
//! - `messages.txt`: the application's balloon texts, as `texts.txt`
//! - `texts.txt`: the world's texts, `n text` per line (UTF-8; a line
//!   break inside a text is `\r`, a backslash `\\`)

use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read, Write};

use harry_core::mask::Mask;
use zip::write::SimpleFileOptions;

use crate::{Face, GameData, Image, MaskImages, Result, Sound, TileSet, WorldInfo, clut8, corrupt, level};

/// The first line of `pack.txt`.
pub const FORMAT: &str = "Harry the Handsome Executive data pack 6";

fn encode_png(width: usize, height: usize, color: png::ColorType, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, width as u32, height as u32);
        e.set_color(color);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().expect("PNG header");
        w.write_image_data(data).expect("PNG data");
    }
    out
}

/// (width, height, RGBA)
fn decode_png(data: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
    let mut d = png::Decoder::new(Cursor::new(data));
    d.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut r = d.read_info().map_err(|e| corrupt(format!("PNG: {e}")))?;
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).map_err(|e| corrupt(format!("PNG: {e}")))?;
    let n = info.color_type.samples();
    let mut rgba = Vec::with_capacity(info.width as usize * info.height as usize * 4);
    for px in buf[..info.buffer_size()].chunks_exact(n) {
        rgba.extend(match n {
            1 => [px[0], px[0], px[0], 255],
            2 => [px[0], px[0], px[0], px[1]],
            3 => [px[0], px[1], px[2], 255],
            _ => [px[0], px[1], px[2], px[3]],
        });
    }
    Ok((info.width as usize, info.height as usize, rgba))
}

fn image_png(img: &Image) -> Vec<u8> {
    let pal = clut8();
    let rgba: Vec<u8> = img
        .pixels
        .iter()
        .flat_map(|p| match p {
            Some(i) => {
                let c = pal[*i as usize];
                [c[0], c[1], c[2], 255]
            }
            None => [0, 0, 0, 0],
        })
        .collect();
    encode_png(img.width, img.height, png::ColorType::Rgba, &rgba)
}

fn png_image(data: &[u8], colours: &HashMap<[u8; 3], u8>) -> Result<Image> {
    let (width, height, rgba) = decode_png(data)?;
    let pixels = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| {
            if p[3] < 128 {
                return Ok(None);
            }
            colours.get(&[p[0], p[1], p[2]]).map(|&i| Some(i)).ok_or_else(|| corrupt(format!("colour {:?} is not in the palette", &p[..3])))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Image { width, height, pixels })
}

fn mask_png(m: &Mask) -> Vec<u8> {
    let data: Vec<u8> = (0..m.height).flat_map(|y| (0..m.width).map(move |x| (x, y))).map(|(x, y)| if m.get(x, y) { 255 } else { 0 }).collect();
    encode_png(m.width as usize, m.height as usize, png::ColorType::Grayscale, &data)
}

fn png_mask(data: &[u8]) -> Result<Mask> {
    let (w, h, rgba) = decode_png(data)?;
    Ok(Mask::new(w as i32, h as i32, rgba.as_chunks::<4>().0.iter().map(|p| p[0] >= 128).collect()))
}

/// The pack of `data` (the bytes of the zip file).
pub fn write(data: &GameData) -> Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut put = |name: &str, bytes: &[u8]| -> Result<()> {
        zip.start_file(name, SimpleFileOptions::default()).map_err(|e| corrupt(e.to_string()))?;
        zip.write_all(bytes).map_err(|e| corrupt(e.to_string()))
    };
    put("pack.txt", format!("{FORMAT}\n").as_bytes())?;
    let mut faces = String::new();
    for (id, f) in &data.faces {
        let (b, r) = (f.bounds, f.rect2);
        faces.push_str(&format!("{id} {} {} {} {} {} {} {} {}\n", b[0], b[1], b[2], b[3], r[0], r[1], r[2], r[3]));
        put(&format!("faces/{id}.png"), &image_png(&f.image))?;
    }
    put("faces.txt", faces.as_bytes())?;
    let mut tiles = String::new();
    for ((fg, bg), t) in &data.tile_sets {
        tiles.push_str(&format!("{fg} {bg}\n"));
        put(&format!("tiles/fg_{fg}.png"), &image_png(&t.foreground))?;
        put(&format!("tiles/bg_{bg}.png"), &image_png(&t.background))?;
    }
    put("tiles.txt", tiles.as_bytes())?;
    for (id, p) in &data.pictures {
        put(&format!("pictures/{id}.png"), &image_png(p))?;
    }
    for (id, p) in &data.world_pictures {
        put(&format!("world/{id}.png"), &image_png(p))?;
    }
    let w = &data.world;
    let world = format!(
        "scroll_music {}\nwin_scroll {}\nwin_show {}\nwin_seconds {}\nwin_music {}\n",
        w.scroll_music, w.win_scroll, w.win_show, w.win_seconds, w.win_music
    );
    put("world.txt", world.as_bytes())?;
    for (id, p) in &data.icons {
        put(&format!("icons/{id}.png"), &image_png(p))?;
    }
    for (id, m) in &data.masks.cicns {
        put(&format!("masks/cicn_{id}.png"), &mask_png(m))?;
    }
    // the 32 wall shapes as one sheet, 8 per row
    let mut sheet = vec![false; 256 * 128];
    for (k, m) in data.masks.walls.iter().enumerate() {
        for y in 0..32 {
            for x in 0..32 {
                sheet[(k / 8 * 32 + y) * 256 + k % 8 * 32 + x] = m.get(x as i32, y as i32);
            }
        }
    }
    put("masks/walls.png", &mask_png(&Mask::new(256, 128, sheet)))?;
    for (i, l) in data.levels.iter().enumerate() {
        put(&format!("levels/{:02}.txt", i + 1), level::write(l).as_bytes())?;
    }
    put("texts.txt", texts_file(&data.texts).as_bytes())?;
    put("messages.txt", texts_file(&data.messages).as_bytes())?;
    for (id, list) in word_lists(&data.words) {
        let lines: Vec<String> = list.iter().map(|w| harry_core::text::mac_roman(w)).collect();
        put(&format!("words/{id}.txt"), format!("{}\n", lines.join("\n")).as_bytes())?;
    }
    for (dir, set) in [("sounds", &data.sounds), ("world_sounds", &data.world_sounds)] {
        for (id, snd) in set {
            put(&format!("{dir}/{id}.wav"), &wav(snd))?;
        }
    }
    let cursor = zip.finish().map_err(|e| corrupt(e.to_string()))?;
    Ok(cursor.into_inner())
}

/// Whether these bytes look like a zip file.
pub fn is_pack(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04")
}

/// Read a pack (the bytes of the zip file).
pub fn read(bytes: &[u8]) -> Result<GameData> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| corrupt(e.to_string()))?;
    let mut get = |name: &str| -> Result<Vec<u8>> {
        let mut f = zip.by_name(name).map_err(|_| corrupt(format!("{name} missing")))?;
        let mut d = Vec::new();
        f.read_to_end(&mut d).map_err(|e| corrupt(format!("{name}: {e}")))?;
        Ok(d)
    };
    let text = |d: Vec<u8>| String::from_utf8_lossy(&d).into_owned();
    if !text(get("pack.txt")?).starts_with(FORMAT) {
        return Err(corrupt(format!("pack.txt does not start with {FORMAT:?}")));
    }
    let colours: HashMap<[u8; 3], u8> = clut8().iter().enumerate().map(|(i, c)| (*c, i as u8)).collect();
    let mut faces = BTreeMap::new();
    for line in text(get("faces.txt")?).lines() {
        let v: Vec<i16> = line.split_whitespace().map(|w| w.parse().map_err(|_| corrupt("faces.txt"))).collect::<Result<_>>()?;
        if v.len() != 9 {
            return Err(corrupt("faces.txt: 9 numbers per line"));
        }
        let image = png_image(&get(&format!("faces/{}.png", v[0]))?, &colours)?;
        faces.insert(v[0], Face { bounds: [v[1], v[2], v[3], v[4]], rect2: [v[5], v[6], v[7], v[8]], image });
    }
    let mut tile_sets = BTreeMap::new();
    for line in text(get("tiles.txt")?).lines() {
        let v: Vec<i16> = line.split_whitespace().map(|w| w.parse().map_err(|_| corrupt("tiles.txt"))).collect::<Result<_>>()?;
        if v.len() != 2 {
            return Err(corrupt("tiles.txt: 2 numbers per line"));
        }
        let foreground = png_image(&get(&format!("tiles/fg_{}.png", v[0]))?, &colours)?;
        let background = png_image(&get(&format!("tiles/bg_{}.png", v[1]))?, &colours)?;
        tile_sets.insert((v[0], v[1]), TileSet { foreground, background });
    }
    let mut pictures = BTreeMap::new();
    for id in crate::PICTURES {
        pictures.insert(id, png_image(&get(&format!("pictures/{id}.png"))?, &colours)?);
    }
    let mut world = WorldInfo::default();
    for line in text(get("world.txt")?).lines() {
        let (k, v) = line.split_once(' ').ok_or_else(|| corrupt("world.txt"))?;
        let v: i16 = v.trim().parse().map_err(|_| corrupt("world.txt"))?;
        match k {
            "scroll_music" => world.scroll_music = v,
            "win_scroll" => world.win_scroll = v,
            "win_show" => world.win_show = v,
            "win_seconds" => world.win_seconds = v,
            "win_music" => world.win_music = v,
            _ => {}
        }
    }
    let mut icons = BTreeMap::new();
    for id in crate::ICONS {
        icons.insert(id, png_image(&get(&format!("icons/{id}.png"))?, &colours)?);
    }
    let mut cicns = BTreeMap::new();
    for id in [290, 299, 571, 572, 573, 574, 598, 599, 1299] {
        cicns.insert(id, png_mask(&get(&format!("masks/cicn_{id}.png"))?)?);
    }
    let sheet = png_mask(&get("masks/walls.png")?)?;
    let walls = (0..32)
        .map(|k| {
            let bits = (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).map(|(x, y)| sheet.get(k % 8 * 32 + x, k / 8 * 32 + y)).collect();
            Mask::new(32, 32, bits)
        })
        .collect();
    let mut levels = Vec::new();
    for i in 1..=15 {
        levels.push(level::parse(&text(get(&format!("levels/{i:02}.txt"))?))?);
    }
    let mut world_pictures = BTreeMap::new();
    for id in world.picture_ids(&levels) {
        world_pictures.insert(id, png_image(&get(&format!("world/{id}.png"))?, &colours)?);
    }
    let texts = read_texts(&text(get("texts.txt")?), "texts.txt")?;
    let messages = read_texts(&text(get("messages.txt")?), "messages.txt")?;
    let mut words = harry_core::words::Words::default();
    for (k, id) in (6900..6906).chain(6800..6802).enumerate() {
        let list: Vec<Vec<u8>> = text(get(&format!("words/{id}.txt"))?).lines().map(harry_core::text::to_mac_roman).collect();
        if k < 6 {
            words.buzz[k] = list;
        } else {
            words.phone[k - 6] = list;
        }
    }
    let names: Vec<String> = zip.file_names().filter_map(|n| n.ok().map(|n| n.into_owned())).collect();
    let mut sound_set = |dir: &str| -> Result<BTreeMap<i16, Sound>> {
        let mut set = BTreeMap::new();
        for name in names.iter().filter(|n| n.starts_with(&format!("{dir}/")) && n.ends_with(".wav")) {
            let id: i16 = name[dir.len() + 1..name.len() - 4].parse().map_err(|_| corrupt(format!("{name}: bad name")))?;
            let mut f = zip.by_name(name).map_err(|e| corrupt(e.to_string()))?;
            let mut d = Vec::new();
            f.read_to_end(&mut d).map_err(|e| corrupt(format!("{name}: {e}")))?;
            set.insert(id, unwav(&d).ok_or_else(|| corrupt(format!("{name}: not an 8-bit mono WAV")))?);
        }
        Ok(set)
    };
    let sounds = sound_set("sounds")?;
    let world_sounds = sound_set("world_sounds")?;
    Ok(GameData { faces, tile_sets, masks: MaskImages { cicns, walls }, pictures, world_pictures, world, icons, levels, texts, messages, words, sounds, world_sounds })
}

fn word_lists(w: &harry_core::words::Words) -> Vec<(i16, &Vec<Vec<u8>>)> {
    let mut v: Vec<(i16, &Vec<Vec<u8>>)> = w.buzz.iter().enumerate().map(|(i, l)| (6900 + i as i16, l)).collect();
    v.extend(w.phone.iter().enumerate().map(|(i, l)| (6800 + i as i16, l)));
    v
}

/// An 8-bit mono WAV file.
fn wav(s: &Sound) -> Vec<u8> {
    let n = s.samples.len() as u32;
    let mut w = Vec::with_capacity(44 + s.samples.len());
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + n).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&s.rate.to_le_bytes());
    w.extend_from_slice(&s.rate.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&8u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&n.to_le_bytes());
    w.extend_from_slice(&s.samples);
    w
}

/// Read an 8-bit mono WAV file.
fn unwav(d: &[u8]) -> Option<Sound> {
    if d.get(..4)? != b"RIFF" || d.get(8..12)? != b"WAVE" {
        return None;
    }
    let le16 = |o: usize| Some(u16::from_le_bytes(d.get(o..o + 2)?.try_into().ok()?));
    let le32 = |o: usize| Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?));
    let mut p = 12;
    let mut rate = None;
    while p + 8 <= d.len() {
        let size = le32(p + 4)? as usize;
        match d.get(p..p + 4)? {
            b"fmt " => {
                if le16(p + 8)? != 1 || le16(p + 10)? != 1 || le16(p + 22)? != 8 {
                    return None;
                }
                rate = le32(p + 12);
            }
            b"data" => return Some(Sound { rate: rate?, samples: d.get(p + 8..p + 8 + size)?.to_vec() }),
            _ => {}
        }
        p += 8 + size + (size & 1);
    }
    None
}

/// Texts by number, `n text` per line (UTF-8; a line break inside a text
/// is `\r`, a backslash `\\`).
fn texts_file(texts: &BTreeMap<i16, Vec<u8>>) -> String {
    let mut out = String::new();
    for (n, t) in texts {
        let t = harry_core::text::mac_roman(t).replace('\\', "\\\\").replace('\r', "\\r");
        out.push_str(&format!("{n} {t}\n"));
    }
    out
}

fn read_texts(file: &str, name: &str) -> Result<BTreeMap<i16, Vec<u8>>> {
    let mut texts = BTreeMap::new();
    for line in file.lines() {
        let (n, t) = line.split_once(' ').ok_or_else(|| corrupt(name))?;
        let n: i16 = n.parse().map_err(|_| corrupt(name))?;
        let mut s = String::new();
        let mut esc = false;
        for c in t.chars() {
            match (esc, c) {
                (false, '\\') => esc = true,
                (true, 'r') => {
                    s.push('\r');
                    esc = false;
                }
                (true, c) => {
                    s.push(c);
                    esc = false;
                }
                (false, c) => s.push(c),
            }
        }
        texts.insert(n, harry_core::text::to_mac_roman(&s));
    }
    Ok(texts)
}
