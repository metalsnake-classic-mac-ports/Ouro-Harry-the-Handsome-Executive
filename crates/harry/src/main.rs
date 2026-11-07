//! Harry the Handsome Executive: the game with SDL3. `ouro-harry [pack.zip]
//! [level]`: the data pack `data/ouro-harry-data.zip` is built in; a pack
//! on the command line, in `HARRY_DATA`, or as `ouro-harry-data.zip` next to
//! the program or in the current folder replaces it. It starts with the
//! main menu, or with a level given at once.
//!
//! The game runs at the original's 30 frames per second, with its sounds. Keys as on the
//! Mac (the default bindings): ← → turn, ↓ scoot, ↑ inch, space kick,
//! control select item, option / alt use item, 1–4 pick a weapon or the
//! swivel. Escape asks before the game is given up (then the main menu;
//! in the menu it quits), F11 (or the key set in the options) toggles
//! full screen; F8 / F7 jump to the
//! next / previous level (for testing).
//!
//! `ouro-harry --screenshot out.bmp [level] [frames]` runs the frames
//! (`HARRY_SAVE=<file>`: from that saved game)
//! without keys and writes the picture, without a window. With
//! `HARRY_STATS` set the game prints its loop timing once a second;
//! `cargo run -p harry --example audio_probe` shows how SDL drives the
//! audio callback.

mod app;
mod audio;
mod balloon;
mod menu;
mod music;
mod options;
mod render;
mod scenes;
mod text;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use harry_core::savegame::SaveGame;
use harry_core::game::{Assets, Balloon, Game, LevelStart, Prefs};
use harry_core::keys::{Bindings, KeyMap};
use harry_core::kinds::LevelCounter;
use harry_core::player::Masks;
use harry_core::random::Random;
use harry_core::view::View;
use harry_core::world::World;
use harry_core::frame::PlayerGlobals;
use harry_data::{GameData, clut8};
use sdl3::event::Event;
use sdl3::keyboard::{KeyboardState, Keycode, Mod, Scancode};
use sdl3::sys::keycode::SDL_KMOD_NONE;
use sdl3::pixels::PixelFormat;
use sdl3::render::ScaleMode;
use sdl3::sys::render::{SDL_LOGICAL_PRESENTATION_INTEGER_SCALE, SDL_LOGICAL_PRESENTATION_LETTERBOX};

/// The original's frame: 2 ticks of 1/60 s.
const FRAME: Duration = Duration::from_micros(33_333);
/// A tick of the Mac's clock (the sequences between frames run in them).
const TICK: Duration = Duration::from_micros(16_667);

/// SDL keys → Mac virtual key codes (the ones the game reads), by
/// position on the keyboard; the letters go by `LETTERS`.
const KEYS: &[(Scancode, i16)] = &[
    (Scancode::Left, 0x7B),
    (Scancode::Right, 0x7C),
    (Scancode::Down, 0x7D),
    (Scancode::Up, 0x7E),
    (Scancode::Space, 0x31),
    (Scancode::Tab, 0x30),
    (Scancode::Return, 0x24),
    (Scancode::LCtrl, 0x3B),
    (Scancode::RCtrl, 0x3B),
    (Scancode::LAlt, 0x3A),
    (Scancode::RAlt, 0x3A),
    (Scancode::LShift, 0x38),
    (Scancode::RShift, 0x38),
    (Scancode::_1, 0x12),
    (Scancode::_2, 0x13),
    (Scancode::_3, 0x14),
    (Scancode::_4, 0x15),
    (Scancode::Equals, 0x18),
    (Scancode::PageUp, 0x74),
    (Scancode::F1, 0x7A),
    (Scancode::F2, 0x78),
    (Scancode::F3, 0x63),
    (Scancode::F4, 0x76),
    (Scancode::F12, 0x6F),
    (Scancode::F13, 0x69),
];

/// The Mac key codes of the letters a–z. The original reads key
/// positions (US layout: on a German keyboard W-A-V-Y-G-R-A-V-Y is typed
/// W-A-V-Z-G-R-A-V-Z); the port goes by the letter the key gives in the
/// current layout.
const LETTERS: [i16; 26] = [
    0x00, 0x0B, 0x08, 0x02, 0x0E, 0x03, 0x05, 0x04, 0x22, 0x26, 0x28, 0x25, 0x2E, 0x2D, 0x1F, 0x23, 0x0C, 0x0F, 0x01,
    0x11, 0x20, 0x09, 0x0D, 0x07, 0x10, 0x06,
];

/// The Mac key code a key gives (positions; letters by the layout), if
/// the game knows one.
fn mac_code(sc: Scancode) -> Option<i16> {
    KEYS.iter().find(|k| k.0 == sc).map(|k| k.1).or_else(|| {
        let key = Keycode::from_scancode(sc, SDL_KMOD_NONE, false).map_or(0, |k| k as u32);
        char::from_u32(key).filter(char::is_ascii_lowercase).map(|c| LETTERS[usize::from(c as u8 - b'a')])
    })
}

/// A Mac key code's name for the options screen.
fn key_name(code: i16) -> String {
    let named = match code {
        0x7B => "Left Arrow",
        0x7C => "Right Arrow",
        0x7D => "Down Arrow",
        0x7E => "Up Arrow",
        0x31 => "Space",
        0x3B => "Control",
        0x3A => "Option / Alt",
        0x38 => "Shift",
        0x30 => "Tab",
        0x24 => "Return",
        _ => "",
    };
    if !named.is_empty() {
        return named.to_string();
    }
    if let Some(i) = LETTERS.iter().position(|&c| c == code) {
        return char::from(b'A' + i as u8).to_string();
    }
    KEYS.iter().find(|k| k.1 == code).map_or(format!("Key {code}"), |k| k.0.name().to_string())
}

fn keymap(state: &KeyboardState) -> KeyMap {
    let mut k = KeyMap::default();
    for &(sc, code) in KEYS {
        if state.is_scancode_pressed(sc) {
            k.press(code);
        }
    }
    for sc in state.pressed_scancodes() {
        let key = Keycode::from_scancode(sc, SDL_KMOD_NONE, false).map_or(0, |k| k as u32);
        if let Some(c) = char::from_u32(key).filter(char::is_ascii_lowercase) {
            k.press(LETTERS[usize::from(c as u8 - b'a')]);
        }
    }
    k
}

/// Now in Mac seconds (since 1904).
fn mac_seconds() -> u32 {
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    (unix + 2_082_844_800) as u32
}

/// The data pack `ouro-harry-extract` makes (`data/ouro-harry-data.zip`),
/// built into the program.
pub(crate) const BUNDLED: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/ouro-harry-data.zip"));
/// A pack of this name next to the program or in the current folder is
/// used instead of the built-in one.
const PACK: &str = "ouro-harry-data.zip";

/// The game data: a pack from the command line, `HARRY_DATA`, next to the
/// program or in the current folder, else the built-in one.
fn load_data(arg: Option<&String>) -> Result<GameData> {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let candidates = arg
        .map(PathBuf::from)
        .into_iter()
        .chain(std::env::var_os("HARRY_DATA").map(PathBuf::from))
        .chain(exe_dir.map(|d| d.join(PACK)))
        .chain([PathBuf::from(PACK)]);
    for path in candidates {
        if arg.is_some_and(|a| PathBuf::from(a) == path) && !path.exists() {
            return Err(anyhow!("{}: not found", path.display()));
        }
        if !path.exists() {
            continue;
        }
        let bytes = std::fs::read(&path).with_context(|| path.display().to_string())?;
        match harry_data::pack::read(&bytes) {
            Ok(data) => return Ok(data),
            Err(e) => eprintln!("{}: {e}", path.display()),
        }
    }
    harry_data::pack::read(BUNDLED).context("the built-in game data")
}

/// The masks the game routines need, from the pack.
struct Loaded {
    player: Masks,
    plant: harry_core::mask::Mask,
    barrel: harry_core::mask::Mask,
    cooler: harry_core::mask::Mask,
    shot: harry_core::mask::Mask,
    ramps: Vec<harry_core::mask::Mask>,
    player_shot: harry_core::mask::Mask,
    robot: harry_core::mask::Mask,
    blob: harry_core::mask::Mask,
    desks: [harry_core::mask::Mask; 2],
}

impl Loaded {
    /// The masks as the game routines take them.
    fn assets<'a>(&'a self, face_bounds: &'a dyn Fn(i16) -> [i16; 4]) -> Assets<'a> {
        Assets {
            player: &self.player,
            plant: &self.plant,
            barrel: &self.barrel,
            cooler: &self.cooler,
            shot: &self.shot,
            ramps: [&self.ramps[0], &self.ramps[1], &self.ramps[2], &self.ramps[3]],
            player_shot: &self.player_shot,
            robot: &self.robot,
            blob: &self.blob,
            desks: [&self.desks[0], &self.desks[1]],
            face_bounds,
        }
    }

    fn new(data: &GameData) -> Result<Loaded> {
        let cicn = |id: i16| data.masks.cicns.get(&id).cloned().with_context(|| format!("mask cicn {id} missing"));
        let face = |id: i16| data.face_mask(id).with_context(|| format!("face {id} missing"));
        Ok(Loaded {
            player: Masks {
                body: cicn(299)?,
                obstacle: cicn(290)?,
                walls: data.masks.walls.clone(),
                faces: (0..144).map(|f| (f >= 107).then(|| data.face_mask(1101 + f as i16)).flatten()).collect(),
            },
            plant: cicn(599)?,
            barrel: cicn(598)?,
            cooler: face(2002)?,
            shot: cicn(1299)?,
            ramps: (571..=574).map(cicn).collect::<Result<_>>()?,
            player_shot: face(4021)?,
            robot: face(4251)?,
            blob: face(1451)?,
            desks: [face(harry_core::desk::MASK_ACROSS)?, face(harry_core::desk::MASK_ALONG)?],
        })
    }
}

fn new_game(data: &GameData, level: usize, g: PlayerGlobals, seed: u32) -> Game {
    let l = data.levels[level].clone();
    Game {
        world: World::default(),
        physics: l.effective_physics(),
        level: l,
        g,
        view: View::default(),
        rng: Random::new(seed),
        prefs: Prefs {
            scroll: harry_core::view::mode::FOLLOW,
            even_v: false,
            blit: false,
            bindings: Bindings { left: 0x7B, right: 0x7C, scoot: 0x7D, inch: 0x7E, kick: 0x31, select_item: 0x3B, use_item: 0x3A },
            autosave: true,
        },
        counter: LevelCounter { current: level as i16 + 1, previous: level as i16 },
        seen_level: level as i16 + 1,
        plant_flag: false,
        robot_flags: (false, false),
        reversor_flags: (false, false),
        blob_flags: (false, false),
        magnet: 2,
        gun: 0,
        harry: None,
        words: data.words.clone(),
        sounds: Vec::new(),
        audio: Default::default(),
        level_start: LevelStart::default(),
        track: Default::default(),
        keep_track: false,
            blits: Vec::new(),
            meters: Default::default(),
        session: Default::default(),
    }
}

const HELP: &str = "\
Ouro Harry: Harry the Handsome Executive (Ben Spees / Ambrosia Software,
1997) for modern systems.

usage: ouro-harry [DATA] [LEVEL]
       ouro-harry --screenshot OUT.bmp [LEVEL] [FRAMES]
       ouro-harry --version | --help

The game data is built in; DATA, a data pack made by ouro-harry-extract,
replaces it (so do HARRY_DATA and ouro-harry-data.zip next to the program
or in the current folder). LEVEL starts that level at once.

Keys: left / right turn, down scoots, up inches, Space kicks, Control
selects an item, Option (Alt) uses it, 1-4 pick a weapon, Tab shows the
map, Caps Lock pauses; Command (or Ctrl) + I opens the options, F11 full
screen, Esc gives up the game. The keys can be changed in the options.";

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().collect();
    if args.iter().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("ouro-harry {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().skip(1).any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let screenshot = (args.get(1).map(String::as_str) == Some("--screenshot")).then(|| {
        let out = args.get(2).cloned().unwrap_or_else(|| "harry.bmp".into());
        let frames = args.get(4).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
        let level = args.get(3).cloned();
        args = vec![args[0].clone(), String::new()];
        if let Some(l) = level {
            args.push(l);
        }
        (out, frames)
    });
    // a level number alone: the built-in pack
    if args.get(1).is_some_and(|a| a.parse::<usize>().is_ok()) {
        args.insert(1, String::new());
    }
    let data = load_data(args.get(1).filter(|s| !s.is_empty()))?;
    let loaded = Loaded::new(&data)?;
    // the settings (the options screen), in SDL's preference folder
    let settings_path = sdl3::filesystem::get_pref_path("MetalSnake", "Ouro Harry").ok().map(|d| d.join("settings.txt"));
    let settings = settings_path.as_deref().map(options::Settings::load).unwrap_or_default();
    let face_bounds = |f: i16| data.faces.get(&f).map_or([0; 4], |x| x.bounds);
    let assets = loaded.assets(&face_bounds);
    // a level on the command line starts it at once, else the main menu
    let direct = args.get(2).is_some();
    let level: usize = args.get(2).and_then(|s| s.parse::<usize>().ok()).map_or(0, |n| n.clamp(1, data.levels.len()) - 1);
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() | 1);
    let game = app::start_game(&data, &assets, level, PlayerGlobals::new_game(), seed);
    let save_path = sdl3::filesystem::get_pref_path("MetalSnake", "Ouro Harry").ok().map(|d| d.join("Harry Save.hrsg"));
    let fonts = text::Fonts::load();
    if let Some((out, frames)) = screenshot {
        return write_screenshot(&data, &assets, &fonts, game, &out, frames);
    }

    let sdl = sdl3::init()?;
    let mixer = audio::Mixer::open(&sdl, &data);
    let video = sdl.video()?;
    let window = video
        .window("Harry the Handsome Executive", 2 * render::SCREEN_WIDTH as u32, 2 * render::SCREEN_HEIGHT as u32)
        .resizable()
        .position_centered()
        .build()?;
    let mut events = sdl.event_pump()?;
    let canvas = window.into_canvas();
    let creator = canvas.texture_creator();
    let mut display = Display::new(canvas, &creator)?;
    display.apply(&settings);
    let mut app = app::App::new(&data, &assets, &fonts, settings, settings_path, save_path, mixer, game, level, direct, Instant::now());
    // HARRY_STATS: loop timing once a second
    let stats = std::env::var_os("HARRY_STATS").is_some();
    let (mut loops, mut busy, mut stat_start) = (0u32, Duration::ZERO, Instant::now());
    loop {
        let loop_start = Instant::now();
        let polled: Vec<Event> = events
            .poll_iter()
            .map(|mut e| {
                e.convert_coords(&display.canvas);
                e
            })
            .collect();
        let input = app::Input { held: keymap(&events.keyboard_state()), caps_lock: sdl.keyboard().mod_state().contains(Mod::CAPSMOD) };
        if !app.step(polled, &input, Instant::now()) {
            break;
        }
        if std::mem::take(&mut app.display_changed) {
            display.apply(&app.settings);
        }
        let (image, lines, dark) = app.compose();
        display.present(&image, lines, dark, &fonts, app.settings.integer_scaling)?;
        if stats {
            loops += 1;
            busy += loop_start.elapsed();
            if stat_start.elapsed() >= Duration::from_secs(1) {
                eprintln!("{loops} loops, {} frames, {:.1} ms per loop", app.frames_run, busy.as_secs_f64() * 1000.0 / f64::from(loops));
                (loops, app.frames_run, busy, stat_start) = (0, 0, Duration::ZERO, Instant::now());
            }
        }
        std::thread::sleep(app.wake().saturating_duration_since(Instant::now()).min(FRAME));
    }
    Ok(())
}

/// The window: the picture as a texture scaled to it, the texts drawn at
/// its resolution on top.
struct Display<'a> {
    canvas: sdl3::render::WindowCanvas,
    creator: &'a sdl3::render::TextureCreator<sdl3::video::WindowContext>,
    texture: sdl3::render::Texture<'a>,
    /// The texts' texture, its lines and size (made again only when they
    /// change).
    text: Option<(sdl3::render::Texture<'a>, Vec<text::Line>, (u32, u32))>,
    palette: [[u8; 3]; 256],
    rgb: Vec<u8>,
}

impl<'a> Display<'a> {
    fn new(
        canvas: sdl3::render::WindowCanvas,
        creator: &'a sdl3::render::TextureCreator<sdl3::video::WindowContext>,
    ) -> Result<Display<'a>> {
        let mut texture =
            creator.create_texture_streaming(PixelFormat::RGB24, render::SCREEN_WIDTH as u32, render::SCREEN_HEIGHT as u32)?;
        texture.set_scale_mode(ScaleMode::Nearest);
        let rgb = vec![0u8; render::SCREEN_WIDTH * render::SCREEN_HEIGHT * 3];
        Ok(Display { canvas, creator, texture, text: None, palette: clut8(), rgb })
    }

    /// Whole multiples of the picture, or as large as the window allows;
    /// full screen or not.
    fn apply(&mut self, s: &options::Settings) {
        let mode = if s.integer_scaling { SDL_LOGICAL_PRESENTATION_INTEGER_SCALE } else { SDL_LOGICAL_PRESENTATION_LETTERBOX };
        let _ = self.canvas.set_logical_size(render::SCREEN_WIDTH as u32, render::SCREEN_HEIGHT as u32, mode);
        let _ = self.canvas.window_mut().set_fullscreen(s.fullscreen);
    }

    /// Show the picture and its texts, `dark` of 256 toward black (the
    /// gamma fades).
    fn present(&mut self, image: &render::Canvas, lines: Vec<text::Line>, dark: i32, fonts: &text::Fonts, integer_scaling: bool) -> Result<()> {
        image.to_rgb24(&self.palette, &mut self.rgb);
        if dark > 0 {
            for c in &mut self.rgb {
                *c = (u32::from(*c) * (256 - dark.clamp(0, 256)) as u32 / 256) as u8;
            }
        }
        let w = render::SCREEN_WIDTH;
        let rgb = &self.rgb;
        self.texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
            for (row, line) in rgb.chunks_exact(w * 3).enumerate() {
                buf[row * pitch..row * pitch + w * 3].copy_from_slice(line);
            }
        })?;
        let (ow, oh) = self.canvas.output_size()?;
        let k = (ow as f32 / w as f32).min(oh as f32 / render::SCREEN_HEIGHT as f32);
        let k = if integer_scaling { k.floor().max(1.0) } else { k };
        let size = ((w as f32 * k) as u32, (render::SCREEN_HEIGHT as f32 * k) as u32);
        if self.text.as_ref().is_none_or(|(_, l, s)| *l != lines || *s != size) {
            let rgba = fonts.draw(&lines, size.0 as usize, size.1 as usize);
            let mut t = self.creator.create_texture_streaming(PixelFormat::ABGR8888, size.0, size.1)?;
            t.set_blend_mode(sdl3::render::BlendMode::Blend);
            t.with_lock(None, |buf: &mut [u8], pitch: usize| {
                let row = size.0 as usize * 4;
                for (y, line) in rgba.chunks_exact(row).enumerate() {
                    buf[y * pitch..y * pitch + row].copy_from_slice(line);
                }
            })?;
            self.text = Some((t, lines, size));
        }
        self.canvas.clear();
        self.canvas.copy(&self.texture, None, None)?;
        if let Some((t, _, _)) = &mut self.text {
            let b = (255 * (256 - dark.clamp(0, 256)) / 256) as u8;
            t.set_color_mod(b, b, b);
            self.canvas.copy(t, None, None)?;
        }
        self.canvas.present();
        Ok(())
    }
}

/// `--screenshot`: the frames run without a window, the picture (with its
/// texts) written as a BMP.
fn write_screenshot(data: &GameData, assets: &Assets, fonts: &text::Fonts, mut game: Game, out: &str, frames: usize) -> Result<()> {
    let mut score = ScoreShown::default();
    // HARRY_SAVE=<file>: continue from that record first
    if let Some(sg) = std::env::var_os("HARRY_SAVE").and_then(|p| std::fs::read(p).ok()).map(|b| SaveGame::from_bytes(&b)) {
        game.continue_game(&sg, &data.levels, assets);
    }
    // HARRY_INV=id:count,… fills the inventory (for testing), the
    // first item selected, half the ammunition
    if let Ok(inv) = std::env::var("HARRY_INV") {
        for (slot, item) in inv.split(',').enumerate().take(game.g.inventory.len()) {
            if let Some((id, n)) = item.split_once(':').and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))) {
                game.g.inventory[slot] = (id, n);
            }
        }
        game.g.cur_slot = 0;
        game.g.ammo = harry_core::keys::Ammo { staple: 50, badass: 100, shrapnel: 2 };
    }
    let mut shown = None;
    for _ in 0..frames {
        if shown.is_some() {
            break;
        }
        let mut show = |b: &Balloon| {
            shown = balloon::place(b, data, fonts);
            shown.is_some()
        };
        game.frame(&KeyMap::default(), assets, &mut show, &mut || false);
        score.step(game.g.score);
    }
    if std::env::var_os("HARRY_DEBUG").is_some() {
        for &i in &game.world.sprites.list {
            let r = game.world.rec(i);
            eprintln!("{} at {:?} face {} task {:?} b8b {} layer {}", r.kind, r.at, r.face, r.task, r.b8b, r.layer);
        }
        eprintln!("view {:?}", game.view.corner);
    }
    let (mut image, lines) = picture(data, &game, &score, shown.as_ref());
    // HARRY_MAP=1 shows the map, =all with every tile seen (for testing)
    if let Ok(m) = std::env::var("HARRY_MAP") {
        if m == "all" {
            game.track.cells.fill(1);
        }
        render::draw_map(&mut image, data, &game);
    }
    let mut rgb = vec![0u8; image.width * image.height * 3];
    image.to_rgb24(&clut8(), &mut rgb);
    // the text at 1:1 on the picture
    let overlay = fonts.draw(&lines, image.width, image.height);
    for (p, o) in rgb.as_chunks_mut::<3>().0.iter_mut().zip(overlay.as_chunks::<4>().0) {
        for k in 0..3 {
            p[k] = ((u32::from(p[k]) * (255 - u32::from(o[3])) + u32::from(o[k]) * u32::from(o[3])) / 255) as u8;
        }
    }
    render::write_bmp(Path::new(&out), image.width, image.height, &rgb)?;
    Ok(())
}

/// The score as the panel shows it (`UpdateText`): it counts up 50 a
/// frame (a lower score is shown at once), brighter while far behind.
#[derive(Default)]
struct ScoreShown {
    value: i32,
    /// Grey level of the digits (32000 … 64000 of 65535).
    grey: u16,
}

impl ScoreShown {
    fn step(&mut self, score: i32) {
        if self.grey == 0 {
            self.grey = 32000;
        }
        if score == self.value {
            return;
        }
        let mut diff = 0;
        if score < self.value {
            self.value = score;
        } else {
            self.value += 50;
            diff = score - self.value;
        }
        self.grey = (diff.min(800) * 40 + 32000) as u16;
    }
}

/// The picture of a frame and its texts: the panel's level name and score
/// (Helvetica 9 at (483, 118), the score bold at (483, 141)) and a balloon.
fn picture(data: &GameData, game: &Game, score: &ScoreShown, shown: Option<&balloon::Shown>) -> (render::Canvas, Vec<text::Line>) {
    let mut image = render::draw(data, game);
    let grey = |v: u16| [(v >> 8) as u8; 3];
    let mut lines = vec![
        text::Line {
            x: 483.0,
            baseline: 118.0,
            size: 9.0,
            face: text::Face::Helvetica,
            color: grey(32000),
            text: game.level.name.clone(),
        },
        text::Line {
            x: 483.0,
            baseline: 141.0,
            size: 9.0,
            face: text::Face::HelveticaBold,
            color: grey(score.grey.max(32000)),
            text: score.value.to_string(),
        },
    ];
    lines.extend(render::inventory_counts(game).into_iter().map(|(x, baseline, text)| text::Line {
        x,
        baseline,
        size: 9.0,
        face: text::Face::HelveticaBold,
        color: [255; 3],
        text,
    }));
    if let Some(s) = shown {
        lines.extend(balloon::draw(&mut image, s));
    }
    (image, lines)
}
