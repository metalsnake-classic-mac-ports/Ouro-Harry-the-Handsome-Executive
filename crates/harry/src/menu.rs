//! The main menu (`MainMenu`, `DrawMainMenu`): PICT 130 on the whole
//! screen, six buttons on the right (pressed: PICT 4901–4906, released:
//! 4911–4916) taken with the mouse (`TrackClickOnCommandButton`) or a
//! key: N new game, O / R continue, I instructions, P options, C
//! credits, Q quit; X a message. Instructions (PICT 132) replace the
//! left part of the screen; the credits (`FUN_10055f90`: DLOG 600, PICT
//! 4804 with PICT 4803 scrolling in it) are a window over the menu.
//!
//! Also the boxes the port shows instead of the original's message
//! dialogs (a message, the question before a game is given up).

use harry_core::sound::{Channel, RATE, SoundEvent};
use harry_data::{GameData, Image};

use crate::render::Canvas;
use crate::text::{Face, Fonts, Line};

const BLACK: u8 = 255;
const WHITE: u8 = 0;

/// Button sounds (`_DAT_10065b4c` / `…48`): pressed, released; `snd`
/// 199 after more than 9 presses in one drag; 206 for the X message.
const PRESS: i16 = 4803;
const RELEASE: i16 = 4804;
const FIDGET: i16 = 199;
const MESSAGE_SOUND: i16 = 206;

/// The buttons as `TrackClickOnCommandButton` draws them: x 492 – 603,
/// (top, bottom). `MainMenu` hits a click 1 px smaller to the right and
/// bottom.
const BUTTONS: [(i32, i32); 6] = [(44, 70), (75, 101), (106, 132), (137, 163), (168, 195), (200, 226)];
const LEFT: i32 = 492;
const RIGHT: i32 = 603;

/// The X message (`ReportDialog`).
const X_MESSAGE: &str =
    "But Mr. Horrible says, \u{201c}I don't mind, the thing that bothers me is \u{2014} someone keeps moving my chair.\u{201d}";

/// What the menu asks of the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    NewGame,
    Continue,
    Options,
    Quit,
}

/// The credits window: DLOG 600 (200 × 285, centred on the screen), PICT
/// 4804 at its top left, PICT 4803 (136 wide) moving up through (34, 120)
/// – (170, 270) a row every 3 ticks, from below and over again.
struct Credits {
    step: i32,
    wait: u32,
}

const CREDITS_SIZE: (i32, i32) = (200, 285);
const CREDITS_AT: (i32, i32) = ((640 - CREDITS_SIZE.0) / 2, (480 - CREDITS_SIZE.1) / 2);
const CREDITS_CLIP: (i32, i32, i32, i32) = (34, 120, 170, 270);

pub struct Menu {
    /// The instructions are shown on the left.
    pub instructions: bool,
    /// The button drawn pressed.
    pressed: Option<usize>,
    /// A key took this button: the action after 10 ticks.
    key_press: Option<(usize, u32)>,
    /// The mouse went down on this button; how often it went in.
    tracking: Option<(usize, u32)>,
    credits: Option<Credits>,
    /// A message box over the menu.
    pub message: Option<String>,
    /// Ticks before X shows its message again.
    x_wait: u32,
    /// Sounds to play.
    pub sounds: Vec<SoundEvent>,
}

fn play(id: i16) -> SoundEvent {
    SoundEvent::Play { id, left: 0x55, right: 0x55, rate: RATE, channel: Channel::Effect, handle: 0 }
}

fn button_at(x: i32, y: i32, track: bool) -> Option<usize> {
    let (r, db) = if track { (RIGHT, 0) } else { (RIGHT - 1, 1) };
    BUTTONS.iter().position(|&(t, b)| (LEFT..r).contains(&x) && (t..b - db).contains(&y))
}

impl Default for Menu {
    fn default() -> Self {
        Menu::new()
    }
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            instructions: false,
            pressed: None,
            key_press: None,
            tracking: None,
            credits: None,
            message: None,
            x_wait: 0,
            sounds: Vec::new(),
        }
    }

    pub fn credits_open(&self) -> bool {
        self.credits.is_some()
    }

    /// A key (a letter of the current layout, lower case; Escape and
    /// Return as '\x1b' and '\r') pressed.
    pub fn key(&mut self, c: char) {
        // a box or the credits close on any key (the port; the original's
        // credits close only with a click in them)
        if self.message.take().is_some() || self.credits.take().is_some() {
            return;
        }
        if self.key_press.is_some() || self.tracking.is_some() {
            return;
        }
        let button = match c {
            'n' => 0,
            'o' | 'r' => 1,
            'i' => 2,
            'p' => 3,
            'c' => 4,
            'q' => 5,
            'x' if self.x_wait == 0 => {
                self.x_wait = 30;
                self.sounds.push(play(MESSAGE_SOUND));
                self.message = Some(X_MESSAGE.to_string());
                return;
            }
            _ => return,
        };
        self.pressed = Some(button);
        self.sounds.push(play(PRESS));
        self.key_press = Some((button, 10));
    }

    pub fn mouse_down(&mut self, x: i32, y: i32) {
        if self.message.take().is_some() {
            return;
        }
        if self.credits.is_some() {
            let (l, t) = CREDITS_AT;
            if (l..l + CREDITS_SIZE.0).contains(&x) && (t..t + CREDITS_SIZE.1).contains(&y) {
                self.credits = None;
            }
            return;
        }
        if self.key_press.is_none()
            && let Some(b) = button_at(x, y, false)
        {
            self.tracking = Some((b, 1));
            self.pressed = Some(b);
            self.sounds.push(play(PRESS));
        }
    }

    pub fn mouse_move(&mut self, x: i32, y: i32) {
        let Some((b, n)) = self.tracking else { return };
        let inside = button_at(x, y, true) == Some(b);
        if inside && self.pressed.is_none() {
            self.pressed = Some(b);
            self.sounds.push(play(PRESS));
            self.tracking = Some((b, n + 1));
        } else if !inside && self.pressed.is_some() {
            self.pressed = None;
            self.sounds.push(play(RELEASE));
            if n > 9 {
                self.sounds.push(play(FIDGET));
                self.tracking = Some((b, 0));
            }
        }
    }

    /// The mouse button released: the button's action if it is still on it.
    pub fn mouse_up(&mut self, x: i32, y: i32) -> Option<Action> {
        let (b, _) = self.tracking.take()?;
        self.pressed = None;
        (button_at(x, y, true) == Some(b)).then(|| self.press(b)).flatten()
    }

    fn press(&mut self, button: usize) -> Option<Action> {
        match button {
            0 => Some(Action::NewGame),
            1 => Some(Action::Continue),
            2 => {
                self.instructions = true;
                None
            }
            // the port's options screen (the original: its preferences dialog)
            3 => Some(Action::Options),
            4 => {
                self.credits = Some(Credits { step: 0, wait: 0 });
                None
            }
            _ => Some(Action::Quit),
        }
    }

    /// One tick (1/60 s).
    pub fn tick(&mut self, data: &GameData) -> Option<Action> {
        self.x_wait = self.x_wait.saturating_sub(1);
        if let Some(c) = &mut self.credits {
            if c.wait > 0 {
                c.wait -= 1;
            } else {
                let height = data.pictures.get(&4803).map_or(0, |p| p.height as i32);
                c.step += 1;
                if c.step >= height + 150 {
                    c.step = 0;
                }
                c.wait = 2;
            }
        }
        if let Some((b, t)) = &mut self.key_press {
            *t -= 1;
            if *t == 0 {
                let b = *b;
                self.key_press = None;
                self.pressed = None;
                return self.press(b);
            }
        }
        None
    }

    /// The menu screen and its texts.
    pub fn draw(&self, data: &GameData, fonts: &Fonts) -> (Canvas, Vec<Line>) {
        let mut c = Canvas::new(640, 480, BLACK);
        if let Some(p) = data.pictures.get(&130) {
            c.draw_image(p, (0, 0, p.width, p.height), 0, 0);
        }
        if self.instructions {
            c.fill(16, 16, 448, 448, BLACK);
            if let Some(p) = data.pictures.get(&132) {
                c.draw_image(p, (0, 0, p.width, p.height), 38, 41);
            }
        }
        if let Some(b) = self.pressed
            && let Some(p) = data.pictures.get(&(4901 + b as i16))
        {
            c.draw_image(p, (0, 0, p.width, p.height), LEFT as usize, BUTTONS[b].0 as usize);
        }
        let mut lines = Vec::new();
        if let Some(cr) = &self.credits {
            draw_credits(&mut c, data, cr);
        }
        if let Some(m) = &self.message {
            lines.extend(message_box(&mut c, fonts, m));
        }
        (c, lines)
    }
}

/// A dialog's frame (`dBoxProc`, approximated): a black line, a white one,
/// two black ones around the content.
fn frame(c: &mut Canvas, l: i32, t: i32, w: i32, h: i32) {
    for (k, colour) in [(1, BLACK), (2, BLACK), (3, WHITE), (4, BLACK)] {
        for x in l - k..l + w + k {
            c.set(x, t - k, colour);
            c.set(x, t + h + k - 1, colour);
        }
        for y in t - k..t + h + k {
            c.set(l - k, y, colour);
            c.set(l + w + k - 1, y, colour);
        }
    }
}

fn draw_credits(c: &mut Canvas, data: &GameData, cr: &Credits) {
    let (l, t) = CREDITS_AT;
    let (w, h) = CREDITS_SIZE;
    for y in t..t + h {
        for x in l..l + w {
            c.set(x, y, WHITE);
        }
    }
    frame(c, l, t, w, h);
    if let Some(p) = data.pictures.get(&4804) {
        draw_at(c, p, l, t);
    }
    let Some(p) = data.pictures.get(&4803) else { return };
    let (cl, ct, cr_, cb) = CREDITS_CLIP;
    let top = cb - cr.step;
    for row in 0..p.height as i32 {
        let y = top + row;
        if y < ct || y >= cb {
            continue;
        }
        for x in 0..p.width as i32 {
            if cl + x >= cr_ {
                break;
            }
            if let Some(px) = p.get(x as usize, row as usize) {
                c.set(l + cl + x, t + y, px);
            }
        }
    }
}

fn draw_at(c: &mut Canvas, p: &Image, x: i32, y: i32) {
    for yy in 0..p.height {
        for xx in 0..p.width {
            if let Some(px) = p.get(xx, yy) {
                c.set(x + xx as i32, y + yy as i32, px);
            }
        }
    }
}

/// A message in a box in the middle of the screen (the port's stand-in
/// for the original's message dialogs): white, framed, Geneva 9 within
/// 300 px. Returns the text lines.
pub fn message_box(c: &mut Canvas, fonts: &Fonts, text: &str) -> Vec<Line> {
    const W: i32 = 320;
    const LINE: i32 = 12;
    let mut rows = Vec::new();
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_inclusive(' ') {
            let candidate = format!("{line}{word}");
            if fonts.width(Face::Geneva, 9.0, candidate.trim_end()) > (W - 20) as f32 && !line.is_empty() {
                rows.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        rows.push(line);
    }
    let h = rows.len() as i32 * LINE + 20;
    let (l, t) = ((640 - W) / 2, (480 - h) / 2);
    for y in t..t + h {
        for x in l..l + W {
            c.set(x, y, WHITE);
        }
    }
    frame(c, l, t, W, h);
    rows.into_iter()
        .enumerate()
        .map(|(i, text)| Line {
            x: (l + 10) as f32,
            baseline: (t + 10 + 9 + i as i32 * LINE) as f32,
            size: 9.0,
            face: Face::Geneva,
            color: [0, 0, 0],
            text: text.trim_end().to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> Option<GameData> {
        harry_data::pack::read(crate::BUNDLED).ok()
    }

    fn dump(c: &Canvas, name: &str) {
        if let Ok(d) = std::env::var("HARRY_SCENE_DUMP") {
            let mut rgb = vec![0; c.width * c.height * 3];
            c.to_rgb24(&harry_data::clut8(), &mut rgb);
            crate::render::write_bmp(&std::path::Path::new(&d).join(format!("{name}.bmp")), c.width, c.height, &rgb).unwrap();
        }
    }

    #[test]
    fn a_key_presses_its_button_for_ten_ticks() {
        let Some(data) = pack() else { return };
        let fonts = Fonts::load();
        let mut m = Menu::new();
        m.key('n');
        assert_eq!(m.sounds, [play(PRESS)]);
        let (c, _) = m.draw(&data, &fonts);
        dump(&c, "menu_new_pressed");
        // the pressed button (PICT 4901) is drawn at (492, 44)
        let p = &data.pictures[&4901];
        assert_eq!(Some(c.pixels[(44 + 10) * 640 + 492 + 10]), p.get(10, 10));
        for _ in 0..9 {
            assert_eq!(m.tick(&data), None);
        }
        assert_eq!(m.tick(&data), Some(Action::NewGame));
        assert!(m.pressed.is_none());
    }

    #[test]
    fn a_click_takes_a_button_only_when_released_on_it() {
        let Some(data) = pack() else { return };
        let fonts = Fonts::load();
        let mut m = Menu::new();
        // instructions: pressed, out (released sound), back in, up on it
        m.mouse_down(500, 110);
        m.mouse_move(300, 110);
        m.mouse_move(500, 110);
        assert_eq!(m.mouse_up(500, 110), None);
        assert_eq!(m.sounds, [play(PRESS), play(RELEASE), play(PRESS)]);
        assert!(m.instructions);
        dump(&m.draw(&data, &fonts).0, "menu_instructions");
        // quit, but released beside it: nothing
        m.mouse_down(500, 210);
        assert_eq!(m.mouse_up(480, 210), None);
        m.mouse_down(500, 210);
        assert_eq!(m.mouse_up(600, 225), Some(Action::Quit));
    }

    #[test]
    fn the_credits_scroll_a_row_every_three_ticks_and_close_on_a_click() {
        let Some(data) = pack() else { return };
        let fonts = Fonts::load();
        let mut m = Menu::new();
        m.key('c');
        for _ in 0..10 {
            m.tick(&data);
        }
        assert!(m.credits_open());
        for _ in 0..3 * 200 {
            m.tick(&data);
        }
        assert_eq!(m.credits.as_ref().unwrap().step, 200);
        dump(&m.draw(&data, &fonts).0, "menu_credits");
        // a click beside the window: still open; in it: closed
        m.mouse_down(10, 10);
        assert!(m.credits_open());
        m.mouse_down(CREDITS_AT.0 + 5, CREDITS_AT.1 + 5);
        assert!(!m.credits_open());
    }

    #[test]
    fn x_shows_its_message_and_waits_half_a_second() {
        let Some(data) = pack() else { return };
        let fonts = Fonts::load();
        let mut m = Menu::new();
        m.key('x');
        assert_eq!(m.message.as_deref(), Some(X_MESSAGE));
        let (mut c, _) = m.draw(&data, &fonts);
        let lines = message_box(&mut c, &fonts, X_MESSAGE);
        assert!(lines.len() >= 2);
        dump(&c, "menu_message");
        m.key(' ');
        assert!(m.message.is_none());
        m.key('x');
        assert!(m.message.is_none(), "again only after 30 ticks");
    }
}
