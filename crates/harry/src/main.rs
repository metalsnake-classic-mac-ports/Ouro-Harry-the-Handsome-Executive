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
use harry_core::game::{Assets, Balloon, FrameEnd, Game, LevelStart, Prefs, UpLevel};
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
    let mut settings = settings_path.as_deref().map(options::Settings::load).unwrap_or_default();
    let face_bounds = |f: i16| data.faces.get(&f).map_or([0; 4], |x| x.bounds);
    let assets = Assets {
        player: &loaded.player,
        plant: &loaded.plant,
        barrel: &loaded.barrel,
        cooler: &loaded.cooler,
        shot: &loaded.shot,
        ramps: [&loaded.ramps[0], &loaded.ramps[1], &loaded.ramps[2], &loaded.ramps[3]],
        player_shot: &loaded.player_shot,
        robot: &loaded.robot,
        blob: &loaded.blob,
        desks: [&loaded.desks[0], &loaded.desks[1]],
        face_bounds: &face_bounds,
    };
    // a level on the command line starts it at once, else the main menu
    let direct = args.get(2).is_some();
    let mut level: usize = args.get(2).and_then(|s| s.parse::<usize>().ok()).map_or(0, |n| n.clamp(1, data.levels.len()) - 1);
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() | 1);
    let start = |level: usize, g: PlayerGlobals, seed: u32| {
        let mut game = new_game(&data, level, g, seed);
        let (h, v) = game.level.start;
        // HARRY_START=h,v puts Harry elsewhere (for testing)
        let (h, v) = std::env::var("HARRY_START")
            .ok()
            .and_then(|s| s.split_once(',').map(|(a, b)| (a.parse().ok(), b.parse().ok())))
            .and_then(|(a, b)| Some((a?, b?)))
            .unwrap_or((h - 32, v - 32));
        let missing = game.start(h, v, game.level.start_dir, &assets);
        if !missing.is_empty() {
            eprintln!("level {}: not ported yet: {missing:?}", level + 1);
        }
        game
    };
    let mut game = start(level, PlayerGlobals::new_game(), seed);
    // the saved game (the record as the original keeps it in its HrSG
    // resource); continuing needs one saved in this session
    let save_path = sdl3::filesystem::get_pref_path("MetalSnake", "Ouro Harry").ok().map(|d| d.join("Harry Save.hrsg"));
    let mut have_save = false;
    let fonts = text::Fonts::load();
    let mut score = ScoreShown::default();
    if let Some((out, frames)) = screenshot {
        // HARRY_SAVE=<file>: continue from that record first
        if let Some(sg) = std::env::var_os("HARRY_SAVE").and_then(|p| std::fs::read(p).ok()).map(|b| SaveGame::from_bytes(&b)) {
            game.continue_game(&sg, &data.levels, &assets);
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
                shown = balloon::place(b, &data, &fonts);
                shown.is_some()
            };
            game.frame(&KeyMap::default(), &assets, &mut show, &mut || false);
            score.step(game.g.score);
        }
        if std::env::var_os("HARRY_DEBUG").is_some() {
            for &i in &game.world.sprites.list {
                let r = game.world.rec(i);
                eprintln!("{} at {:?} face {} task {:?} b8b {} layer {}", r.kind, r.at, r.face, r.task, r.b8b, r.layer);
            }
            eprintln!("view {:?}", game.view.corner);
        }
        let (mut image, lines) = picture(&data, &game, &score, shown.as_ref());
        // HARRY_MAP=1 shows the map, =all with every tile seen (for testing)
        if let Ok(m) = std::env::var("HARRY_MAP") {
            if m == "all" {
                game.track.cells.fill(1);
            }
            render::draw_map(&mut image, &data, &game);
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
        return Ok(());
    }

    let sdl = sdl3::init()?;
    let mut mixer = audio::Mixer::open(&sdl, &data);
    let video = sdl.video()?;
    let window = video
        .window("Harry the Handsome Executive", 2 * render::SCREEN_WIDTH as u32, 2 * render::SCREEN_HEIGHT as u32)
        .resizable()
        .position_centered()
        .build()?;
    let mut events = sdl.event_pump()?;
    let mut canvas = window.into_canvas();
    let creator = canvas.texture_creator();
    let mut texture =
        creator.create_texture_streaming(PixelFormat::RGB24, render::SCREEN_WIDTH as u32, render::SCREEN_HEIGHT as u32)?;
    texture.set_scale_mode(ScaleMode::Nearest);
    // whole multiples of the picture, or as large as the window allows
    let apply_display = |canvas: &mut sdl3::render::WindowCanvas, s: &options::Settings| {
        let mode = if s.integer_scaling { SDL_LOGICAL_PRESENTATION_INTEGER_SCALE } else { SDL_LOGICAL_PRESENTATION_LETTERBOX };
        let _ = canvas.set_logical_size(render::SCREEN_WIDTH as u32, render::SCREEN_HEIGHT as u32, mode);
        let _ = canvas.window_mut().set_fullscreen(s.fullscreen);
    };
    apply_display(&mut canvas, &settings);
    mixer.set_volume(settings.volume);
    mixer.set_music_volume(settings.music_volume);
    let palette = clut8();
    let mut rgb = vec![0u8; render::SCREEN_WIDTH * render::SCREEN_HEIGHT * 3];
    let mut next = Instant::now();
    // a balloon being shown and since when (the game waits meanwhile)
    let mut shown: Option<(balloon::Shown, Instant)> = None;
    // a sequence between frames (fades, pictures; the game waits), the next
    // tick, and whether the screen is faded to black when none runs
    let start_screen = |game: &Game| {
        let mut c = render::draw(&data, game);
        c.fill(16, 16, 448, 448, 255);
        c
    };
    let menu_screen = |m: &menu::Menu| m.draw(&data, &fonts).0;
    // the main menu (when it shows, the game waits); the question before a
    // game is given up
    let mut in_menu: Option<menu::Menu> = None;
    // the options screen over the menu
    let mut opts: Option<options::Screen> = None;
    let mut confirm = false;
    // Pause (the original's `Pause`): by the pause key - caps lock as in
    // the original, paused while it is locked; another key by turns - or
    // when the window loses focus (until a key or a click)
    let mut pause_toggled = false;
    let mut focus_paused = false;
    let mut was_paused = false;
    // Command-I (Control-I off the Mac) in the game: the options, the game
    // waits
    let options_mod = if cfg!(target_os = "macos") { Mod::LGUIMOD | Mod::RGUIMOD } else { Mod::LCTRLMOD | Mod::RCTRLMOD };
    // ShowMap: open since when; frames before the key opens it again
    let mut map: Option<Instant> = None;
    let mut map_cooldown = 0u32;
    let mut scene = if direct {
        Some(scenes::Scenes::new(start_screen(&game), true, scenes::game_start(&game.level, true, &data.world)))
    } else {
        let m = menu::Menu::new();
        let mut sc = scenes::Scenes::new(start_screen(&game), true, Vec::new());
        // main: SetMusic(600), the menu
        sc.then = Some((menu_screen(&m), vec![scenes::Step::Music(600), scenes::Step::FadeIn(8)]));
        in_menu = Some(m);
        Some(sc)
    };
    let mut tick = Instant::now();
    let mut faded = true;
    // the music is faded out (FadeMusic's DAT_10072711)
    let mut music_faded = false;
    // to the main menu from the screen shown: fade out, the menu, fade in
    let to_menu = |screen: render::Canvas, faded: bool, steps: Vec<scenes::Step>| {
        let m = menu::Menu::new();
        let mut steps = steps;
        steps.push(scenes::Step::FadeOut(8));
        let mut sc = scenes::Scenes::new(screen, faded, steps);
        let mut then = vec![scenes::Step::FadeIn(8)];
        then.extend(scenes::menu_music());
        sc.then = Some((menu_screen(&m), then));
        (m, sc)
    };
    let mut text_texture: Option<(sdl3::render::Texture, Vec<text::Line>, (u32, u32))> = None;
    // HARRY_STATS: loop timing once a second
    let stats = std::env::var_os("HARRY_STATS").is_some();
    let (mut loops, mut frames_run, mut busy, mut stat_start) = (0u32, 0u32, Duration::ZERO, Instant::now());
    'run: loop {
        let loop_start = Instant::now();
        let mut jump: Option<usize> = None;
        // a key or mouse button pressed now (it can end a scrolled picture;
        // one held since before cannot)
        let mut pressed = false;
        let mut action: Option<menu::Action> = None;
        let mut opt_event: Option<options::Event> = None;
        let mut give_up = false;
        for mut e in events.poll_iter() {
            e.convert_coords(&canvas);
            let menu_live = scene.is_none() && in_menu.is_some();
            match e {
                Event::Quit { .. } => break 'run,
                Event::Window { win_event: sdl3::event::WindowEvent::FocusLost, .. } => focus_paused = true,
                // a key or a click ends the pause of a lost focus (and does
                // nothing else)
                Event::KeyDown { .. } | Event::MouseButtonDown { .. } if focus_paused && scene.is_none() && in_menu.is_none() => {
                    focus_paused = false;
                }
                Event::MouseButtonDown { x, y, .. } => {
                    pressed = true;
                    if scene.is_none() && let Some(o) = opts.as_mut() {
                        opt_event = opt_event.or(o.click(x as i32, y as i32));
                    } else if menu_live && let Some(m) = in_menu.as_mut() {
                        m.mouse_down(x as i32, y as i32);
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    if menu_live && let Some(m) = in_menu.as_mut() {
                        m.mouse_move(x as i32, y as i32);
                    }
                }
                Event::MouseButtonUp { x, y, .. } => {
                    if menu_live && let Some(m) = in_menu.as_mut() {
                        action = action.or(m.mouse_up(x as i32, y as i32));
                    }
                }
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } if confirm => match sc {
                    Scancode::Return | Scancode::KpEnter | Scancode::Y => {
                        confirm = false;
                        give_up = true;
                    }
                    Scancode::Escape | Scancode::N => {
                        confirm = false;
                        next = Instant::now();
                    }
                    _ => {}
                },
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } if scene.is_none() && opts.is_some() => {
                    if let Some(o) = opts.as_mut() {
                        opt_event = opt_event.or(o.key(sc, mac_code(sc)));
                    }
                }
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } if menu_live && sc != settings.fullscreen_key => {
                    let Some(m) = in_menu.as_mut() else { continue };
                    if sc == Scancode::Escape {
                        if m.message.is_none() && !m.credits_open() {
                            break 'run;
                        }
                        m.key('\x1b');
                    } else if let Some(c) = Keycode::from_scancode(sc, SDL_KMOD_NONE, false)
                        .and_then(|k| char::from_u32(k as u32))
                        .filter(char::is_ascii_lowercase)
                    {
                        m.key(c);
                    } else if matches!(sc, Scancode::Return | Scancode::KpEnter) {
                        m.key('\r');
                    }
                }
                Event::KeyDown { scancode: Some(sc), keymod, repeat: false, .. } => {
                    if let Some((_, since)) = &shown {
                        // Return / Enter at once, any key after a second
                        if matches!(sc, Scancode::Return | Scancode::KpEnter) || since.elapsed() >= Duration::from_secs(1) {
                            shown = None;
                            next = Instant::now();
                        }
                        continue;
                    }
                    match sc {
                        _ if scene.is_some() && sc != settings.fullscreen_key => pressed = true,
                        // CheckGameLoopKeys: Escape asks before the game is given up
                        Scancode::Escape => confirm = true,
                        // Command-I: the options
                        Scancode::I if keymod.intersects(options_mod) => opts = Some(options::Screen::new(settings.clone())),
                        _ if sc == settings.pause_key && sc != Scancode::CapsLock => pause_toggled = !pause_toggled,
                        _ if sc == settings.fullscreen_key => {
                            settings.fullscreen = !settings.fullscreen;
                            apply_display(&mut canvas, &settings);
                            if let Some(p) = &settings_path {
                                let _ = settings.save(p);
                            }
                        }
                        Scancode::F8 => jump = Some((level + 1) % data.levels.len()),
                        Scancode::F7 => jump = Some((level + data.levels.len() - 1) % data.levels.len()),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if let Some(n) = jump {
            level = n;
            game = start(level, PlayerGlobals::new_game(), game.rng.seed);
            scene = Some(scenes::Scenes::new(start_screen(&game), faded, scenes::game_start(&game.level, false, &data.world)));
            tick = Instant::now();
        }
        let now = Instant::now();
        if let Some(sc) = scene.as_mut() {
            sc.music_faded = music_faded;
            while tick <= now && !sc.done() {
                sc.tick(&data, std::mem::take(&mut pressed));
                for c in std::mem::take(&mut sc.music) {
                    match c {
                        scenes::MusicCommand::Play(id) => mixer.music(id),
                        scenes::MusicCommand::Fade { out, ticks } => mixer.fade_music(out, ticks),
                    }
                }
                tick += TICK;
            }
            music_faded = sc.music_faded;
            if sc.done() {
                faded = sc.dark >= 256;
                scene = None;
                next = Instant::now();
            }
        }
        // the main menu: its ticks, sounds and what it asks for
        if scene.is_none()
            && let Some(m) = in_menu.as_mut()
        {
            while tick <= now {
                action = action.or(m.tick(&data));
                tick += TICK;
            }
            mixer.events(&std::mem::take(&mut m.sounds), false);
        }
        if let Some(e) = opt_event
            && let Some(o) = &opts
        {
            settings = o.settings.clone();
            apply_display(&mut canvas, &settings);
            mixer.set_volume(settings.volume);
            mixer.set_music_volume(settings.music_volume);
            if e == options::Event::Done {
                if let Some(p) = &settings_path
                    && let Err(err) = settings.save(p)
                {
                    eprintln!("{}: {err}", p.display());
                }
                opts = None;
                next = Instant::now();
            }
        }
        match action {
            Some(menu::Action::Options) => opts = Some(options::Screen::new(settings.clone())),
            Some(menu::Action::NewGame) => {
                level = 0;
                game = start(level, PlayerGlobals::new_game(), game.rng.seed);
                in_menu = None;
                scene = Some(scenes::Scenes::new(start_screen(&game), faded, scenes::game_start(&game.level, true, &data.world)));
                tick = now;
            }
            // ContinueGame: the save list is to come; the game saved in
            // this session goes on (a copier save without the story)
            Some(menu::Action::Continue) => {
                // HARRY_SAVE=<file> continues from that record instead (for
                // testing, e.g. the saves of `ouro-harry-repro`)
                let saved = match std::env::var_os("HARRY_SAVE") {
                    Some(p) => std::fs::read(p).ok(),
                    None => save_path.as_ref().filter(|_| have_save).and_then(|p| std::fs::read(p).ok()),
                };
                if let Some(sg) = saved.map(|b| SaveGame::from_bytes(&b))
                    && let Some(missing) = game.continue_game(&sg, &data.levels, &assets)
                {
                    level = (sg.level as usize).saturating_sub(1);
                    if !missing.is_empty() {
                        eprintln!("level {}: not ported yet: {missing:?}", level + 1);
                    }
                    in_menu = None;
                    scene = Some(scenes::Scenes::new(start_screen(&game), faded, scenes::game_start(&game.level, !sg.copier, &data.world)));
                    tick = now;
                }
            }
            Some(menu::Action::Quit) => break 'run,
            None => {}
        }
        if give_up {
            let (m, sc) = to_menu(render::draw(&data, &game), faded, Vec::new());
            in_menu = Some(m);
            scene = Some(sc);
            tick = now;
        }
        // the map closes with its key once 12 ticks have passed
        if let Some(since) = map
            && since.elapsed() >= TICK * 12
            && keymap(&events.keyboard_state()).down(settings.map_key)
        {
            map = None;
            next = Instant::now();
        }
        // the pause: its sound (snd 198) as it starts; when it ends the
        // game goes on from now
        let in_game = scene.is_none() && in_menu.is_none();
        let caps = settings.pause_key == Scancode::CapsLock && sdl.keyboard().mod_state().contains(Mod::CAPSMOD);
        let paused = in_game && (caps || pause_toggled || focus_paused);
        if paused && !was_paused {
            mixer.events(
                &[harry_core::sound::SoundEvent::Play {
                    id: 198,
                    left: 0x100,
                    right: 0x100,
                    rate: harry_core::sound::RATE,
                    channel: harry_core::sound::Channel::Effect,
                    handle: 0,
                }],
                false,
            );
        } else if !paused && was_paused {
            next = Instant::now();
        }
        was_paused = paused;
        if !in_game {
            (pause_toggled, focus_paused) = (false, false);
        }
        // the settings the game reads
        game.prefs.scroll = settings.camera;
        game.prefs.autosave = settings.autosave;
        game.prefs.bindings = settings.bindings;
        while scene.is_none() && in_menu.is_none() && !paused && opts.is_none() && !confirm && map.is_none() && shown.is_none() && next <= now {
            next += FRAME;
            let keys = keymap(&events.keyboard_state());
            let mut asked = None;
            let mut show = |b: &Balloon| {
                asked = balloon::place(b, &data, &fonts);
                asked.is_some()
            };
            // CopierSave's dialog is not ported: at the copier Harry saves
            let mut save = || true;
            game.audio.playing = mixer.playing();
            let end = game.frame(&keys, &assets, &mut show, &mut save);
            frames_run += 1;
            // CheckGameLoopKeys: the map key (not again for 6 passes)
            map_cooldown = map_cooldown.saturating_sub(1);
            if keys.down(settings.map_key) && map_cooldown == 0 {
                map = Some(Instant::now());
                map_cooldown = 6;
            }
            mixer.events(&game.sounds, game.level.own_sounds);
            if game.sounds.iter().any(|e| matches!(e, harry_core::sound::SoundEvent::FadeMusicOut(_))) {
                music_faded = true;
            }
            score.step(game.g.score);
            if let Some(b) = asked {
                shown = Some((b, Instant::now()));
            }
            match end {
                FrameEnd::Running => {}
                FrameEnd::LevelDone => {
                    // the screen as the level ended, under the sequence
                    let (screen, old) = (render::draw(&data, &game), game.level.clone());
                    let heard = game.sounds.len();
                    let up = game.up_level(&data.levels, &assets);
                    // UpLevel's jingle (4800)
                    mixer.events(&game.sounds[heard..], old.own_sounds);
                    match up {
                        (UpLevel::Next, missing) => {
                            level = (game.counter.current as usize).saturating_sub(1);
                            if !missing.is_empty() {
                                eprintln!("level {}: not ported yet: {missing:?}", level + 1);
                            }
                            scene = Some(scenes::Scenes::new(screen, faded, scenes::up_level(&old, &game.level, &data.world)));
                        }
                        // after Win the main menu
                        (UpLevel::Won, _) => {
                            let (m, sc) = to_menu(screen, faded, scenes::win(&data.world));
                            in_menu = Some(m);
                            scene = Some(sc);
                        }
                    }
                    tick = Instant::now();
                }
                FrameEnd::Died => {
                    // GameLoop's end: EndLevelEffect when comfort or favor
                    // is gone
                    let screen = render::draw(&data, &game);
                    let mut steps = Vec::new();
                    if game.g.comfort < 1 || game.g.favor < 1 {
                        steps.push(scenes::Step::EndLevel);
                    }
                    // then the main menu (AskToContinue is to come with the
                    // save list; the menu's Continue goes on from the game
                    // saved in this session)
                    let (m, sc) = to_menu(screen, faded, steps);
                    in_menu = Some(m);
                    scene = Some(sc);
                    tick = Instant::now();
                }
            }
            if std::mem::take(&mut game.session.save_pending) {
                // SaveSG: the record, dated, to the save file
                game.session.save.date = mac_seconds();
                if let Some(p) = &save_path {
                    match std::fs::write(p, game.session.save.to_bytes()) {
                        Ok(()) => have_save = true,
                        Err(e) => eprintln!("{}: {e}", p.display()),
                    }
                }
            }
        }
        let (mut image, mut lines) = match &in_menu {
            Some(m) if scene.is_none() => {
                let (mut c, mut l) = m.draw(&data, &fonts);
                if let Some(o) = &opts {
                    l = o.draw(&mut c, &key_name);
                }
                (c, l)
            }
            _ => {
                let (mut c, mut l) = picture(&data, &game, &score, shown.as_ref().map(|(b, _)| b));
                if let Some(o) = &opts {
                    // over the game area; the panel's texts stay
                    l.extend(o.draw(&mut c, &key_name));
                } else if paused && let Some(p) = data.pictures.get(&4921) {
                    // Pause: PICT 4921 at (128, 192), over the game area's middle
                    c.draw_image(p, (0, 0, p.width, p.height), 128, 192);
                    // its "Press CAPS LOCK To Resume" (rows 64–82) when
                    // something else goes on
                    let hint = if focus_paused {
                        Some("Press any key to resume".to_string())
                    } else if settings.pause_key != Scancode::CapsLock {
                        Some(format!("Press {} to resume", settings.pause_key.name()))
                    } else {
                        None
                    };
                    if let Some(hint) = hint {
                        if let Some(grey) = p.get(20, 60) {
                            c.fill(128 + 6, 192 + 62, 213, 22, grey);
                        }
                        let w = fonts.width(text::Face::HelveticaBold, 14.0, &hint);
                        l.push(text::Line {
                            x: 128.0 + (225.0 - w) / 2.0,
                            baseline: 192.0 + 79.0,
                            size: 14.0,
                            face: text::Face::HelveticaBold,
                            color: [0; 3],
                            text: hint,
                        });
                    }
                }
                (c, l)
            }
        };
        if map.is_some() && scene.is_none() && in_menu.is_none() {
            render::draw_map(&mut image, &data, &game);
        }
        if confirm {
            lines.extend(menu::message_box(
                &mut image,
                &fonts,
                "Are you sure you want to abort this game?\n\nReturn: yes      Escape: no",
            ));
        }
        let mut dark = if faded { 256 } else { 0 };
        if let Some(sc) = &scene {
            image = sc.image.clone();
            dark = sc.dark;
            if !sc.texts {
                lines.clear();
            }
        }
        image.to_rgb24(&palette, &mut rgb);
        // the gamma fades: toward black
        if dark > 0 {
            for c in &mut rgb {
                *c = (u32::from(*c) * (256 - dark.clamp(0, 256)) as u32 / 256) as u8;
            }
        }
        let w = render::SCREEN_WIDTH;
        texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
            for (row, line) in rgb.chunks_exact(w * 3).enumerate() {
                buf[row * pitch..row * pitch + w * 3].copy_from_slice(line);
            }
        })?;
        // the texts at the window's resolution (made again only when they change)
        let (ow, oh) = canvas.output_size()?;
        let k = (ow as f32 / w as f32).min(oh as f32 / render::SCREEN_HEIGHT as f32);
        let k = if settings.integer_scaling { k.floor().max(1.0) } else { k };
        let size = ((w as f32 * k) as u32, (render::SCREEN_HEIGHT as f32 * k) as u32);
        if text_texture.as_ref().is_none_or(|(_, l, s)| *l != lines || *s != size) {
            let rgba = fonts.draw(&lines, size.0 as usize, size.1 as usize);
            let mut t = creator.create_texture_streaming(PixelFormat::ABGR8888, size.0, size.1)?;
            t.set_blend_mode(sdl3::render::BlendMode::Blend);
            t.with_lock(None, |buf: &mut [u8], pitch: usize| {
                let row = size.0 as usize * 4;
                for (y, line) in rgba.chunks_exact(row).enumerate() {
                    buf[y * pitch..y * pitch + row].copy_from_slice(line);
                }
            })?;
            text_texture = Some((t, lines, size));
        }
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        if let Some((t, _, _)) = &mut text_texture {
            let b = (255 * (256 - dark.clamp(0, 256)) / 256) as u8;
            t.set_color_mod(b, b, b);
            canvas.copy(t, None, None)?;
        }
        canvas.present();
        if stats {
            loops += 1;
            busy += loop_start.elapsed();
            if stat_start.elapsed() >= Duration::from_secs(1) {
                eprintln!("{loops} loops, {frames_run} frames, {:.1} ms per loop", busy.as_secs_f64() * 1000.0 / f64::from(loops));
                (loops, frames_run, busy, stat_start) = (0, 0, Duration::ZERO, Instant::now());
            }
        }
        let wake = if scene.is_some() || in_menu.is_some() { tick } else { next };
        std::thread::sleep(wake.saturating_duration_since(Instant::now()).min(FRAME));
    }
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
