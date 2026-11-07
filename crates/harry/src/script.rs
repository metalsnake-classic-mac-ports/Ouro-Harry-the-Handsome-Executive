//! `HARRY_SCRIPT`: input played into the running window, for GUI tests
//! without the system's accessibility access. Whitespace-separated steps
//! `<seconds>:<what>`, the seconds counted from the start:
//!
//! - `key=<name>` a key pressed (SDL's scancode name, e.g. `N`, `Escape`,
//!   `Return`, `F8`, `Left_Shift` - `_` for a space; `+cmd` after the
//!   name adds Command / Control);
//! - `hold=<name>` / `release` the game's held keys;
//! - `caps=on` / `caps=off` caps lock;
//! - `focus=lost` the window loses the focus;
//! - `shot=<name>` the picture (with its texts) to `HARRY_SHOTS/<name>.bmp`;
//! - `quit`.
//!
//! Example: `HARRY_SCRIPT="3:key=N 5:key=Space 30:shot=game 31:quit"`.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{Mod, Scancode};

use crate::app::Input;

pub enum Action {
    Key(Scancode, Mod),
    Hold(Scancode),
    Release,
    Caps(bool),
    FocusLost,
    Shot(String),
    Quit,
}

pub struct Script {
    start: Instant,
    steps: Vec<(Duration, Action)>,
}

fn scancode(name: &str) -> Result<Scancode> {
    Scancode::from_name(&name.replace('_', " ")).ok_or_else(|| anyhow!("HARRY_SCRIPT: no key {name}"))
}

impl Script {
    pub fn parse(text: &str, start: Instant) -> Result<Script> {
        let mut steps = Vec::new();
        for item in text.split_whitespace() {
            let (t, what) = item.split_once(':').with_context(|| format!("HARRY_SCRIPT: {item}: no time"))?;
            let t: f64 = t.parse().with_context(|| format!("HARRY_SCRIPT: {item}: bad time"))?;
            let (verb, arg) = what.split_once('=').unwrap_or((what, ""));
            let action = match verb {
                "key" => {
                    let (name, cmd) = arg.strip_suffix("+cmd").map_or((arg, false), |n| (n, true));
                    let keymod = if !cmd {
                        Mod::NOMOD
                    } else if cfg!(target_os = "macos") {
                        Mod::LGUIMOD
                    } else {
                        Mod::LCTRLMOD
                    };
                    Action::Key(scancode(name)?, keymod)
                }
                "hold" => Action::Hold(scancode(arg)?),
                "release" => Action::Release,
                "caps" => Action::Caps(arg == "on"),
                "focus" => Action::FocusLost,
                "shot" => Action::Shot(arg.to_string()),
                "quit" => Action::Quit,
                _ => return Err(anyhow!("HARRY_SCRIPT: {item}: unknown step")),
            };
            steps.push((Duration::from_secs_f64(t), action));
        }
        steps.sort_by_key(|s| s.0);
        steps.reverse();
        Ok(Script { start, steps })
    }

    /// The steps due by `now`, in order.
    pub fn due(&mut self, now: Instant) -> Vec<Action> {
        let mut out = Vec::new();
        while self.steps.last().is_some_and(|(t, _)| self.start + *t <= now) {
            out.push(self.steps.pop().unwrap().1);
        }
        out
    }
}

/// The key-down event a script key makes.
pub fn key_event(sc: Scancode, keymod: Mod) -> Event {
    Event::KeyDown { timestamp: 0, window_id: 0, keycode: None, scancode: Some(sc), keymod, repeat: false, which: 0, raw: 0 }
}

/// The script's state of the held keys and caps lock, applied over the
/// real ones.
#[derive(Default)]
pub struct Held {
    keys: Vec<i16>,
    caps: Option<bool>,
}

impl Held {
    pub fn apply(&mut self, a: &Action) {
        match a {
            Action::Hold(sc) => self.keys.extend(crate::mac_code(*sc)),
            Action::Release => self.keys.clear(),
            Action::Caps(on) => self.caps = Some(*on),
            _ => {}
        }
    }

    pub fn over(&self, mut input: Input) -> Input {
        for &k in &self.keys {
            input.held.press(k);
        }
        if let Some(c) = self.caps {
            input.caps_lock = c;
        }
        input
    }
}

/// The focus-lost event.
pub fn focus_lost() -> Event {
    Event::Window { timestamp: 0, window_id: 0, win_event: WindowEvent::FocusLost }
}
