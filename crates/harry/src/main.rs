//! Harry the Handsome Executive: the game with SDL3. `ouro-harry [pack.zip]
//! [level]`; without a path the data pack `harry-data.zip` is looked for
//! next to the program, in the current folder and in `re/work`.
//!
//! The game runs at the original's 30 frames per second, with its sounds. Keys as on the
//! Mac (the default bindings): ← → turn, ↓ scoot, ↑ inch, space kick,
//! control select item, option / alt use item, 1–4 pick a weapon or the
//! swivel. Escape quits, F11 toggles full screen; N / P jump to the next /
//! previous level (for testing).
//!
//! `ouro-harry --screenshot out.bmp [level] [frames]` runs the frames
//! without keys and writes the picture, without a window. With
//! `HARRY_STATS` set the game prints its loop timing once a second;
//! `cargo run -p harry --example audio_probe` shows how SDL drives the
//! audio callback.

mod audio;
mod balloon;
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
use sdl3::keyboard::{KeyboardState, Keycode, Scancode};
use sdl3::sys::keycode::SDL_KMOD_NONE;
use sdl3::pixels::PixelFormat;
use sdl3::render::ScaleMode;
use sdl3::sys::render::SDL_LOGICAL_PRESENTATION_INTEGER_SCALE;

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

fn find_pack(arg: Option<&String>) -> Result<PathBuf> {
    if let Some(a) = arg {
        return Ok(PathBuf::from(a));
    }
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
    let candidates = [exe_dir.map(|d| d.join("harry-data.zip")), Some(PathBuf::from("harry-data.zip")), Some(PathBuf::from("re/work/harry-data.zip"))];
    candidates.into_iter().flatten().find(|p| p.exists()).ok_or_else(|| {
        anyhow!("harry-data.zip not found - make it with ouro-harry-extract from the original game's files")
    })
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
        level_start: LevelStart::default(),
        track: Default::default(),
        keep_track: false,
            blits: Vec::new(),
            meters: Default::default(),
        session: Default::default(),
    }
}

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().collect();
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
    let path = find_pack(args.get(1).filter(|s| !s.is_empty()))?;
    let bytes = std::fs::read(&path).with_context(|| path.display().to_string())?;
    let data = harry_data::pack::read(&bytes).with_context(|| path.display().to_string())?;
    let loaded = Loaded::new(&data)?;
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
        face_bounds: &face_bounds,
    };
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
        let (image, lines) = picture(&data, &game, &score, shown.as_ref());
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
    canvas.set_logical_size(render::SCREEN_WIDTH as u32, render::SCREEN_HEIGHT as u32, SDL_LOGICAL_PRESENTATION_INTEGER_SCALE)?;
    let palette = clut8();
    let mut rgb = vec![0u8; render::SCREEN_WIDTH * render::SCREEN_HEIGHT * 3];
    let mut next = Instant::now();
    let mut fullscreen = false;
    // a balloon being shown and since when (the game waits meanwhile)
    let mut shown: Option<(balloon::Shown, Instant)> = None;
    // a sequence between frames (fades, pictures; the game waits), the next
    // tick, and whether the screen is faded to black when none runs
    let start_screen = |game: &Game| {
        let mut c = render::draw(&data, game);
        c.fill(16, 16, 448, 448, 255);
        c
    };
    let mut scene = Some(scenes::Scenes::new(start_screen(&game), true, scenes::game_start(&game.level, true)));
    let mut tick = Instant::now();
    let mut faded = true;
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
        for e in events.poll_iter() {
            match e {
                Event::Quit { .. } => break 'run,
                Event::MouseButtonDown { .. } => pressed = true,
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } => {
                    if let Some((_, since)) = &shown {
                        // Return / Enter at once, any key after a second
                        if matches!(sc, Scancode::Return | Scancode::KpEnter) || since.elapsed() >= Duration::from_secs(1) {
                            shown = None;
                            next = Instant::now();
                        }
                        continue;
                    }
                    match sc {
                        Scancode::Escape => break 'run,
                        _ if scene.is_some() && sc != Scancode::F11 => pressed = true,
                        Scancode::F11 => {
                            fullscreen = !fullscreen;
                            let _ = canvas.window_mut().set_fullscreen(fullscreen);
                        }
                        Scancode::N => jump = Some((level + 1) % data.levels.len()),
                        Scancode::P => jump = Some((level + data.levels.len() - 1) % data.levels.len()),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if let Some(n) = jump {
            level = n;
            game = start(level, PlayerGlobals::new_game(), game.rng.seed);
            scene = Some(scenes::Scenes::new(start_screen(&game), faded, scenes::game_start(&game.level, false)));
            tick = Instant::now();
        }
        let now = Instant::now();
        if let Some(sc) = scene.as_mut() {
            while tick <= now && !sc.done() {
                sc.tick(&data, std::mem::take(&mut pressed));
                tick += TICK;
            }
            if sc.done() {
                faded = sc.dark >= 256;
                scene = None;
                next = Instant::now();
            }
        }
        while scene.is_none() && shown.is_none() && next <= now {
            next += FRAME;
            let keys = keymap(&events.keyboard_state());
            let mut asked = None;
            let mut show = |b: &Balloon| {
                asked = balloon::place(b, &data, &fonts);
                asked.is_some()
            };
            // CopierSave's dialog is not ported: at the copier Harry saves
            let mut save = || true;
            let end = game.frame(&keys, &assets, &mut show, &mut save);
            frames_run += 1;
            mixer.events(&game.sounds, game.level.own_sounds);
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
                            scene = Some(scenes::Scenes::new(screen, faded, scenes::up_level(&old, &game.level)));
                        }
                        // after Win the main menu (not ported): a new game
                        (UpLevel::Won, _) => {
                            let mut steps = scenes::win(&data.world);
                            level = 0;
                            game = start(level, PlayerGlobals::new_game(), game.rng.seed);
                            steps.push(scenes::Step::BlackScreen);
                            let mut sc = scenes::Scenes::new(screen, faded, steps);
                            sc.then = Some((start_screen(&game), scenes::game_start(&game.level, true)));
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
                    // AskToContinue's dialog is not ported: with a game saved
                    // in this session it goes on from there, else a new game
                    // starts at this level; both start with GameLoop's
                    // sequence (a copier save without the story)
                    let saved = save_path.as_ref().filter(|_| have_save).and_then(|p| std::fs::read(p).ok());
                    let mut intro = true;
                    match saved.map(|b| SaveGame::from_bytes(&b)) {
                        Some(sg) => {
                            if let Some(missing) = game.continue_game(&sg, &data.levels, &assets) {
                                level = (sg.level as usize).saturating_sub(1);
                                intro = !sg.copier;
                                if !missing.is_empty() {
                                    eprintln!("level {}: not ported yet: {missing:?}", level + 1);
                                }
                            }
                        }
                        None => {
                            let session = std::mem::take(&mut game.session);
                            game = start(level, PlayerGlobals::new_game(), game.rng.seed);
                            game.session = session;
                        }
                    }
                    let mut sc = scenes::Scenes::new(screen, faded, steps);
                    sc.then = Some((start_screen(&game), scenes::game_start(&game.level, intro)));
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
        let (mut image, mut lines) = picture(&data, &game, &score, shown.as_ref().map(|(b, _)| b));
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
        let k = (ow as f32 / w as f32).min(oh as f32 / render::SCREEN_HEIGHT as f32).floor().max(1.0);
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
        let wake = if scene.is_some() { tick } else { next };
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
    if let Some(s) = shown {
        lines.extend(balloon::draw(&mut image, s));
    }
    (image, lines)
}
