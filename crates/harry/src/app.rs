//! The game as the window runs it: the main menu, the sequences between
//! levels, the game with its pause, map, balloons and options. `App::step`
//! takes one pass of the main loop's input (the events, the keys held, the
//! time); `App::compose` makes the picture. Nothing here touches the
//! window, so the tests drive it with made-up events and time.
//!
//! The Ouro ports' rules on top of the original (`../CLAUDE.md`):
//!
//! 1. Deaths, not game over: when comfort or favor run out the level
//!    starts again - from the last copier save in it, else from its start
//!    - after the original's `EndLevelEffect`; the panel counts the deaths.
//! 3. The level selection (the main menu's box, or L) with the records
//!    (`records.json`): a run is from the level's start to its exit,
//!    deaths included; its time counts the game's frames (30 a second).
//! 4. Continue: Escape in the game or quitting keeps the game
//!    (`continue.json`, the snapshots in `continue/`); the menu's
//!    Continue plays on exactly there. The original's question before a
//!    game is given up goes: nothing is lost.
//! 5. Rewind: a snapshot every 5 seconds of play; Backspace (or Timeline in
//!    the menu's box) shows them as a tree.
//! 6. A copier save is a snapshot too, marked in the timeline.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ouro_play::records::{Records, clock};
use ouro_play::rewind::Rewind;
use serde::{Deserialize, Serialize};

use harry_core::frame::PlayerGlobals;
use harry_core::game::{Assets, Balloon, FrameEnd, Game, UpLevel};
use harry_core::keys::KeyMap;
use harry_core::savegame::SaveGame;
use harry_data::GameData;
use sdl3::event::Event;
use sdl3::keyboard::{Keycode, Mod, Scancode};
use sdl3::sys::keycode::SDL_KMOD_NONE;

use crate::{FRAME, ScoreShown, TICK, audio, balloon, key_name, mac_code, mac_seconds, menu, options, picture, render, scenes, text, theme};

/// The game's frames a second; a snapshot every 5 seconds, an hour of them
/// kept whole.
const RATE: u32 = 30;
const SNAPSHOT_EVERY: u64 = 5 * RATE as u64;
const SNAPSHOTS: usize = 720;
/// The format of Continue's file and the snapshots (raised when [`Snap`]
/// or the core's state changes so an older file cannot be read).
const FORMAT: u32 = 1;
const SCREEN: ouro_ui::Rect = ouro_ui::Rect::new(0, 0, 640, 480);

/// What Continue and a snapshot keep: the core's game and what the port
/// counts beside it.
#[derive(Clone, Serialize, Deserialize)]
pub struct Snap {
    level: usize,
    game: Game,
    deaths: u32,
    run_frames: u32,
    run_deaths: u32,
    /// The game's frames played (when the next snapshot is due)
    frames: u64,
}

/// What the main loop reads besides the events.
#[derive(Clone, Copy, Default)]
pub struct Input {
    /// The keys held now.
    pub held: KeyMap,
    /// Caps lock is locked.
    pub caps_lock: bool,
}

/// What shows.
pub enum Screen {
    /// A sequence between frames (fades, pictures); the main menu after it,
    /// else the game.
    Scene { scenes: scenes::Scenes, then_menu: Option<menu::Menu> },
    /// The main menu, perhaps with the options or a page over it.
    Menu { menu: menu::Menu, options: Option<options::Screen>, page: Option<MenuPage> },
    /// The game; it waits while something is over it.
    Game(Overlay),
}

/// The port's pages over the main menu.
pub enum MenuPage {
    Levels(ouro_ui::levels::Levels),
    /// The kept game's snapshots
    Timeline(ouro_ui::timeline::Timeline),
}

/// What can be over the game (one at a time; a new one replaces the one
/// before).
pub enum Overlay {
    None,
    /// The options (Command-I).
    Options(options::Screen),
    /// The snapshots (Backspace); the game waits.
    Timeline(ouro_ui::timeline::Timeline),
    /// ShowMap, open since then.
    Map(Instant),
    /// A balloon, shown since then.
    Balloon(balloon::Shown, Instant),
}

pub struct App<'a> {
    data: &'a GameData,
    assets: &'a Assets<'a>,
    fonts: &'a text::Fonts,
    pub settings: options::Settings,
    settings_path: Option<PathBuf>,
    /// The saved game (the record as the original keeps it in its HrSG
    /// resource); continuing needs one saved in this session.
    save_path: Option<PathBuf>,
    have_save: bool,
    pub mixer: audio::Mixer,
    pub game: Game,
    level: usize,
    score: ScoreShown,
    pub screen: Screen,
    /// The next game frame.
    next: Instant,
    /// The next tick of a sequence or the menu.
    tick: Instant,
    /// The screen is faded to black when no sequence runs.
    faded: bool,
    /// The music is faded out (FadeMusic's DAT_10072711).
    music_faded: bool,
    /// Pause (the original's `Pause`): by the pause key - caps lock as in
    /// the original, paused while it is locked; another key by turns - or
    /// when the window loses focus (until a key or a click). Only in the
    /// game.
    pause_toggled: bool,
    focus_paused: bool,
    pub paused: bool,
    /// Frames before the map key opens the map again.
    map_cooldown: u32,
    /// The window's scaling or full screen changed.
    pub display_changed: bool,
    /// Game frames run (for `HARRY_STATS`).
    pub frames_run: u32,
    /// SDL's preference folder: the records, Continue's game (none in
    /// tests).
    folder: Option<PathBuf>,
    records: Records,
    rewind: Rewind<Snap>,
    /// Deaths in this game; the run of this level: its frames and deaths;
    /// the game's frames played.
    deaths: u32,
    run_frames: u32,
    run_deaths: u32,
    frames: u64,
    /// A game is under way (kept when it is left).
    in_game: bool,
    /// The level selection's start views.
    previews: RefCell<HashMap<usize, render::Canvas>>,
}

/// Command-I (Control-I off the Mac) in the game: the options, the game
/// waits.
fn options_mod() -> Mod {
    if cfg!(target_os = "macos") { Mod::LGUIMOD | Mod::RGUIMOD } else { Mod::LCTRLMOD | Mod::RCTRLMOD }
}

/// A new game at `level` (0-based) with Harry at the level's start
/// (`HARRY_START=h,v` puts him elsewhere, for testing).
pub fn start_game(data: &GameData, assets: &Assets, level: usize, g: PlayerGlobals, seed: u32) -> Game {
    let mut game = crate::new_game(data, level, g, seed);
    let (h, v) = game.level.start;
    let (h, v) = std::env::var("HARRY_START")
        .ok()
        .and_then(|s| s.split_once(',').map(|(a, b)| (a.parse().ok(), b.parse().ok())))
        .and_then(|(a, b)| Some((a?, b?)))
        .unwrap_or((h - 32, v - 32));
    let missing = game.start(h, v, game.level.start_dir, assets);
    if !missing.is_empty() {
        eprintln!("level {}: not ported yet: {missing:?}", level + 1);
    }
    game
}

impl<'a> App<'a> {
    /// The main menu, or with `direct` the game at `level` at once.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        data: &'a GameData,
        assets: &'a Assets<'a>,
        fonts: &'a text::Fonts,
        settings: options::Settings,
        settings_path: Option<PathBuf>,
        save_path: Option<PathBuf>,
        mut mixer: audio::Mixer,
        game: Game,
        level: usize,
        direct: bool,
        now: Instant,
    ) -> App<'a> {
        mixer.set_volume(settings.volume);
        mixer.set_music_volume(settings.music_volume);
        let folder = save_path.as_ref().and_then(|p| p.parent()).map(PathBuf::from);
        let mut app = App {
            data,
            assets,
            fonts,
            settings,
            settings_path,
            save_path,
            have_save: false,
            mixer,
            game,
            level,
            score: ScoreShown::default(),
            screen: Screen::Game(Overlay::None),
            next: now,
            tick: now,
            faded: true,
            music_faded: false,
            pause_toggled: false,
            focus_paused: false,
            paused: false,
            map_cooldown: 0,
            display_changed: false,
            frames_run: 0,
            records: Records::load(folder.as_ref().map(|d| d.join("records.json")), RATE),
            folder,
            rewind: Rewind::new(SNAPSHOT_EVERY, SNAPSHOTS),
            deaths: 0,
            run_frames: 0,
            run_deaths: 0,
            frames: 0,
            in_game: direct,
            previews: RefCell::new(HashMap::new()),
        };
        app.screen = if direct {
            let scenes = scenes::Scenes::new(app.start_screen(), true, scenes::game_start(&app.game.level, true, &data.world));
            Screen::Scene { scenes, then_menu: None }
        } else {
            let m = app.new_menu();
            let mut scenes = scenes::Scenes::new(app.start_screen(), true, Vec::new());
            // main: SetMusic(600), the menu
            scenes.then = Some((app.menu_screen(&m), vec![scenes::Step::Music(600), scenes::Step::FadeIn(8)]));
            Screen::Scene { scenes, then_menu: Some(m) }
        };
        app
    }

    fn continue_file(&self) -> Option<PathBuf> {
        self.folder.as_ref().map(|d| d.join("continue.json"))
    }

    fn snapshots_folder(&self) -> Option<PathBuf> {
        self.folder.as_ref().map(|d| d.join("continue"))
    }

    /// The main menu, its box offering Continue and Timeline when a game is
    /// kept.
    fn new_menu(&self) -> menu::Menu {
        let mut m = menu::Menu::new();
        m.kept = self.continue_file().is_some_and(|p| p.exists());
        m.snapshots = self.snapshots_folder().is_some_and(|d| d.join("index.json").exists());
        m
    }

    fn snap(&self) -> Snap {
        Snap { level: self.level, game: self.game.clone(), deaths: self.deaths, run_frames: self.run_frames, run_deaths: self.run_deaths, frames: self.frames }
    }

    fn apply(&mut self, s: Snap) {
        self.level = s.level;
        self.game = s.game;
        self.deaths = s.deaths;
        self.run_frames = s.run_frames;
        self.run_deaths = s.run_deaths;
        self.frames = s.frames;
        self.score = ScoreShown::default();
        self.in_game = true;
    }

    /// A new game (from the menu or the level selection): the counters
    /// from 0, no snapshots.
    fn begin(&mut self, level: usize) {
        self.level = level;
        self.game = start_game(self.data, self.assets, level, PlayerGlobals::new_game(), self.game.rng.seed);
        self.deaths = 0;
        self.run_frames = 0;
        self.run_deaths = 0;
        self.frames = 0;
        self.rewind.clear();
        self.in_game = true;
    }

    /// The game under way kept for Continue (rule 4).
    fn keep(&mut self) {
        if !self.in_game {
            return;
        }
        let (Some(file), Some(dir)) = (self.continue_file(), self.snapshots_folder()) else { return };
        if let Err(e) = ouro_play::saved::save(&file, FORMAT, &self.snap()) {
            eprintln!("{}: {e}", file.display());
        }
        if let Err(e) = self.rewind.save(&dir) {
            eprintln!("{}: {e}", dir.display());
        }
    }

    /// The kept game is done with (won).
    fn forget(&mut self) {
        self.in_game = false;
        if let Some(f) = self.continue_file() {
            let _ = std::fs::remove_file(f);
        }
        if let Some(d) = self.snapshots_folder() {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    /// The kept snapshots (unless they are in memory already).
    fn load_snapshots(&mut self) {
        if !self.rewind.is_empty() {
            return;
        }
        if let Some(d) = self.snapshots_folder() {
            match Rewind::load(&d, SNAPSHOT_EVERY, SNAPSHOTS) {
                Ok(r) => self.rewind = r,
                Err(e) => eprintln!("{}: {e}", d.display()),
            }
        }
    }

    /// Continue: the kept game, if there is one that can be read.
    fn load_kept(&mut self) -> bool {
        let Some(file) = self.continue_file() else { return false };
        let Some(snap) = ouro_play::saved::load::<Snap>(&file, ouro_play::saved::only(FORMAT)) else { return false };
        self.rewind = Rewind::new(SNAPSHOT_EVERY, SNAPSHOTS);
        self.load_snapshots();
        self.apply(snap);
        true
    }

    /// From the game as it is (Continue, rewind): its music, a fade in.
    fn resume(&mut self, now: Instant) {
        let scenes = scenes::Scenes::new(render::draw(self.data, &self.game), self.faded, scenes::resume(&self.game.level));
        self.screen = Screen::Scene { scenes, then_menu: None };
        self.tick = now;
    }

    /// Play on from snapshot `k`.
    fn rewind_to(&mut self, k: usize, now: Instant) {
        if let Some(s) = self.rewind.restore(k) {
            self.apply(s);
            self.resume(now);
        }
    }

    /// The timeline's line about the game now.
    fn snapshot_info(&self) -> String {
        format!(
            "Level {} {}   {}   {} {}",
            self.level + 1,
            self.game.level.name,
            clock(self.run_frames, RATE),
            self.deaths,
            if self.deaths == 1 { "death" } else { "deaths" }
        )
    }

    /// A snapshot now; `manual`: a copier save (rule 6).
    fn take_snapshot(&mut self, manual: bool) {
        let picture = theme::half_png(&render::draw(self.data, &self.game));
        let (snap, frames) = (self.snap(), self.frames);
        if manual {
            let info = format!("Copier save   {}", self.snapshot_info());
            self.rewind.save_point(frames, &snap, picture, info);
        } else {
            let info = self.snapshot_info();
            self.rewind.take(frames, &snap, picture, info);
        }
    }

    /// Level `level`'s start view (the level selection's preview).
    fn preview(&self, level: usize) -> render::Canvas {
        self.previews
            .borrow_mut()
            .entry(level)
            .or_insert_with(|| render::draw(self.data, &start_game(self.data, self.assets, level, PlayerGlobals::new_game(), 1)))
            .clone()
    }

    /// Rule 1: after a death the level again, from the last copier save
    /// in it (the record the original continues from), else from its
    /// start (its meters, score, ammunition and items as it began).
    fn restart_after_death(&mut self, now: Instant) {
        let screen = render::draw(self.data, &self.game);
        let current = self.game.counter.current;
        let mut save = self.game.session.save.clone();
        if !(save.copier && save.level == current) {
            let st = &self.game.level_start;
            save.copier = false;
            save.level = current;
            save.world = self.game.session.world_id;
            save.world_name = self.game.session.world_name.clone();
            (save.score, save.comfort, save.surface, save.favor) = (st.score, st.comfort, st.surface, st.favor);
            (save.ammo, save.inventory) = (st.ammo, st.inventory);
        }
        if let Some(missing) = self.game.continue_game(&save, &self.data.levels, self.assets)
            && !missing.is_empty()
        {
            eprintln!("level {current}: not ported yet: {missing:?}");
        }
        let scenes = scenes::Scenes::new(screen, self.faded, scenes::after_death(&self.game.level, &self.data.world));
        self.screen = Screen::Scene { scenes, then_menu: None };
        self.tick = now;
    }

    /// The game's screen with the game area white (under a level's start).
    fn start_screen(&self) -> render::Canvas {
        let mut c = render::draw(self.data, &self.game);
        c.fill(16, 16, 448, 448, 255);
        c
    }

    fn menu_screen(&self, m: &menu::Menu) -> render::Canvas {
        m.draw(self.data, self.fonts).0
    }

    /// The game's start (with the story when `intro`), then the game.
    fn show_game_start(&mut self, intro: bool, now: Instant) {
        let scenes = scenes::Scenes::new(self.start_screen(), self.faded, scenes::game_start(&self.game.level, intro, &self.data.world));
        self.screen = Screen::Scene { scenes, then_menu: None };
        self.tick = now;
    }

    /// To the main menu from the screen shown: fade out, the menu, fade in.
    fn back_to_menu(&mut self, screen: render::Canvas, steps: Vec<scenes::Step>, now: Instant) {
        let m = self.new_menu();
        let mut steps = steps;
        steps.push(scenes::Step::FadeOut(8));
        let mut scenes = scenes::Scenes::new(screen, self.faded, steps);
        let mut then = vec![scenes::Step::FadeIn(8)];
        then.extend(scenes::menu_music());
        scenes.then = Some((self.menu_screen(&m), then));
        self.screen = Screen::Scene { scenes, then_menu: Some(m) };
        self.tick = now;
    }

    /// The options screen, wherever it is open.
    fn options_mut(&mut self) -> Option<&mut options::Screen> {
        match &mut self.screen {
            Screen::Menu { options: Some(o), .. } | Screen::Game(Overlay::Options(o)) => Some(o),
            _ => None,
        }
    }

    fn options(&self) -> Option<&options::Screen> {
        match &self.screen {
            Screen::Menu { options: Some(o), .. } | Screen::Game(Overlay::Options(o)) => Some(o),
            _ => None,
        }
    }

    fn close_options(&mut self) {
        match &mut self.screen {
            Screen::Menu { options, .. } => *options = None,
            Screen::Game(o @ Overlay::Options(_)) => *o = Overlay::None,
            _ => {}
        }
    }

    fn save_settings(&self) {
        if let Some(p) = &self.settings_path
            && let Err(err) = self.settings.save(p)
        {
            eprintln!("{}: {err}", p.display());
        }
    }

    /// One pass of the main loop: the events, then the sequences, the menu
    /// and the game frames due by `now`. False: quit.
    pub fn step(&mut self, events: impl IntoIterator<Item = Event>, input: &Input, now: Instant) -> bool {
        let data = self.data;
        let mut jump: Option<usize> = None;
        // a key or mouse button pressed now (it can end a scrolled picture;
        // one held since before cannot)
        let mut pressed = false;
        let mut action: Option<menu::Action> = None;
        let mut opt_event: Option<options::Event> = None;
        // the game left (Escape): kept, the main menu
        let mut leave = false;
        // play on from this snapshot
        let mut rewind: Option<usize> = None;
        for e in events {
            let in_game = matches!(self.screen, Screen::Game(_));
            match e {
                Event::Quit { .. } => {
                    self.keep();
                    return false;
                }
                Event::Window { win_event: sdl3::event::WindowEvent::FocusLost, .. } => self.focus_paused = true,
                // a key or a click ends the pause of a lost focus (and does
                // nothing else)
                Event::KeyDown { .. } | Event::MouseButtonDown { .. } if self.focus_paused && in_game => self.focus_paused = false,
                Event::MouseButtonDown { x, y, .. } => {
                    pressed = true;
                    let (x, y) = (x as i32, y as i32);
                    let levels = data.levels.len();
                    let tree = self.rewind.tree();
                    if let Some(o) = self.options_mut() {
                        opt_event = opt_event.or(o.mouse(x, y, true));
                    } else {
                        match &mut self.screen {
                            Screen::Menu { page: Some(MenuPage::Levels(l)), .. } => {
                                if let Some(ouro_ui::levels::Choice::Take(k)) = l.mouse(x, y, true, levels, SCREEN) {
                                    jump = Some(k);
                                }
                            }
                            Screen::Menu { page: Some(MenuPage::Timeline(t)), .. } | Screen::Game(Overlay::Timeline(t)) => {
                                if let Some(ouro_ui::timeline::Choice::Play(k)) = t.mouse(x, y, true, &tree, SCREEN) {
                                    rewind = Some(k);
                                }
                            }
                            Screen::Menu { menu, .. } => action = action.or(menu.mouse_down(x, y)),
                            _ => {}
                        }
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    let (x, y) = (x as i32, y as i32);
                    let levels = data.levels.len();
                    let tree = self.rewind.tree();
                    if let Some(o) = self.options_mut() {
                        o.mouse(x, y, false);
                    } else {
                        match &mut self.screen {
                            Screen::Menu { page: Some(MenuPage::Levels(l)), .. } => {
                                l.mouse(x, y, false, levels, SCREEN);
                            }
                            Screen::Menu { page: Some(MenuPage::Timeline(t)), .. } | Screen::Game(Overlay::Timeline(t)) => {
                                t.mouse(x, y, false, &tree, SCREEN);
                            }
                            Screen::Menu { menu, .. } => menu.mouse_move(x, y),
                            _ => {}
                        }
                    }
                }
                Event::MouseWheel { y, .. } => {
                    if let Screen::Menu { page: Some(MenuPage::Levels(l)), .. } = &mut self.screen {
                        l.wheel(y.round() as i32, data.levels.len(), SCREEN);
                    }
                }
                Event::MouseButtonUp { x, y, .. } => {
                    if let Screen::Menu { menu, .. } = &mut self.screen {
                        action = action.or(menu.mouse_up(x as i32, y as i32));
                    }
                }
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } if self.options().is_some() => {
                    if let Some(o) = self.options_mut() {
                        opt_event = opt_event.or(o.key(sc, mac_code(sc)));
                    }
                }
                // the level selection and the timeline (over the menu or the game)
                Event::KeyDown { scancode: Some(sc), .. }
                    if matches!(self.screen, Screen::Menu { page: Some(_), .. } | Screen::Game(Overlay::Timeline(_))) =>
                {
                    let levels = data.levels.len();
                    let tree = self.rewind.tree();
                    let Some(k) = theme::ui_key(sc) else { continue };
                    match &mut self.screen {
                        Screen::Menu { page: page @ Some(MenuPage::Levels(_)), .. } => {
                            let Some(MenuPage::Levels(l)) = page else { continue };
                            match l.key(k, levels, SCREEN) {
                                Some(ouro_ui::levels::Choice::Take(n)) => jump = Some(n),
                                Some(ouro_ui::levels::Choice::Back) => *page = None,
                                None => {}
                            }
                        }
                        Screen::Menu { page: page @ Some(MenuPage::Timeline(_)), .. } => {
                            let Some(MenuPage::Timeline(t)) = page else { continue };
                            match t.key(k, &tree) {
                                Some(ouro_ui::timeline::Choice::Play(n)) => rewind = Some(n),
                                Some(ouro_ui::timeline::Choice::Back) => *page = None,
                                None => {}
                            }
                        }
                        Screen::Game(o @ Overlay::Timeline(_)) => {
                            let Overlay::Timeline(t) = o else { continue };
                            match t.key(k, &tree) {
                                Some(ouro_ui::timeline::Choice::Play(n)) => rewind = Some(n),
                                Some(ouro_ui::timeline::Choice::Back) => {
                                    *o = Overlay::None;
                                    self.next = now;
                                }
                                None => {}
                            }
                        }
                        _ => {}
                    }
                }
                Event::KeyDown { scancode: Some(sc), repeat: false, .. }
                    if matches!(self.screen, Screen::Menu { .. }) && sc != self.settings.fullscreen_key =>
                {
                    let Screen::Menu { menu: m, .. } = &mut self.screen else { continue };
                    if sc == Scancode::Escape {
                        if m.message.is_none() && !m.credits_open() {
                            return false;
                        }
                        m.key('\x1b');
                    } else if let Some(c) =
                        Keycode::from_scancode(sc, SDL_KMOD_NONE, false).and_then(|k| char::from_u32(k as u32)).filter(char::is_ascii_lowercase)
                    {
                        action = action.or(m.key(c));
                    } else if let Some(k) = theme::ui_key(sc) {
                        action = action.or(m.arrow(k));
                    }
                }
                Event::KeyDown { scancode: Some(sc), keymod, repeat: false, .. } => {
                    if let Screen::Game(Overlay::Balloon(_, since)) = &self.screen {
                        // Return / Enter at once, any key after a second
                        if matches!(sc, Scancode::Return | Scancode::KpEnter) || now.saturating_duration_since(*since) >= Duration::from_secs(1) {
                            self.screen = Screen::Game(Overlay::None);
                            self.next = now;
                        }
                        continue;
                    }
                    let scene = matches!(self.screen, Screen::Scene { .. });
                    match sc {
                        _ if scene && sc != self.settings.fullscreen_key => pressed = true,
                        // CheckGameLoopKeys: Escape asks before the game is given
                        // up; the port keeps it instead (rule 4)
                        Scancode::Escape if in_game => leave = true,
                        // the timeline (rule 5); the game waits
                        Scancode::Backspace if in_game => {
                            let mut view = ouro_ui::timeline::Timeline::default();
                            view.open(&self.rewind.tree(), self.rewind.current());
                            self.screen = Screen::Game(Overlay::Timeline(view));
                        }
                        // Command-I: the options
                        Scancode::I if in_game && keymod.intersects(options_mod()) => {
                            self.screen = Screen::Game(Overlay::Options(options::Screen::new(self.settings.clone())));
                        }
                        _ if sc == self.settings.pause_key && sc != Scancode::CapsLock => self.pause_toggled = !self.pause_toggled,
                        _ if sc == self.settings.fullscreen_key => {
                            self.settings.fullscreen = !self.settings.fullscreen;
                            self.display_changed = true;
                            if let Some(p) = &self.settings_path {
                                let _ = self.settings.save(p);
                            }
                        }
                        Scancode::F8 if in_game => jump = Some((self.level + 1) % data.levels.len()),
                        Scancode::F7 if in_game => jump = Some((self.level + data.levels.len() - 1) % data.levels.len()),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if let Some(n) = jump {
            // F7 / F8, or a level of the level selection: a game from it
            let from_menu = matches!(self.screen, Screen::Menu { .. });
            self.begin(n);
            self.show_game_start(from_menu, now);
        }
        if let Some(k) = rewind {
            self.rewind_to(k, now);
        }
        if let Screen::Scene { scenes: sc, then_menu } = &mut self.screen {
            sc.music_faded = self.music_faded;
            while self.tick <= now && !sc.done() {
                sc.tick(data, std::mem::take(&mut pressed));
                for c in std::mem::take(&mut sc.music) {
                    match c {
                        scenes::MusicCommand::Play(id) => self.mixer.music(id),
                        scenes::MusicCommand::Fade { out, ticks } => self.mixer.fade_music(out, ticks),
                    }
                }
                self.tick += TICK;
            }
            self.music_faded = sc.music_faded;
            if sc.done() {
                self.faded = sc.dark >= 256;
                self.screen = match then_menu.take() {
                    Some(menu) => Screen::Menu { menu, options: None, page: None },
                    None => Screen::Game(Overlay::None),
                };
                self.next = now;
            }
        }
        // the main menu: its ticks, sounds and what it asks for
        if let Screen::Menu { menu: m, .. } = &mut self.screen {
            while self.tick <= now {
                action = action.or(m.tick(data));
                self.tick += TICK;
            }
            self.mixer.events(&std::mem::take(&mut m.sounds), false);
        }
        if let Some(e) = opt_event
            && let Some(o) = self.options()
        {
            self.settings = o.settings.clone();
            self.display_changed = true;
            self.mixer.set_volume(self.settings.volume);
            self.mixer.set_music_volume(self.settings.music_volume);
            if e == options::Event::Done {
                self.save_settings();
                self.close_options();
                self.next = now;
            }
        }
        match action {
            Some(menu::Action::Options) => {
                if let Screen::Menu { options, .. } = &mut self.screen {
                    *options = Some(options::Screen::new(self.settings.clone()));
                }
            }
            Some(menu::Action::NewGame) => {
                self.begin(0);
                self.show_game_start(true, now);
            }
            Some(menu::Action::Levels) => {
                let mut l = ouro_ui::levels::Levels::default();
                l.select(self.level.min(data.levels.len() - 1), SCREEN);
                if let Screen::Menu { page, .. } = &mut self.screen {
                    *page = Some(MenuPage::Levels(l));
                }
            }
            Some(menu::Action::Timeline) => {
                self.load_snapshots();
                let mut view = ouro_ui::timeline::Timeline::default();
                view.open(&self.rewind.tree(), self.rewind.current());
                if let Screen::Menu { page, .. } = &mut self.screen {
                    *page = Some(MenuPage::Timeline(view));
                }
            }
            // Continue (rule 4): the game kept when it was left
            Some(menu::Action::Continue) if self.load_kept() => self.resume(now),
            // else ContinueGame with the record saved in this session (a
            // copier save without the story)
            Some(menu::Action::Continue) => {
                // HARRY_SAVE=<file> continues from that record instead (for
                // testing, e.g. the saves of `ouro-harry-repro`)
                let saved = match std::env::var_os("HARRY_SAVE") {
                    Some(p) => std::fs::read(p).ok(),
                    None => self.save_path.as_ref().filter(|_| self.have_save).and_then(|p| std::fs::read(p).ok()),
                };
                if let Some(sg) = saved.map(|b| SaveGame::from_bytes(&b))
                    && let Some(missing) = self.game.continue_game(&sg, &data.levels, self.assets)
                {
                    self.level = (sg.level as usize).saturating_sub(1);
                    if !missing.is_empty() {
                        eprintln!("level {}: not ported yet: {missing:?}", self.level + 1);
                    }
                    self.show_game_start(!sg.copier, now);
                }
            }
            Some(menu::Action::Quit) => {
                self.keep();
                return false;
            }
            None => {}
        }
        if leave {
            self.keep();
            self.in_game = false;
            self.back_to_menu(render::draw(data, &self.game), Vec::new(), now);
        }
        // the map closes with its key once 12 ticks have passed
        if let Screen::Game(Overlay::Map(since)) = self.screen
            && now.saturating_duration_since(since) >= TICK * 12
            && input.held.down(self.settings.map_key)
        {
            self.screen = Screen::Game(Overlay::None);
            self.next = now;
        }
        // the pause: its sound (snd 198) as it starts; when it ends the
        // game goes on from now
        let in_game = matches!(self.screen, Screen::Game(_));
        let caps = self.settings.pause_key == Scancode::CapsLock && input.caps_lock;
        let paused = in_game && (caps || self.pause_toggled || self.focus_paused);
        if paused && !self.paused {
            self.mixer.events(
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
        } else if !paused && self.paused {
            self.next = now;
        }
        self.paused = paused;
        if !in_game {
            (self.pause_toggled, self.focus_paused) = (false, false);
        }
        // the settings the game reads
        self.game.prefs.scroll = self.settings.camera;
        self.game.prefs.autosave = self.settings.autosave;
        self.game.prefs.bindings = self.settings.bindings;
        while matches!(self.screen, Screen::Game(Overlay::None)) && !paused && self.next <= now {
            self.next += FRAME;
            self.game_frame(&input.held, now);
        }
        true
    }

    /// One frame of the game with the keys held.
    fn game_frame(&mut self, keys: &KeyMap, now: Instant) {
        let (data, fonts) = (self.data, self.fonts);
        let mut asked = None;
        let mut show = |b: &Balloon| {
            asked = balloon::place(b, data, fonts);
            asked.is_some()
        };
        // CopierSave's dialog is not ported: at the copier Harry saves
        let mut save = || true;
        self.game.audio.playing = self.mixer.playing();
        let end = self.game.frame(keys, self.assets, &mut show, &mut save);
        self.frames_run += 1;
        self.frames += 1;
        self.run_frames += 1;
        // CheckGameLoopKeys: the map key (not again for 6 passes)
        self.map_cooldown = self.map_cooldown.saturating_sub(1);
        if keys.down(self.settings.map_key) && self.map_cooldown == 0 {
            self.screen = Screen::Game(Overlay::Map(now));
            self.map_cooldown = 6;
        }
        self.mixer.events(&self.game.sounds, self.game.level.own_sounds);
        if self.game.sounds.iter().any(|e| matches!(e, harry_core::sound::SoundEvent::FadeMusicOut(_))) {
            self.music_faded = true;
        }
        self.score.step(self.game.g.score);
        if let Some(b) = asked {
            self.screen = Screen::Game(Overlay::Balloon(b, now));
        }
        match end {
            FrameEnd::Running => {
                // a copier save is a snapshot of its own (rule 6)
                if self.game.session.save_pending && self.game.session.save.copier {
                    self.take_snapshot(true);
                } else if self.rewind.due(self.frames) {
                    self.take_snapshot(false);
                }
            }
            FrameEnd::LevelDone => {
                // the level's run (rule 3)
                self.records.add(self.level as i32 + 1, self.run_frames, self.run_deaths);
                self.run_frames = 0;
                self.run_deaths = 0;
                // the screen as the level ended, under the sequence
                let (screen, old) = (render::draw(data, &self.game), self.game.level.clone());
                let heard = self.game.sounds.len();
                let up = self.game.up_level(&data.levels, self.assets);
                // UpLevel's jingle (4800)
                self.mixer.events(&self.game.sounds[heard..], old.own_sounds);
                match up {
                    (UpLevel::Next, missing) => {
                        self.level = (self.game.counter.current as usize).saturating_sub(1);
                        if !missing.is_empty() {
                            eprintln!("level {}: not ported yet: {missing:?}", self.level + 1);
                        }
                        let scenes = scenes::Scenes::new(screen, self.faded, scenes::up_level(&old, &self.game.level, &data.world));
                        self.screen = Screen::Scene { scenes, then_menu: None };
                        self.tick = now;
                    }
                    // after Win the main menu; the game is over
                    (UpLevel::Won, _) => {
                        self.forget();
                        self.back_to_menu(screen, scenes::win(&data.world), now);
                    }
                }
            }
            // GameLoop's end (the original: EndLevelEffect, the main menu,
            // AskToContinue): the level again (rule 1)
            FrameEnd::Died => {
                self.deaths += 1;
                self.run_deaths += 1;
                self.restart_after_death(now);
            }
        }
        if std::mem::take(&mut self.game.session.save_pending) {
            // SaveSG: the record, dated, to the save file
            self.game.session.save.date = mac_seconds();
            if let Some(p) = &self.save_path {
                match std::fs::write(p, self.game.session.save.to_bytes()) {
                    Ok(()) => self.have_save = true,
                    Err(e) => eprintln!("{}: {e}", p.display()),
                }
            }
        }
    }

    /// The picture to show, its texts and how dark (of 256) it is.
    pub fn compose(&self) -> (render::Canvas, Vec<text::Line>, i32) {
        let (data, fonts) = (self.data, self.fonts);
        let dark = if self.faded { 256 } else { 0 };
        match &self.screen {
            Screen::Scene { scenes: sc, .. } => {
                // the sequence's picture; the panel's texts when it shows them
                let lines = if sc.texts { picture(data, &self.game, &self.score, None, self.deaths).1 } else { Vec::new() };
                (sc.image.clone(), lines, sc.dark)
            }
            Screen::Menu { menu, options, page } => {
                let look = theme::Look { fonts };
                if let Some(o) = options {
                    let mut c = render::Canvas::new(640, 480, 255);
                    let l = o.draw(&mut c, &look, &key_name);
                    return (c, l, dark);
                }
                match page {
                    Some(MenuPage::Levels(levels)) => {
                        let mut c = render::Canvas::new(640, 480, 255);
                        let rows: Vec<_> = data
                            .levels
                            .iter()
                            .enumerate()
                            .map(|(k, l)| ouro_ui::levels::Row { number: Some(k as i32 + 1), name: l.name.clone(), done: self.records.done(k as i32 + 1), locked: false })
                            .collect();
                        let k = levels.selected.min(data.levels.len() - 1);
                        let preview = self.preview(k);
                        let l = levels.draw(&look, &mut c, SCREEN, "Levels", &rows, &self.records.lines(k as i32 + 1), |c, r| theme::scaled_into(c, r, &preview));
                        (c, l, dark)
                    }
                    Some(MenuPage::Timeline(view)) => self.timeline_picture(view, &look, dark),
                    None => {
                        let (c, l) = menu.draw(data, fonts);
                        (c, l, dark)
                    }
                }
            }
            Screen::Game(overlay) => {
                let shown = match overlay {
                    Overlay::Balloon(b, _) => Some(b),
                    _ => None,
                };
                if let Overlay::Timeline(view) = overlay {
                    return self.timeline_picture(view, &theme::Look { fonts }, dark);
                }
                let (mut c, mut l) = picture(data, &self.game, &self.score, shown, self.deaths);
                if let Overlay::Options(o) = overlay {
                    // over the whole screen; the game waits
                    c = render::Canvas::new(640, 480, 255);
                    l = o.draw(&mut c, &theme::Look { fonts }, &key_name);
                } else if self.paused
                    && let Some(p) = data.pictures.get(&4921)
                {
                    self.draw_pause(&mut c, &mut l, p);
                }
                if let Overlay::Map(_) = overlay {
                    render::draw_map(&mut c, data, &self.game);
                }
                (c, l, dark)
            }
        }
    }

    /// The timeline over the whole screen: the chosen snapshot's picture.
    fn timeline_picture(&self, view: &ouro_ui::timeline::Timeline, look: &theme::Look, dark: i32) -> (render::Canvas, Vec<text::Line>, i32) {
        let mut c = render::Canvas::new(640, 480, 255);
        let chosen = view.chosen.and_then(|k| self.rewind.get(k)).and_then(|n| theme::from_png(&n.picture));
        let info = view.chosen.map(|k| self.rewind.info(k)).unwrap_or_default();
        let l = view.draw(look, &mut c, SCREEN, &self.rewind.tree(), self.rewind.current(), &info, |c, r| {
            if let Some(p) = &chosen {
                theme::scaled_into(c, r, p);
            }
        });
        (c, l, dark)
    }

    /// Pause: PICT 4921 at (128, 192), over the game area's middle, with its
    /// "Press CAPS LOCK To Resume" (rows 64–82) replaced when something else
    /// goes on.
    fn draw_pause(&self, c: &mut render::Canvas, l: &mut Vec<text::Line>, p: &harry_data::Image) {
        c.draw_picture(p, 128, 192);
        let hint = if self.focus_paused {
            Some("Press any key to resume".to_string())
        } else if self.settings.pause_key != Scancode::CapsLock {
            Some(format!("Press {} to resume", self.settings.pause_key.name()))
        } else {
            None
        };
        if let Some(hint) = hint {
            if let Some(grey) = p.get(20, 60) {
                c.fill(128 + 6, 192 + 62, 213, 22, grey);
            }
            let w = self.fonts.width(text::Face::HelveticaBold, 14.0, &hint);
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

    /// What shows, for the tests: "scene", "menu" (with "+options") or
    /// "game" with what is over it.
    #[cfg(test)]
    pub fn describe(&self) -> String {
        match &self.screen {
            Screen::Scene { .. } => "scene".into(),
            Screen::Menu { options, page, .. } => match (options, page) {
                (Some(_), _) => "menu+options",
                (None, Some(MenuPage::Levels(_))) => "menu+levels",
                (None, Some(MenuPage::Timeline(_))) => "menu+timeline",
                (None, None) => "menu",
            }
            .into(),
            Screen::Game(o) => {
                let paused = if self.paused { "+paused" } else { "" };
                match o {
                    Overlay::None => format!("game{paused}"),
                    Overlay::Options(_) => format!("game+options{paused}"),
                    Overlay::Timeline(_) => format!("game+timeline{paused}"),
                    Overlay::Map(_) => format!("game{paused}+map"),
                    Overlay::Balloon(..) => format!("game{paused}+balloon"),
                }
            }
        }
    }

    /// When the main loop should run again: the next tick of a sequence or
    /// the menu, else the next game frame.
    pub fn wake(&self) -> Instant {
        if matches!(self.screen, Screen::Game(_)) { self.next } else { self.tick }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{Hash, Hasher};

    fn key(sc: Scancode, keymod: Mod) -> Event {
        Event::KeyDown { timestamp: 0, window_id: 0, keycode: None, scancode: Some(sc), keymod, repeat: false, which: 0, raw: 0 }
    }

    /// Runs an `App` on a made-up clock; `APP_TRACE=1` prints what shows
    /// and a hash of the picture after each step (to compare two versions).
    struct Driver<'a> {
        app: App<'a>,
        now: Instant,
        input: Input,
        steps: usize,
    }

    impl Driver<'_> {
        fn step(&mut self, events: Vec<Event>) -> bool {
            let go = self.app.step(events, &self.input, self.now);
            self.steps += 1;
            if std::env::var_os("APP_TRACE").is_some() {
                let (image, lines, dark) = self.app.compose();
                let mut h = std::collections::hash_map::DefaultHasher::new();
                image.pixels.hash(&mut h);
                format!("{lines:?}").hash(&mut h);
                dark.hash(&mut h);
                eprintln!("TRACE {} {} {:016x}", self.steps, self.app.describe(), h.finish());
            }
            go
        }

        /// Time passes, a tick at a time.
        fn run(&mut self, d: Duration) {
            let end = self.now + d;
            while self.now < end {
                self.now += TICK;
                assert!(self.step(Vec::new()));
            }
        }

        fn press(&mut self, sc: Scancode, keymod: Mod) -> bool {
            self.step(vec![key(sc, keymod)])
        }

        /// Until the game shows (keys end the pictures), at most `limit`.
        fn until_game(&mut self, limit: Duration) {
            let end = self.now + limit;
            while self.app.describe() != "game" {
                assert!(self.now < end, "still {} after {limit:?}", self.app.describe());
                self.run(Duration::from_millis(500));
                if self.app.describe() == "scene" {
                    self.press(Scancode::Space, Mod::NOMOD);
                }
            }
        }
    }

    #[test]
    fn a_game_from_the_menu_with_everything_over_it() {
        let Ok(data) = harry_data::pack::read(crate::BUNDLED) else { return };
        let loaded = crate::Loaded::new(&data).unwrap();
        let face_bounds = |f: i16| data.faces.get(&f).map_or([0; 4], |x| x.bounds);
        let assets = loaded.assets(&face_bounds);
        let fonts = text::Fonts::load();
        let t0 = Instant::now();
        let game = start_game(&data, &assets, 0, PlayerGlobals::new_game(), 1);
        let settings = options::Settings::default();
        let app = App::new(&data, &assets, &fonts, settings, None, None, audio::Mixer::silent(&data), game, 0, false, t0);
        let mut d = Driver { app, now: t0, input: Input::default(), steps: 0 };
        let cmd = options_mod();

        // the main menu fades in
        assert_eq!(d.app.describe(), "scene");
        d.run(Duration::from_secs(3));
        assert_eq!(d.app.describe(), "menu");
        // Preferences: the options over the menu, closed with Escape
        d.press(Scancode::P, Mod::NOMOD);
        d.run(Duration::from_secs(1));
        assert_eq!(d.app.describe(), "menu+options");
        d.press(Scancode::Escape, Mod::NOMOD);
        assert_eq!(d.app.describe(), "menu");

        // New Game: the story and the level's start, then the game
        d.press(Scancode::N, Mod::NOMOD);
        d.run(Duration::from_secs(1));
        assert_eq!(d.app.describe(), "scene");
        d.until_game(Duration::from_secs(120));
        let frames = |d: &Driver| d.app.frames_run;
        let f = frames(&d);
        d.run(Duration::from_secs(1));
        assert!(frames(&d) >= f + 29, "the game runs at 30 frames a second ({} in a second)", frames(&d) - f);

        // Backspace: the timeline (a snapshot at the start), the game waits;
        // Escape goes on
        d.press(Scancode::Backspace, Mod::NOMOD);
        assert_eq!(d.app.describe(), "game+timeline");
        assert!(!d.app.rewind.is_empty(), "a snapshot at the level's start");
        let f = frames(&d);
        d.run(Duration::from_millis(500));
        assert_eq!(frames(&d), f, "the game waits");
        d.press(Scancode::Escape, Mod::NOMOD);
        d.run(Duration::from_millis(500));
        assert_eq!(d.app.describe(), "game");
        assert!(frames(&d) > f);

        // caps lock pauses while locked
        d.input.caps_lock = true;
        d.run(Duration::from_millis(100));
        assert_eq!(d.app.describe(), "game+paused");
        let f = frames(&d);
        d.run(Duration::from_millis(500));
        assert_eq!(frames(&d), f);
        d.input.caps_lock = false;
        d.run(Duration::from_millis(500));
        assert_eq!(d.app.describe(), "game");
        assert!(frames(&d) > f);

        // losing the focus pauses until a key
        d.step(vec![Event::Window { timestamp: 0, window_id: 0, win_event: sdl3::event::WindowEvent::FocusLost }]);
        d.run(Duration::from_millis(100));
        assert_eq!(d.app.describe(), "game+paused");
        d.press(Scancode::LShift, Mod::NOMOD);
        d.run(Duration::from_millis(100));
        assert_eq!(d.app.describe(), "game");

        // Command-I: the options over the game, which waits
        d.press(Scancode::I, cmd);
        assert_eq!(d.app.describe(), "game+options");
        let f = frames(&d);
        d.run(Duration::from_millis(500));
        assert_eq!(frames(&d), f);
        d.press(Scancode::Escape, Mod::NOMOD);
        d.run(Duration::from_millis(100));
        assert_eq!(d.app.describe(), "game");

        // the map opens with its key and closes with it after 12 ticks
        d.input.held.press(d.app.settings.map_key);
        d.run(Duration::from_millis(100));
        d.input.held = KeyMap::default();
        assert_eq!(d.app.describe(), "game+map");
        let f = frames(&d);
        d.run(Duration::from_secs(1));
        assert_eq!((d.app.describe().as_str(), frames(&d)), ("game+map", f));
        d.input.held.press(d.app.settings.map_key);
        d.run(Duration::from_millis(50));
        d.input.held = KeyMap::default();
        assert_eq!(d.app.describe(), "game");

        // F8: the next level, after its start
        d.press(Scancode::F8, Mod::NOMOD);
        assert_eq!(d.app.describe(), "scene");
        assert_eq!(d.app.game.counter.current, 2);
        d.until_game(Duration::from_secs(60));

        // Escape: the game is left (kept for Continue), back to the menu;
        // L the level selection, Escape back
        d.press(Scancode::Escape, Mod::NOMOD);
        assert_eq!(d.app.describe(), "scene");
        d.run(Duration::from_secs(3));
        assert_eq!(d.app.describe(), "menu");
        d.press(Scancode::L, Mod::NOMOD);
        assert_eq!(d.app.describe(), "menu+levels");
        d.press(Scancode::Escape, Mod::NOMOD);
        assert_eq!(d.app.describe(), "menu");
        // Escape in the menu quits
        assert!(!d.press(Scancode::Escape, Mod::NOMOD));
    }

    /// A game kept as JSON plays on exactly as the one it was made from
    /// (Continue and rewind keep the core's state so).
    #[test]
    fn a_kept_game_plays_on_the_same() {
        let Ok(data) = harry_data::pack::read(crate::BUNDLED) else { return };
        let loaded = crate::Loaded::new(&data).unwrap();
        let face_bounds = |f: i16| data.faces.get(&f).map_or([0; 4], |x| x.bounds);
        let assets = loaded.assets(&face_bounds);
        let mut game = start_game(&data, &assets, 2, PlayerGlobals::new_game(), 7);
        let b = game.prefs.bindings;
        let keys = [b.left, b.right, b.scoot, b.inch, b.kick];
        let held = |f: usize| {
            let mut k = KeyMap::default();
            k.press(keys[f / 13 % keys.len()]);
            k
        };
        let mut no_balloon = |_: &Balloon| false;
        let mut save = || true;
        for f in 0..300 {
            game.frame(&held(f), &assets, &mut no_balloon, &mut save);
        }
        let json = serde_json::to_vec(&game).unwrap();
        eprintln!("a game as JSON: {} bytes", json.len());
        let packed = miniz_oxide_size(&json);
        eprintln!("deflated: {packed} bytes");
        let mut copy: Game = serde_json::from_slice(&json).unwrap();
        for f in 300..700 {
            game.frame(&held(f), &assets, &mut no_balloon, &mut save);
            copy.frame(&held(f), &assets, &mut no_balloon, &mut save);
        }
        assert_eq!(serde_json::to_vec(&game).unwrap(), serde_json::to_vec(&copy).unwrap());
    }

    fn miniz_oxide_size(json: &[u8]) -> usize {
        let mut r: ouro_play::rewind::Rewind<serde_json::Value> = ouro_play::rewind::Rewind::new(1, 1);
        r.take(0, &serde_json::from_slice(json).unwrap(), Vec::new(), String::new());
        r.get(0).unwrap().state.as_ref().unwrap().len()
    }

    /// Rule 1: a death starts the level again (no game over), counted;
    /// rule 3: the level selection starts a level.
    #[test]
    fn a_death_starts_the_level_again() {
        let Ok(data) = harry_data::pack::read(crate::BUNDLED) else { return };
        let loaded = crate::Loaded::new(&data).unwrap();
        let face_bounds = |f: i16| data.faces.get(&f).map_or([0; 4], |x| x.bounds);
        let assets = loaded.assets(&face_bounds);
        let fonts = text::Fonts::load();
        let t0 = Instant::now();
        let game = start_game(&data, &assets, 0, PlayerGlobals::new_game(), 1);
        let app = App::new(&data, &assets, &fonts, options::Settings::default(), None, None, audio::Mixer::silent(&data), game, 0, false, t0);
        let mut d = Driver { app, now: t0, input: Input::default(), steps: 0 };
        d.run(Duration::from_secs(3));
        // the level selection: level 3
        d.press(Scancode::L, Mod::NOMOD);
        d.press(Scancode::Down, Mod::NOMOD);
        d.press(Scancode::Down, Mod::NOMOD);
        d.press(Scancode::Return, Mod::NOMOD);
        d.until_game(Duration::from_secs(120));
        assert_eq!(d.app.game.counter.current, 3);
        let score = d.app.game.g.score;
        // comfort gone: Harry dies
        d.app.game.g.comfort = 0;
        // the meter runs down, Harry's death takes 110 frames
        for _ in 0..100 {
            if d.app.deaths > 0 {
                break;
            }
            d.run(Duration::from_millis(100));
        }
        assert_eq!(d.app.deaths, 1);
        assert_eq!(d.app.describe(), "scene", "EndLevelEffect and the level's start");
        d.until_game(Duration::from_secs(120));
        assert_eq!((d.app.game.counter.current, d.app.game.g.comfort, d.app.game.g.score), (3, 72, score), "the level again as it began");
        let (_, lines, _) = d.app.compose();
        assert!(lines.iter().any(|l| l.text == "Deaths: 1"));
    }

    #[test]
    fn the_whole_level_shows_every_sprite_with_a_face() {
        let Ok(data) = harry_data::pack::read(crate::BUNDLED) else { return };
        let loaded = crate::Loaded::new(&data).unwrap();
        let face_bounds = |f: i16| data.faces.get(&f).map_or([0; 4], |x| x.bounds);
        let assets = loaded.assets(&face_bounds);
        let mut game = start_game(&data, &assets, 7, PlayerGlobals::new_game(), 1);
        let near = game.world.sprites.list.len();
        game.whole_level(&assets);
        let w = &game.world;
        let live: Vec<_> = w.sprites.list.iter().map(|&i| w.rec(i)).filter(|r| r.task.is_some()).collect();
        assert!(live.len() > near, "the parked sprites come back ({near} near the start, {} in all)", live.len());
        // of the level's sprites only what the game shows by nothing:
        // invisible message spots (sprites made just now get their face
        // in the next frame)
        let faceless: Vec<i16> = live.iter().filter(|r| r.face == 0 && r.level_index >= 0).map(|r| r.kind).collect();
        assert!(faceless.iter().all(|&k| k == 21000), "{faceless:?}");
        let c = crate::render::draw_level(&data, &game).unwrap();
        assert_eq!((c.width, c.height), (4096, 4096));
    }
}
