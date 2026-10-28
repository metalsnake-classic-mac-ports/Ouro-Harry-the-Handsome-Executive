//! Harry the Handsome Executive: the game with SDL3. `ouro-harry [pack.zip]
//! [level]`; without a path the data pack `harry-data.zip` is looked for
//! next to the program, in the current folder and in `re/work`.
//!
//! The game runs at the original's 30 frames per second. Keys as on the
//! Mac (the default bindings): ← → turn, ↓ scoot, ↑ inch, space kick,
//! control select item, option / alt use item, 1–4 pick a weapon or the
//! swivel. Escape quits, F11 toggles full screen; N / P jump to the next /
//! previous level (for testing).
//!
//! `ouro-harry --screenshot out.bmp [level] [frames]` runs the frames
//! without keys and writes the picture, without a window.

mod render;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use harry_core::game::{Assets, FrameEnd, Game, Prefs};
use harry_core::keys::{Bindings, KeyMap};
use harry_core::kinds::LevelCounter;
use harry_core::player::Masks;
use harry_core::random::Random;
use harry_core::view::View;
use harry_core::world::World;
use harry_core::frame::PlayerGlobals;
use harry_data::{GameData, clut8};
use sdl3::event::Event;
use sdl3::keyboard::{KeyboardState, Scancode};
use sdl3::pixels::PixelFormat;
use sdl3::render::ScaleMode;
use sdl3::sys::render::SDL_LOGICAL_PRESENTATION_INTEGER_SCALE;

/// The original's frame: 2 ticks of 1/60 s.
const FRAME: Duration = Duration::from_micros(33_333);

/// SDL keys → Mac virtual key codes (the ones the game reads).
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
    (Scancode::A, 0x00),
    (Scancode::S, 0x01),
    (Scancode::D, 0x02),
    (Scancode::F, 0x03),
    (Scancode::H, 0x04),
    (Scancode::G, 0x05),
    (Scancode::Z, 0x06),
    (Scancode::X, 0x07),
    (Scancode::C, 0x08),
    (Scancode::V, 0x09),
    (Scancode::B, 0x0B),
    (Scancode::Q, 0x0C),
    (Scancode::W, 0x0D),
    (Scancode::E, 0x0E),
    (Scancode::R, 0x0F),
    (Scancode::Y, 0x10),
    (Scancode::T, 0x11),
    (Scancode::O, 0x1F),
    (Scancode::U, 0x20),
    (Scancode::I, 0x22),
    (Scancode::L, 0x25),
    (Scancode::J, 0x26),
    (Scancode::K, 0x28),
    (Scancode::M, 0x2E),
];

fn keymap(state: &KeyboardState) -> KeyMap {
    let mut k = KeyMap::default();
    for &(sc, code) in KEYS {
        if state.is_scancode_pressed(sc) {
            k.press(code);
        }
    }
    k
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
        },
        counter: LevelCounter { current: level as i16 + 1, previous: level as i16 },
        seen_level: level as i16 + 1,
        plant_flag: false,
        magnet: 0,
        gun: 0,
        harry: None,
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
        face_bounds: &face_bounds,
    };
    let mut level: usize = args.get(2).and_then(|s| s.parse::<usize>().ok()).map_or(0, |n| n.clamp(1, data.levels.len()) - 1);
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.subsec_nanos() | 1);
    let start = |level: usize, g: PlayerGlobals, seed: u32| {
        let mut game = new_game(&data, level, g, seed);
        let (h, v) = game.level.start;
        let missing = game.start(h - 32, v - 32, game.level.start_dir, &assets);
        if !missing.is_empty() {
            eprintln!("level {}: not ported yet: {missing:?}", level + 1);
        }
        game
    };
    let mut game = start(level, PlayerGlobals::new_game(), seed);
    if let Some((out, frames)) = screenshot {
        for _ in 0..frames {
            game.frame(&KeyMap::default(), &assets, &mut |_, _, _| false, &mut || false);
        }
        if std::env::var_os("HARRY_DEBUG").is_some() {
            for &i in &game.world.sprites.list {
                let r = game.world.rec(i);
                eprintln!("{} at {:?} face {} task {:?} b8b {} layer {}", r.kind, r.at, r.face, r.task, r.b8b, r.layer);
            }
            eprintln!("view {:?}", game.view.corner);
        }
        let image = render::draw(&data, &game);
        let mut rgb = vec![0u8; image.width * image.height * 3];
        image.to_rgb24(&clut8(), &mut rgb);
        render::write_bmp(Path::new(&out), image.width, image.height, &rgb)?;
        return Ok(());
    }

    let sdl = sdl3::init()?;
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
    'run: loop {
        let mut jump: Option<usize> = None;
        for e in events.poll_iter() {
            match e {
                Event::Quit { .. } => break 'run,
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } => match sc {
                    Scancode::Escape => break 'run,
                    Scancode::F11 => {
                        fullscreen = !fullscreen;
                        let _ = canvas.window_mut().set_fullscreen(fullscreen);
                    }
                    Scancode::N => jump = Some((level + 1) % data.levels.len()),
                    Scancode::P => jump = Some((level + data.levels.len() - 1) % data.levels.len()),
                    _ => {}
                },
                _ => {}
            }
        }
        if let Some(n) = jump {
            level = n;
            game = start(level, PlayerGlobals::new_game(), game.rng.seed);
        }
        let now = Instant::now();
        while next <= now {
            next += FRAME;
            let keys = keymap(&events.keyboard_state());
            let mut show = |_: &harry_core::sprite::Sprite, _, _| false;
            let mut save = || false;
            match game.frame(&keys, &assets, &mut show, &mut save) {
                FrameEnd::Running => {}
                FrameEnd::LevelDone => {
                    let n = (game.counter.current as usize).saturating_sub(1);
                    level = if n < data.levels.len() { n } else { 0 };
                    let mut g = game.g.clone();
                    g.tile.floor_kind = 1;
                    game = start(level, g, game.rng.seed);
                }
                FrameEnd::Died => {
                    // HandlePlayerSpriteDeath is not ported yet: start the level again
                    game = start(level, PlayerGlobals::new_game(), game.rng.seed);
                }
            }
        }
        let image = render::draw(&data, &game);
        image.to_rgb24(&palette, &mut rgb);
        let w = render::SCREEN_WIDTH;
        texture.with_lock(None, |buf: &mut [u8], pitch: usize| {
            for (row, line) in rgb.chunks_exact(w * 3).enumerate() {
                buf[row * pitch..row * pitch + w * 3].copy_from_slice(line);
            }
        })?;
        canvas.clear();
        canvas.copy(&texture, None, None)?;
        canvas.present();
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    Ok(())
}
