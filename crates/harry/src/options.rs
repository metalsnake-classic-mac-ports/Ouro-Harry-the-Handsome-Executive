//! The settings and the options screen: the port's own screen in place of
//! the original's preferences dialog (DITL 6900–6903). From it: the camera
//! mode, saving between levels, the sound volume and the keys; the
//! original's speed options (reduce frame rate, line skipping, interlace,
//! compatible drawing, system tasks) are left out, the display options
//! (scaling, full screen) are new. Saved as text in SDL's preference
//! folder.

use std::path::Path;

use harry_core::keys::Bindings;
use harry_core::view::mode;
use sdl3::keyboard::Scancode;

use crate::render::Canvas;
use crate::text::{Face, Line};

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
    /// Integer scaling of the picture, else as large as the window allows.
    pub integer_scaling: bool,
    pub fullscreen: bool,
    /// The key that switches full screen (the port's; stored by its SDL
    /// name).
    pub fullscreen_key: Scancode,
    pub bindings: Bindings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            camera: mode::FOLLOW,
            autosave: true,
            volume: 9,
            integer_scaling: true,
            fullscreen: false,
            fullscreen_key: Scancode::F11,
            bindings: Bindings { left: 0x7B, right: 0x7C, scoot: 0x7D, inch: 0x7E, kick: 0x31, select_item: 0x3B, use_item: 0x3A },
        }
    }
}

/// The camera modes as the original's dialog names them.
const CAMERAS: [(i16, &str); 4] =
    [(1, "Constant, Catch-up"), (2, "Constant, Anticipate"), (3, "Step-Scroll, Smoother"), (4, "Step-Scroll, Faster")];

/// The actions to bind, as the original's dialog names them.
const ACTIONS: [&str; 7] = ["Rotate Left", "Rotate Right", "Scoot", "Inch", "Power Kick", "Select Item", "Use Item"];

fn binding(b: &mut Bindings, k: usize) -> &mut i16 {
    match k {
        0 => &mut b.left,
        1 => &mut b.right,
        2 => &mut b.scoot,
        3 => &mut b.inch,
        4 => &mut b.kick,
        5 => &mut b.select_item,
        _ => &mut b.use_item,
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
                "integer_scaling" => s.integer_scaling = n != 0,
                "fullscreen" => s.fullscreen = n == 1,
                "fullscreen_key" => {
                    if let Some(sc) = Scancode::from_name(v.trim()) {
                        s.fullscreen_key = sc;
                    }
                }
                _ => {
                    if let Some(a) = k.strip_prefix("key ").or_else(|| k.strip_prefix("key_"))
                        && let Ok(a) = a.parse::<usize>()
                        && a < ACTIONS.len()
                        && (0..128).contains(&n)
                    {
                        *binding(&mut s.bindings, a) = n as i16;
                    }
                }
            }
        }
        s
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut out = format!(
            "camera {}\nautosave {}\nvolume {}\ninteger_scaling {}\nfullscreen {}\n",
            self.camera,
            u8::from(self.autosave),
            self.volume,
            u8::from(self.integer_scaling),
            u8::from(self.fullscreen)
        );
        out.push_str(&format!("fullscreen_key {}\n", self.fullscreen_key.name()));
        let mut b = self.bindings;
        for (k, name) in ACTIONS.iter().enumerate() {
            out.push_str(&format!("key_{k} {}  # {name}\n", binding(&mut b, k)));
        }
        std::fs::write(path, out)
    }
}

/// The rows of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Camera,
    Autosave,
    Volume,
    Scaling,
    Fullscreen,
    FullscreenKey,
    Key(usize),
    Defaults,
    Done,
}

const ROWS: [Row; 15] = [
    Row::Camera,
    Row::Autosave,
    Row::Volume,
    Row::Scaling,
    Row::Fullscreen,
    Row::FullscreenKey,
    Row::Key(0),
    Row::Key(1),
    Row::Key(2),
    Row::Key(3),
    Row::Key(4),
    Row::Key(5),
    Row::Key(6),
    Row::Defaults,
    Row::Done,
];

/// The first row of the keys.
const FIRST_KEY: usize = 6;

/// Where the rows are: y of each row's top; 22 px high, from x 32 to 448.
fn row_top(i: usize) -> i32 {
    // a gap before the keys and before the buttons
    let gap = if i >= FIRST_KEY { 26 } else { 0 } + if i >= FIRST_KEY + ACTIONS.len() { 18 } else { 0 };
    72 + i as i32 * 22 + gap
}

/// The options screen over the main menu's left part.
pub struct Screen {
    pub settings: Settings,
    selected: usize,
    /// Waiting for the key of this row.
    capture: Option<Row>,
}

/// What happened on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A setting changed (apply it now).
    Changed,
    /// The screen is done (save and close).
    Done,
}

impl Screen {
    pub fn new(settings: Settings) -> Screen {
        Screen { settings, selected: 0, capture: None }
    }

    /// A key pressed (`code`: its Mac key code, if it has one).
    pub fn key(&mut self, sc: Scancode, code: Option<i16>) -> Option<Event> {
        if let Some(row) = self.capture.take() {
            if sc == Scancode::Escape {
                return None;
            }
            let Row::Key(a) = row else {
                // the full screen key: any key the game does not use
                let b = self.settings.bindings;
                let used = [b.left, b.right, b.scoot, b.inch, b.kick, b.select_item, b.use_item];
                if code.is_some_and(|c| used.contains(&c)) {
                    return None;
                }
                self.settings.fullscreen_key = sc;
                return Some(Event::Changed);
            };
            let code = code?;
            // a key bound elsewhere swaps with this one
            let b = &mut self.settings.bindings;
            let old = *binding(b, a);
            for k in 0..ACTIONS.len() {
                if k != a && *binding(b, k) == code {
                    *binding(b, k) = old;
                }
            }
            *binding(b, a) = code;
            return Some(Event::Changed);
        }
        match sc {
            Scancode::Up => self.selected = (self.selected + ROWS.len() - 1) % ROWS.len(),
            Scancode::Down => self.selected = (self.selected + 1) % ROWS.len(),
            Scancode::Left => return self.change(-1),
            Scancode::Right => return self.change(1),
            Scancode::Return | Scancode::KpEnter | Scancode::Space => return self.activate(),
            Scancode::Escape => return Some(Event::Done),
            _ => {}
        }
        None
    }

    /// A click at (x, y): selects the row and does what Return does.
    pub fn click(&mut self, x: i32, y: i32) -> Option<Event> {
        if self.capture.is_some() || !(32..448).contains(&x) {
            return None;
        }
        let i = (0..ROWS.len()).find(|&i| (row_top(i)..row_top(i) + 22).contains(&y))?;
        self.selected = i;
        self.activate()
    }

    fn activate(&mut self) -> Option<Event> {
        match ROWS[self.selected] {
            row @ (Row::Key(_) | Row::FullscreenKey) => {
                self.capture = Some(row);
                None
            }
            Row::Defaults => {
                self.settings = Settings::default();
                Some(Event::Changed)
            }
            Row::Done => Some(Event::Done),
            _ => self.change(1),
        }
    }

    fn change(&mut self, by: i32) -> Option<Event> {
        let s = &mut self.settings;
        match ROWS[self.selected] {
            Row::Camera => {
                let i = CAMERAS.iter().position(|c| c.0 == s.camera).unwrap_or(0) as i32;
                s.camera = CAMERAS[(i + by).rem_euclid(CAMERAS.len() as i32) as usize].0;
            }
            Row::Autosave => s.autosave = !s.autosave,
            Row::Volume => s.volume = (i32::from(s.volume) + by).clamp(0, 9) as u8,
            Row::Scaling => s.integer_scaling = !s.integer_scaling,
            Row::Fullscreen => s.fullscreen = !s.fullscreen,
            _ => return None,
        }
        Some(Event::Changed)
    }

    /// Draw over the menu's left part (the game area); the texts.
    pub fn draw(&self, c: &mut Canvas, key_name: &dyn Fn(i16) -> String) -> Vec<Line> {
        c.fill(16, 16, 448, 448, 255);
        // the selected row on dark red, as the inventory marks its slot
        let red = harry_data::clut8().iter().position(|p| *p == [0x88, 0, 0]).unwrap_or(255) as u8;
        let t = row_top(self.selected);
        c.fill(32, t as usize, 416, 22, red);
        let white = [255; 3];
        let grey = [0xbb; 3];
        let label = |x: f32, y: i32, text: &str, face: Face, color: [u8; 3]| Line {
            x,
            baseline: (y + 15) as f32,
            size: 12.0,
            face,
            color,
            text: text.to_string(),
        };
        let mut lines = vec![
            Line { x: 40.0, baseline: 50.0, size: 18.0, face: Face::HelveticaBold, color: white, text: "Preferences".into() },
            label(40.0, row_top(FIRST_KEY) - 24, "Keys", Face::HelveticaBold, white),
        ];
        let s = &self.settings;
        let mut b = s.bindings;
        for (i, row) in ROWS.iter().enumerate() {
            let y = row_top(i);
            let colour = if i == self.selected { white } else { grey };
            let (name, value) = match *row {
                Row::Camera => ("Camera Mode", CAMERAS.iter().find(|c| c.0 == s.camera).map_or("?", |c| c.1).to_string()),
                Row::Autosave => ("Save between levels", on_off(s.autosave)),
                Row::Volume => ("Sound Volume", if s.volume == 0 { "Off".into() } else { s.volume.to_string() }),
                Row::Scaling => ("Scaling", if s.integer_scaling { "Whole multiples (sharp)".into() } else { "Fill the window".into() }),
                Row::Fullscreen => ("Full Screen", on_off(s.fullscreen)),
                Row::FullscreenKey => (
                    "Full Screen Key",
                    if self.capture == Some(*row) { "Press a key… (Escape: keep)".into() } else { s.fullscreen_key.name().to_string() },
                ),
                Row::Key(a) => (
                    ACTIONS[a],
                    if self.capture == Some(*row) { "Press a key… (Escape: keep)".into() } else { key_name(*binding(&mut b, a)) },
                ),
                Row::Defaults => ("Default Settings", String::new()),
                Row::Done => ("Done", String::new()),
            };
            lines.push(label(48.0, y, name, Face::HelveticaBold, colour));
            if !value.is_empty() {
                let shown = if i == self.selected && !matches!(row, Row::Key(_) | Row::FullscreenKey) { format!("<  {value}  >") } else { value };
                lines.push(label(230.0, y, &shown, Face::Helvetica, colour));
            }
        }
        lines.push(Line {
            x: 40.0,
            baseline: 452.0,
            size: 10.0,
            face: Face::Helvetica,
            color: grey,
            text: "Up / Down: choose     Left / Right: change     Return: set a key     Escape: done".into(),
        });
        lines
    }
}

fn on_off(b: bool) -> String {
    if b { "On".into() } else { "Off".into() }
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
            integer_scaling: false,
            fullscreen: true,
            fullscreen_key: Scancode::F10,
            ..Default::default()
        };
        s.bindings.kick = 0x24;
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn a_key_bound_twice_swaps() {
        let mut sc = Screen::new(Settings::default());
        // the kick row: Return, then the Up arrow (the inch key)
        sc.selected = FIRST_KEY + 4;
        assert_eq!(sc.key(Scancode::Return, None), None);
        assert_eq!(sc.key(Scancode::Up, Some(0x7E)), Some(Event::Changed));
        assert_eq!((sc.settings.bindings.kick, sc.settings.bindings.inch), (0x7E, 0x31));
        // Escape while waiting keeps the key
        sc.key(Scancode::Return, None);
        assert_eq!(sc.key(Scancode::Escape, None), None);
        assert_eq!(sc.settings.bindings.kick, 0x7E);
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
        assert_eq!(sc.click(100, row_top(ROWS.len() - 1) + 5), Some(Event::Done));
    }

    #[test]
    fn the_full_screen_key_takes_any_key_the_game_does_not_use() {
        let mut sc = Screen::new(Settings::default());
        sc.selected = FIRST_KEY - 1;
        sc.key(Scancode::Return, None);
        // Space kicks: refused
        assert_eq!(sc.key(Scancode::Space, Some(0x31)), None);
        assert_eq!(sc.settings.fullscreen_key, Scancode::F11);
        sc.key(Scancode::Return, None);
        assert_eq!(sc.key(Scancode::F, Some(0x03)), Some(Event::Changed));
        assert_eq!(sc.settings.fullscreen_key, Scancode::F);
    }
}

#[cfg(test)]
mod picture {
    use super::*;

    /// With HARRY_SCENE_DUMP=<dir>: the screen over the menu with its texts.
    #[test]
    fn dump_the_screen() {
        let Ok(dir) = std::env::var("HARRY_SCENE_DUMP") else { return };
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re/work/harry-data.zip");
        let Ok(bytes) = std::fs::read(root) else { return };
        let data = harry_data::pack::read(&bytes).unwrap();
        let fonts = crate::text::Fonts::load();
        let menu = crate::menu::Menu::new();
        let (mut c, _) = menu.draw(&data, &fonts);
        let mut sc = Screen::new(Settings::default());
        sc.key(Scancode::Down, None);
        sc.key(Scancode::Down, None);
        let lines = sc.draw(&mut c, &|code| crate::key_name(code));
        let mut rgb = vec![0; 640 * 480 * 3];
        c.to_rgb24(&harry_data::clut8(), &mut rgb);
        let overlay = fonts.draw(&lines, 640, 480);
        for (p, o) in rgb.as_chunks_mut::<3>().0.iter_mut().zip(overlay.as_chunks::<4>().0) {
            for k in 0..3 {
                p[k] = ((u32::from(p[k]) * (255 - u32::from(o[3])) + u32::from(o[k]) * u32::from(o[3])) / 255) as u8;
            }
        }
        crate::render::write_bmp(&std::path::Path::new(&dir).join("options.bmp"), 640, 480, &rgb).unwrap();
    }
}
