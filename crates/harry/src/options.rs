//! The settings and the options screen: the Ouro ports' options and keys
//! pages (ouro-ui, rule 9) in place of the original's preferences dialog
//! (DITL 6900–6903). From it: the camera mode, saving between levels, the
//! sound and music volumes and the keys; the original's speed options
//! (reduce frame rate, line skipping, interlace, compatible drawing,
//! system tasks) are left out, the display options (scaling, full screen)
//! are new. Saved as text in SDL's preference folder.

use std::path::Path;

use harry_core::keys::Bindings;
use harry_core::view::mode;
use sdl3::keyboard::Scancode;

use crate::render::Canvas;
use crate::text::Line;

/// What the player chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// prefs +8: 1 constant, catch-up; 2 constant, anticipate; 3
    /// step-scroll, smoother; 4 step-scroll, faster.
    pub camera: i16,
    /// prefs +1: save at each level's start.
    pub autosave: bool,
    /// The sound effects' volume, 0 (off) – 9 (prefs +0xc).
    pub volume: u8,
    /// The music's volume, 0 (off) – 9 (prefs +0xe).
    pub music_volume: u8,
    /// How the picture is scaled to the window.
    pub scaling: Scaling,
    pub fullscreen: bool,
    /// The key that switches full screen (the port's; stored by its SDL
    /// name).
    pub fullscreen_key: Scancode,
    /// The key that pauses (the port's; the original: caps lock, paused
    /// while it is locked). Another key pauses and goes on by turns.
    pub pause_key: Scancode,
    pub bindings: Bindings,
    /// prefs +0x1e: the key that shows the map (Tab).
    pub map_key: i16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            camera: mode::FOLLOW,
            autosave: true,
            volume: 9,
            music_volume: 7,
            scaling: Scaling::Whole,
            fullscreen: false,
            fullscreen_key: Scancode::F11,
            pause_key: Scancode::CapsLock,
            bindings: Bindings { left: 0x7B, right: 0x7C, scoot: 0x7D, inch: 0x7E, kick: 0x31, select_item: 0x3B, use_item: 0x3A },
            map_key: 0x30,
        }
    }
}

/// How the picture is scaled to the window or the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scaling {
    /// By the largest whole multiple that fits (sharp pixels).
    Whole,
    /// As large as fits, keeping the shape.
    Fill,
    /// Not at all: one pixel of the original is one point of the screen,
    /// in the middle (cut off if the window is smaller).
    Original,
}

impl Scaling {
    const ALL: [Scaling; 3] = [Scaling::Whole, Scaling::Fill, Scaling::Original];

    pub fn name(self) -> &'static str {
        match self {
            Scaling::Whole => "Whole multiples (sharp)",
            Scaling::Fill => "Fill the window",
            Scaling::Original => "Original size",
        }
    }

    /// `by` steps on, round.
    fn step(self, by: i32) -> Scaling {
        let i = Scaling::ALL.iter().position(|&s| s == self).unwrap_or(0) as i32;
        Scaling::ALL[(i + by).rem_euclid(Scaling::ALL.len() as i32) as usize]
    }

    /// In the settings file: 1 whole, 0 fill (as `integer_scaling` before),
    /// 2 original.
    fn code(self) -> u8 {
        match self {
            Scaling::Fill => 0,
            Scaling::Whole => 1,
            Scaling::Original => 2,
        }
    }
}

/// The camera modes as the original's dialog names them.
const CAMERAS: [(i16, &str); 4] =
    [(1, "Constant, Catch-up"), (2, "Constant, Anticipate"), (3, "Step-Scroll, Smoother"), (4, "Step-Scroll, Faster")];

/// The actions to bind, as the original's dialog names them.
const ACTIONS: [&str; 8] = ["Rotate Left", "Rotate Right", "Scoot", "Inch", "Power Kick", "Select Item", "Use Item", "Toggle Map"];

fn binding(s: &mut Settings, k: usize) -> &mut i16 {
    let b = &mut s.bindings;
    match k {
        0 => &mut b.left,
        1 => &mut b.right,
        2 => &mut b.scoot,
        3 => &mut b.inch,
        4 => &mut b.kick,
        5 => &mut b.select_item,
        6 => &mut b.use_item,
        _ => &mut s.map_key,
    }
}

impl Settings {
    pub fn load(path: &Path) -> Settings {
        let mut s = Settings::default();
        let Ok(text) = std::fs::read_to_string(path) else { return s };
        for line in text.lines() {
            let Some((k, v)) = line.split_once(' ') else { continue };
            // the value, then maybe a comment
            let n: i32 = v.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(-1);
            match k {
                "camera" if (1..=4).contains(&n) => s.camera = n as i16,
                "autosave" => s.autosave = n != 0,
                "volume" if (0..=9).contains(&n) => s.volume = n as u8,
                "music_volume" if (0..=9).contains(&n) => s.music_volume = n as u8,
                // `integer_scaling` before 0.2.1: 1 whole, 0 fill
                "scaling" | "integer_scaling" => {
                    if let Some(&sc) = Scaling::ALL.iter().find(|sc| i32::from(sc.code()) == n) {
                        s.scaling = sc;
                    }
                }
                "fullscreen" => s.fullscreen = n == 1,
                "fullscreen_key" => {
                    if let Some(sc) = Scancode::from_name(v.trim()) {
                        s.fullscreen_key = sc;
                    }
                }
                "pause_key" => {
                    if let Some(sc) = Scancode::from_name(v.trim()) {
                        s.pause_key = sc;
                    }
                }
                _ => {
                    if let Some(a) = k.strip_prefix("key ").or_else(|| k.strip_prefix("key_"))
                        && let Ok(a) = a.parse::<usize>()
                        && a < ACTIONS.len()
                        && (0..128).contains(&n)
                    {
                        *binding(&mut s, a) = n as i16;
                    }
                }
            }
        }
        s
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut out = format!(
            "camera {}\nautosave {}\nvolume {}\nmusic_volume {}\nscaling {}  # 1 whole multiples, 0 fill, 2 original size\nfullscreen {}\n",
            self.camera,
            u8::from(self.autosave),
            self.volume,
            self.music_volume,
            self.scaling.code(),
            u8::from(self.fullscreen)
        );
        out.push_str(&format!("fullscreen_key {}\n", self.fullscreen_key.name()));
        out.push_str(&format!("pause_key {}\n", self.pause_key.name()));
        let mut b = self.clone();
        for (k, name) in ACTIONS.iter().enumerate() {
            out.push_str(&format!("key_{k} {}  # {name}\n", binding(&mut b, k)));
        }
        std::fs::write(path, out)
    }
}

/// The options page's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Opt {
    Camera,
    Autosave,
    Volume,
    MusicVolume,
    Scaling,
    Fullscreen,
    Keys,
    Defaults,
}

const OPTIONS: [Opt; 8] = [Opt::Camera, Opt::Autosave, Opt::Volume, Opt::MusicVolume, Opt::Scaling, Opt::Fullscreen, Opt::Keys, Opt::Defaults];

/// The keys page's rows: the game's actions, then the port's keys.
const FULLSCREEN_ROW: usize = ACTIONS.len();
const PAUSE_ROW: usize = FULLSCREEN_ROW + 1;
const KEY_ROWS: usize = PAUSE_ROW + 1;

/// What each key row does (the keys page's line under the list).
const KEY_ABOUT: [&str; KEY_ROWS] = [
    "Turn left",
    "Turn right",
    "Move backwards",
    "Move forwards",
    "Kick with the chair",
    "Choose the next item",
    "Use the chosen item",
    "Show the map of the level",
    "Switch between window and full screen",
    "Pause (caps lock: while locked, another key: by turns)",
];
const KEY_NOTES: [&str; 1] = ["1-4 pick a weapon; Backspace shows the timeline; Escape leaves the game (it is kept)"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Options,
    Keys,
}

/// The options page (ouro-ui's, in Harry's look) and its keys page.
pub struct Screen {
    pub settings: Settings,
    page: Page,
    options: ouro_ui::options::Options,
    keys: ouro_ui::keys::Keys,
}

/// What happened on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A setting changed (apply it now).
    Changed,
    /// The screen is done (save and close).
    Done,
}

/// The pages cover the screen.
const SCREEN: ouro_ui::Rect = ouro_ui::Rect::new(0, 0, 640, 480);

impl Screen {
    pub fn new(settings: Settings) -> Screen {
        Screen { settings, page: Page::Options, options: Default::default(), keys: Default::default() }
    }

    fn rows(&self) -> Vec<ouro_ui::options::Row> {
        use ouro_ui::options::{Row, on_off};
        let s = &self.settings;
        let volume = |v: u8| if v == 0 { "Off".to_string() } else { v.to_string() };
        OPTIONS
            .iter()
            .map(|o| match o {
                Opt::Camera => Row::setting("Camera Mode", CAMERAS.iter().find(|c| c.0 == s.camera).map_or("?", |c| c.1), "How the view follows Harry"),
                Opt::Autosave => Row::setting("Save between levels", on_off(s.autosave), "The original's save at each level's start"),
                Opt::Volume => Row::setting("Sound Volume", volume(s.volume), "The sound effects, 0 (off) to 9"),
                Opt::MusicVolume => Row::setting("Music Volume", volume(s.music_volume), "The music, 0 (off) to 9"),
                Opt::Scaling => Row::setting(
                    "Scaling",
                    s.scaling.name(),
                    "The picture by whole multiples, as large as the window allows, or not scaled",
                ),
                Opt::Fullscreen => Row::setting("Full Screen", on_off(s.fullscreen), "The whole screen or a window"),
                Opt::Keys => Row::action("Keys...", "The game's keys, full screen and pause"),
                Opt::Defaults => Row::action("Defaults", "Camera, saving, volumes and display as at first (the keys stay)"),
            })
            .collect()
    }

    fn bindings(&self, key_name: &dyn Fn(i16) -> String) -> Vec<ouro_ui::keys::Binding> {
        let mut b = self.settings.clone();
        (0..KEY_ROWS)
            .map(|k| {
                let (action, key) = match k {
                    FULLSCREEN_ROW => ("Full Screen", b.fullscreen_key.name().to_string()),
                    PAUSE_ROW => ("Pause", b.pause_key.name().to_string()),
                    a => (ACTIONS[a], key_name(*binding(&mut b, a))),
                };
                ouro_ui::keys::Binding { action: action.into(), key, pad: String::new(), about: KEY_ABOUT[k].into() }
            })
            .collect()
    }

    /// A key pressed (`code`: its Mac key code, if it has one).
    pub fn key(&mut self, sc: Scancode, code: Option<i16>) -> Option<Event> {
        if self.page == Page::Keys && self.keys.waiting {
            self.keys.done();
            return if sc == Scancode::Escape { None } else { self.set_key(self.keys.selected, sc, code) };
        }
        let key = crate::theme::ui_key(sc)?;
        match self.page {
            Page::Options => {
                let choice = self.options.key(key, &self.rows())?;
                self.options_choice(choice)
            }
            Page::Keys => {
                let choice = self.keys.key(key, KEY_ROWS)?;
                self.keys_choice(choice)
            }
        }
    }

    /// The mouse at (x, y); `click`: its button went down.
    pub fn mouse(&mut self, x: i32, y: i32, click: bool) -> Option<Event> {
        match self.page {
            Page::Options => {
                let choice = self.options.mouse(x, y, click, &self.rows(), 0, SCREEN)?;
                self.options_choice(choice)
            }
            Page::Keys => {
                let choice = self.keys.mouse(x, y, click, KEY_ROWS, KEY_NOTES.len(), SCREEN)?;
                self.keys_choice(choice)
            }
        }
    }

    fn options_choice(&mut self, choice: ouro_ui::options::Choice) -> Option<Event> {
        use ouro_ui::options::Choice;
        let (row, by) = match choice {
            Choice::Back => return Some(Event::Done),
            Choice::Change(k, by) => (OPTIONS[k], by),
            Choice::Take(k) => (OPTIONS[k], 1),
        };
        let s = &mut self.settings;
        match row {
            Opt::Camera => {
                let i = CAMERAS.iter().position(|c| c.0 == s.camera).unwrap_or(0) as i32;
                s.camera = CAMERAS[(i + by).rem_euclid(CAMERAS.len() as i32) as usize].0;
            }
            Opt::Autosave => s.autosave = !s.autosave,
            // 9 and on goes round to off, as a click steps on
            Opt::Volume => s.volume = (i32::from(s.volume) + by).rem_euclid(10) as u8,
            Opt::MusicVolume => s.music_volume = (i32::from(s.music_volume) + by).rem_euclid(10) as u8,
            Opt::Scaling => s.scaling = s.scaling.step(by),
            Opt::Fullscreen => s.fullscreen = !s.fullscreen,
            Opt::Keys => {
                self.page = Page::Keys;
                self.keys = Default::default();
                return None;
            }
            Opt::Defaults => {
                let d = Settings::default();
                *s = Settings { bindings: s.bindings, map_key: s.map_key, fullscreen_key: s.fullscreen_key, pause_key: s.pause_key, ..d };
            }
        }
        Some(Event::Changed)
    }

    fn keys_choice(&mut self, choice: ouro_ui::keys::Choice) -> Option<Event> {
        match choice {
            // the page waits for the key now
            ouro_ui::keys::Choice::Change(_) => None,
            ouro_ui::keys::Choice::Defaults => {
                let d = Settings::default();
                let s = &mut self.settings;
                (s.bindings, s.map_key, s.fullscreen_key, s.pause_key) = (d.bindings, d.map_key, d.fullscreen_key, d.pause_key);
                Some(Event::Changed)
            }
            ouro_ui::keys::Choice::Back => {
                self.page = Page::Options;
                self.options.selected = OPTIONS.iter().position(|&o| o == Opt::Keys).unwrap_or(0);
                None
            }
        }
    }

    /// The key of row `row`: a game key swaps with one that has it; the
    /// full screen and pause keys take any key the game and the other one
    /// do not use. Backspace (0x33, the timeline) is no game key.
    fn set_key(&mut self, row: usize, sc: Scancode, code: Option<i16>) -> Option<Event> {
        let s = &mut self.settings;
        if row >= FULLSCREEN_ROW {
            let b = s.bindings;
            let used = [b.left, b.right, b.scoot, b.inch, b.kick, b.select_item, b.use_item, s.map_key];
            let other = if row == PAUSE_ROW { s.fullscreen_key } else { s.pause_key };
            if code.is_some_and(|c| used.contains(&c)) || sc == other || sc == Scancode::Backspace {
                return None;
            }
            if row == PAUSE_ROW {
                s.pause_key = sc;
            } else {
                s.fullscreen_key = sc;
            }
            return Some(Event::Changed);
        }
        let code = code.filter(|&c| c != 0x33 && Some(sc) != Some(s.fullscreen_key) && sc != s.pause_key)?;
        let old = *binding(s, row);
        for k in 0..ACTIONS.len() {
            if k != row && *binding(s, k) == code {
                *binding(s, k) = old;
            }
        }
        *binding(s, row) = code;
        Some(Event::Changed)
    }

    /// The page over the whole screen; its texts.
    pub fn draw(&self, c: &mut Canvas, look: &crate::theme::Look, key_name: &dyn Fn(i16) -> String) -> Vec<Line> {
        match self.page {
            Page::Options => self.options.draw(look, c, SCREEN, "Preferences", &self.rows(), &[]),
            Page::Keys => self.keys.draw(look, c, SCREEN, &self.bindings(key_name), &KEY_NOTES),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_their_file() {
        let dir = std::env::temp_dir().join(format!("ouro-harry-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.txt");
        let mut s = Settings {
            camera: 3,
            autosave: false,
            volume: 4,
            scaling: Scaling::Original,
            fullscreen: true,
            fullscreen_key: Scancode::F10,
            pause_key: Scancode::Pause,
            ..Default::default()
        };
        s.bindings.kick = 0x24;
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    /// To the keys page and onto row `row`.
    fn keys_page(row: usize) -> Screen {
        let mut sc = Screen::new(Settings::default());
        sc.options.selected = OPTIONS.iter().position(|&o| o == Opt::Keys).unwrap();
        assert_eq!(sc.key(Scancode::Return, None), None);
        assert_eq!(sc.page, Page::Keys);
        sc.keys.selected = row;
        sc
    }

    #[test]
    fn a_key_bound_twice_swaps() {
        // the kick row: Return, then the Up arrow (the inch key)
        let mut sc = keys_page(4);
        assert_eq!(sc.key(Scancode::Return, None), None);
        assert_eq!(sc.key(Scancode::Up, Some(0x7E)), Some(Event::Changed));
        assert_eq!((sc.settings.bindings.kick, sc.settings.bindings.inch), (0x7E, 0x31));
        // Escape while waiting keeps the key; Backspace is the timeline's
        sc.key(Scancode::Return, None);
        assert_eq!(sc.key(Scancode::Escape, None), None);
        sc.key(Scancode::Return, None);
        assert_eq!(sc.key(Scancode::Backspace, Some(0x33)), None);
        assert_eq!(sc.settings.bindings.kick, 0x7E);
        // Escape on the page: back to the options
        sc.key(Scancode::Escape, None);
        assert_eq!(sc.page, Page::Options);
        assert_eq!(sc.key(Scancode::Escape, None), Some(Event::Done));
    }

    #[test]
    fn left_and_right_change_the_values() {
        let mut sc = Screen::new(Settings::default());
        sc.key(Scancode::Right, None);
        assert_eq!(sc.settings.camera, 2);
        sc.key(Scancode::Left, None);
        sc.key(Scancode::Left, None);
        assert_eq!(sc.settings.camera, 4);
        sc.key(Scancode::Down, None);
        sc.key(Scancode::Down, None);
        sc.key(Scancode::Left, None);
        assert_eq!(sc.settings.volume, 8);
        sc.key(Scancode::Down, None);
        sc.key(Scancode::Down, None);
        assert_eq!(sc.key(Scancode::Space, None), Some(Event::Changed));
        assert_eq!(sc.settings.scaling, Scaling::Fill);
        sc.key(Scancode::Right, None);
        assert_eq!(sc.settings.scaling, Scaling::Original);
        sc.key(Scancode::Right, None);
        assert_eq!(sc.settings.scaling, Scaling::Whole);
        sc.key(Scancode::Left, None);
        assert_eq!(sc.settings.scaling, Scaling::Original);
    }

    #[test]
    fn the_full_screen_key_takes_any_key_the_game_does_not_use() {
        let mut sc = keys_page(FULLSCREEN_ROW);
        sc.key(Scancode::Return, None);
        // Space kicks: refused
        assert_eq!(sc.key(Scancode::Space, Some(0x31)), None);
        assert_eq!(sc.settings.fullscreen_key, Scancode::F11);
        sc.key(Scancode::Return, None);
        assert_eq!(sc.key(Scancode::F, Some(0x03)), Some(Event::Changed));
        assert_eq!(sc.settings.fullscreen_key, Scancode::F);
    }

    #[test]
    fn the_pause_key_takes_any_key_the_game_and_full_screen_do_not_use() {
        let mut sc = keys_page(PAUSE_ROW);
        sc.key(Scancode::Return, None);
        // F11 is the full screen key, ← turns: refused
        assert_eq!(sc.key(Scancode::F11, None), None);
        sc.key(Scancode::Return, None);
        assert_eq!(sc.key(Scancode::Left, Some(0x7B)), None);
        assert_eq!(sc.settings.pause_key, Scancode::CapsLock);
        sc.key(Scancode::Return, None);
        assert_eq!(sc.key(Scancode::P, Some(0x23)), Some(Event::Changed));
        assert_eq!(sc.settings.pause_key, Scancode::P);
        // and the full screen key cannot take it then
        sc.keys.selected = FULLSCREEN_ROW;
        sc.key(Scancode::Return, None);
        assert_eq!(sc.key(Scancode::P, Some(0x23)), None);
    }
}

#[cfg(test)]
mod picture {
    use super::*;

    /// With HARRY_SCENE_DUMP=<dir>: the options and keys pages with their texts.
    #[test]
    fn dump_the_screen() {
        let Ok(dir) = std::env::var("HARRY_SCENE_DUMP") else { return };
        let fonts = crate::text::Fonts::load();
        let look = crate::theme::Look { fonts: &fonts };
        let mut sc = Screen::new(Settings::default());
        sc.key(Scancode::Down, None);
        sc.key(Scancode::Down, None);
        for (name, keys) in [("options", false), ("keys", true)] {
            if keys {
                sc.options.selected = OPTIONS.iter().position(|&o| o == Opt::Keys).unwrap();
                sc.key(Scancode::Return, None);
            }
            let mut c = Canvas::new(640, 480, 255);
            let lines = sc.draw(&mut c, &look, &|code| crate::key_name(code));
            crate::write_picture(&std::path::Path::new(&dir).join(format!("{name}.bmp")), &c, &lines, 0, &fonts).unwrap();
        }
    }
}
