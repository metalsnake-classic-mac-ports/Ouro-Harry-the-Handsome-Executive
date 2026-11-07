//! The game as the window runs it: the main menu, the sequences between
//! levels, the game with its pause, map, balloons, options and the
//! question before it is given up. `App::step` takes one pass of the main
//! loop's input (the events, the keys held, the time); `App::compose`
//! makes the picture. Nothing here touches the window, so the tests drive
//! it with made-up events and time.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use harry_core::frame::PlayerGlobals;
use harry_core::game::{Assets, Balloon, FrameEnd, Game, UpLevel};
use harry_core::keys::KeyMap;
use harry_core::savegame::SaveGame;
use harry_data::GameData;
use sdl3::event::Event;
use sdl3::keyboard::{Keycode, Mod, Scancode};
use sdl3::sys::keycode::SDL_KMOD_NONE;

use crate::{FRAME, ScoreShown, TICK, audio, balloon, key_name, mac_code, mac_seconds, menu, options, picture, render, scenes, text};

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
    /// The main menu, perhaps with the options over it.
    Menu { menu: menu::Menu, options: Option<options::Screen> },
    /// The game; it waits while something is over it.
    Game(Overlay),
}

/// What can be over the game (one at a time; a new one replaces the one
/// before).
pub enum Overlay {
    None,
    /// The options (Command-I).
    Options(options::Screen),
    /// The question before the game is given up (Escape).
    Confirm,
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
        };
        app.screen = if direct {
            let scenes = scenes::Scenes::new(app.start_screen(), true, scenes::game_start(&app.game.level, true, &data.world));
            Screen::Scene { scenes, then_menu: None }
        } else {
            let m = menu::Menu::new();
            let mut scenes = scenes::Scenes::new(app.start_screen(), true, Vec::new());
            // main: SetMusic(600), the menu
            scenes.then = Some((app.menu_screen(&m), vec![scenes::Step::Music(600), scenes::Step::FadeIn(8)]));
            Screen::Scene { scenes, then_menu: Some(m) }
        };
        app
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
        let m = menu::Menu::new();
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
        let mut give_up = false;
        for e in events {
            let in_game = matches!(self.screen, Screen::Game(_));
            match e {
                Event::Quit { .. } => return false,
                Event::Window { win_event: sdl3::event::WindowEvent::FocusLost, .. } => self.focus_paused = true,
                // a key or a click ends the pause of a lost focus (and does
                // nothing else)
                Event::KeyDown { .. } | Event::MouseButtonDown { .. } if self.focus_paused && in_game => self.focus_paused = false,
                Event::MouseButtonDown { x, y, .. } => {
                    pressed = true;
                    if let Some(o) = self.options_mut() {
                        opt_event = opt_event.or(o.click(x as i32, y as i32));
                    } else if let Screen::Menu { menu, .. } = &mut self.screen {
                        menu.mouse_down(x as i32, y as i32);
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    if let Screen::Menu { menu, .. } = &mut self.screen {
                        menu.mouse_move(x as i32, y as i32);
                    }
                }
                Event::MouseButtonUp { x, y, .. } => {
                    if let Screen::Menu { menu, .. } = &mut self.screen {
                        action = action.or(menu.mouse_up(x as i32, y as i32));
                    }
                }
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } if matches!(self.screen, Screen::Game(Overlay::Confirm)) => {
                    match sc {
                        Scancode::Return | Scancode::KpEnter | Scancode::Y => {
                            self.screen = Screen::Game(Overlay::None);
                            give_up = true;
                        }
                        Scancode::Escape | Scancode::N => {
                            self.screen = Screen::Game(Overlay::None);
                            self.next = now;
                        }
                        _ => {}
                    }
                }
                Event::KeyDown { scancode: Some(sc), repeat: false, .. } if self.options().is_some() => {
                    if let Some(o) = self.options_mut() {
                        opt_event = opt_event.or(o.key(sc, mac_code(sc)));
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
                        m.key(c);
                    } else if matches!(sc, Scancode::Return | Scancode::KpEnter) {
                        m.key('\r');
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
                        // CheckGameLoopKeys: Escape asks before the game is given up
                        Scancode::Escape if in_game => self.screen = Screen::Game(Overlay::Confirm),
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
            self.level = n;
            self.game = start_game(data, self.assets, self.level, PlayerGlobals::new_game(), self.game.rng.seed);
            self.show_game_start(false, now);
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
                    Some(menu) => Screen::Menu { menu, options: None },
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
                self.level = 0;
                self.game = start_game(data, self.assets, self.level, PlayerGlobals::new_game(), self.game.rng.seed);
                self.show_game_start(true, now);
            }
            // ContinueGame: the save list is to come; the game saved in
            // this session goes on (a copier save without the story)
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
            Some(menu::Action::Quit) => return false,
            None => {}
        }
        if give_up {
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
            FrameEnd::Running => {}
            FrameEnd::LevelDone => {
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
                    // after Win the main menu
                    (UpLevel::Won, _) => self.back_to_menu(screen, scenes::win(&data.world), now),
                }
            }
            FrameEnd::Died => {
                // GameLoop's end: EndLevelEffect when comfort or favor is gone
                let screen = render::draw(data, &self.game);
                let mut steps = Vec::new();
                if self.game.g.comfort < 1 || self.game.g.favor < 1 {
                    steps.push(scenes::Step::EndLevel);
                }
                // then the main menu (AskToContinue is to come with the save
                // list; the menu's Continue goes on from the game saved in
                // this session)
                self.back_to_menu(screen, steps, now);
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
                let lines = if sc.texts { picture(data, &self.game, &self.score, None).1 } else { Vec::new() };
                (sc.image.clone(), lines, sc.dark)
            }
            Screen::Menu { menu, options } => {
                let (mut c, mut l) = menu.draw(data, fonts);
                if let Some(o) = options {
                    l = o.draw(&mut c, &key_name);
                }
                (c, l, dark)
            }
            Screen::Game(overlay) => {
                let shown = match overlay {
                    Overlay::Balloon(b, _) => Some(b),
                    _ => None,
                };
                let (mut c, mut l) = picture(data, &self.game, &self.score, shown);
                if let Overlay::Options(o) = overlay {
                    // over the game area; the panel's texts stay
                    l.extend(o.draw(&mut c, &key_name));
                } else if self.paused
                    && let Some(p) = data.pictures.get(&4921)
                {
                    self.draw_pause(&mut c, &mut l, p);
                }
                match overlay {
                    Overlay::Map(_) => render::draw_map(&mut c, data, &self.game),
                    Overlay::Confirm => {
                        l.extend(menu::message_box(&mut c, fonts, "Are you sure you want to abort this game?\n\nReturn: yes      Escape: no"))
                    }
                    _ => {}
                }
                (c, l, dark)
            }
        }
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
            Screen::Menu { options, .. } => if options.is_some() { "menu+options" } else { "menu" }.into(),
            Screen::Game(o) => {
                let paused = if self.paused { "+paused" } else { "" };
                match o {
                    Overlay::None => format!("game{paused}"),
                    Overlay::Options(_) => format!("game+options{paused}"),
                    Overlay::Confirm => format!("game+confirm{paused}"),
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

        // Escape asks; No goes on
        d.press(Scancode::Escape, Mod::NOMOD);
        assert_eq!(d.app.describe(), "game+confirm");
        let f = frames(&d);
        d.run(Duration::from_millis(500));
        assert_eq!(frames(&d), f, "the game waits for the answer");
        d.press(Scancode::N, Mod::NOMOD);
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

        // Escape, Return: the game is given up, back to the menu
        d.press(Scancode::Escape, Mod::NOMOD);
        d.press(Scancode::Return, Mod::NOMOD);
        assert_eq!(d.app.describe(), "scene");
        d.run(Duration::from_secs(3));
        assert_eq!(d.app.describe(), "menu");
        // Escape in the menu quits
        assert!(!d.press(Scancode::Escape, Mod::NOMOD));
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
