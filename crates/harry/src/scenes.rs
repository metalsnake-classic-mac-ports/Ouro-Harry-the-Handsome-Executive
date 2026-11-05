//! The sequences between the game's frames, as the original runs them in
//! ticks of 1/60 s while the game loop waits: gamma fades, the end of a
//! level (`EndLevelEffect`), scrolled story pictures (`ScrollPlot`), a
//! level's title card (`IntroLevel`) and the end of the world (`Win`).
//! They draw on a copy of the screen that stays between steps, as the
//! original draws on the window.

use std::collections::VecDeque;

use harry_data::{GameData, Image};

use crate::render::Canvas;

/// The game area (`DAT_1006748c`): (16, 16) – (464, 464).
const AREA: (usize, usize, usize, usize) = (16, 16, 448, 448);
const BLACK: u8 = 255;

/// One step of a sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// `GammaFadeOut(n)` / `GammaFadeIn(n)`: to black in steps of (256 / n)
    /// / 2 per tick, back in steps of 256 / n; nothing when already there.
    FadeOut(i16),
    FadeIn(i16),
    /// `EndLevelEffect`: black squares grow over the game area.
    EndLevel,
    /// `ScrollPlot(pict)`: the picture scrolls up through the game area.
    Scroll(i16),
    /// The game area black (`PaintRect` in black).
    Black,
    /// The whole screen black.
    BlackScreen,
    /// A level's title card (`IntroLevel`'s picture): centred across the
    /// game area, 16 px too high (the original adds the area's offset only
    /// horizontally).
    Title(i16),
    /// A picture at its own frame (`FUN_1005dacc` mode 0), on the whole
    /// screen; the panel's texts are gone.
    Whole(i16),
    /// `Delay` or a wait: this many ticks.
    Wait(u32),
    /// `SetMusic(id)`: the tune from its start (silent while faded out).
    Music(i16),
    /// `FadeMusic(in, ticks)`: the music fades out (or in) in 24 steps of
    /// `ticks`, and the sequence waits for it; nothing when it is already
    /// out (in).
    FadeMusic { out: bool, ticks: u16 },
}

/// What a sequence asks of the music player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicCommand {
    Play(i16),
    Fade { out: bool, ticks: u16 },
}

/// What a running step remembers.
enum Running {
    Fade { out: bool, level: i32, step: i32 },
    EndLevel(Box<[[i16; 14]; 14]>),
    Scroll { pict: i16, top: i32, n: i32, cooldown: i16, wait: u32, asked: bool },
    Wait(u32),
}

pub struct Scenes {
    /// The screen being drawn on.
    pub image: Canvas,
    /// How dark the screen is: 0 (as drawn) … 256 (black).
    pub dark: i32,
    /// The panel's texts are shown (not after a whole-screen picture).
    pub texts: bool,
    steps: VecDeque<Step>,
    running: Option<Running>,
    /// What follows on another screen (a new game's start after a death or
    /// the win): its screen and steps.
    pub then: Option<(Canvas, Vec<Step>)>,
    /// The music is faded out (`DAT_10072711`); kept by the caller.
    pub music_faded: bool,
    /// Music commands for the caller to pass on.
    pub music: Vec<MusicCommand>,
}

impl Scenes {
    /// A sequence over `image`; `faded` if the screen is black already.
    pub fn new(image: Canvas, faded: bool, steps: Vec<Step>) -> Scenes {
        Scenes {
            image,
            dark: if faded { 256 } else { 0 },
            texts: true,
            steps: steps.into(),
            running: None,
            then: None,
            music_faded: false,
            music: Vec::new(),
        }
    }

    pub fn done(&self) -> bool {
        self.running.is_none() && self.steps.is_empty() && self.then.is_none()
    }

    /// One tick (1/60 s). `pressed`: a key or the mouse button was pressed
    /// since the last tick. A press during a scrolled picture ends it once
    /// 8 of its steps are shown (the original ends it on any key held down
    /// then, also one held since before; NOTES.md, "Port improvements").
    pub fn tick(&mut self, data: &GameData, pressed: bool) {
        loop {
            if self.running.is_none() {
                if self.steps.is_empty()
                    && let Some((image, steps)) = self.then.take()
                {
                    self.image = image;
                    self.texts = true;
                    self.steps = steps.into();
                }
                let Some(step) = self.steps.pop_front() else { return };
                if !self.begin(step, data) {
                    continue;
                }
            }
            break;
        }
        let finished = match self.running.as_mut() {
            Some(Running::Fade { out, level, step }) => {
                // a tick per level 0, step, … up to 256; then at once the end
                *level += *step;
                let end = *level + *step > 256;
                let l = if end { 256 } else { *level };
                self.dark = if *out { l } else { 256 - l };
                end
            }
            Some(Running::EndLevel(cells)) => {
                for c in cells.iter_mut().flatten() {
                    if *c < 17 {
                        *c += 1;
                    }
                }
                for (row, line) in cells.iter().enumerate() {
                    for (col, &c) in line.iter().enumerate() {
                        if 0 < c && c < 17 {
                            let (h, v, c) = (col as i32 * 32 + 32, row as i32 * 32 + 32, i32::from(c));
                            frame_rect(&mut self.image, h - c, v - c, h + c, v + c);
                        }
                    }
                }
                cells.iter().flatten().all(|&c| c == 17)
            }
            Some(Running::Scroll { pict, top, n, cooldown, wait, asked }) => {
                *asked |= pressed;
                if *wait > 0 {
                    *wait -= 1;
                    false
                } else {
                    let Some(p) = data.world_pictures.get(pict) else { return };
                    *top += 1;
                    scroll_copy(&mut self.image, p, *top);
                    if *cooldown > 0 {
                        *cooldown -= 1;
                    }
                    *n += 1;
                    *wait = 3;
                    let end = *n > p.height as i32 + 448 || (*asked && *cooldown == 0);
                    if end {
                        self.image.fill(AREA.0, AREA.1, AREA.2, AREA.3, BLACK);
                    }
                    end
                }
            }
            Some(Running::Wait(t)) => {
                *t = t.saturating_sub(1);
                *t == 0
            }
            None => false,
        };
        if finished {
            self.running = None;
        }
    }

    /// Start a step; false if it is done at once.
    fn begin(&mut self, step: Step, data: &GameData) -> bool {
        self.running = match step {
            Step::FadeOut(n) | Step::FadeIn(n) => {
                let out = matches!(step, Step::FadeOut(_));
                // the fades do nothing when the screen is already there
                if out == (self.dark >= 256) {
                    return false;
                }
                let step = if out { (256 / i32::from(n)) >> 1 } else { 256 / i32::from(n) };
                Some(Running::Fade { out, level: -step, step: step.max(1) })
            }
            Step::EndLevel => {
                // the top half row by row from the top left, the bottom
                // half from the bottom right, one square a tick
                let mut cells = [[0i16; 14]; 14];
                for (row, line) in cells.iter_mut().enumerate().take(7) {
                    for (col, c) in line.iter_mut().enumerate() {
                        *c = -(row as i16 * 14) - col as i16;
                    }
                }
                let mut n = 0;
                for row in (7..14).rev() {
                    for col in (0..14).rev() {
                        cells[row][col] = n;
                        n -= 1;
                    }
                }
                Some(Running::EndLevel(Box::new(cells)))
            }
            Step::Scroll(pict) => {
                if !data.world_pictures.contains_key(&pict) {
                    return false;
                }
                Some(Running::Scroll { pict, top: -448, n: 0, cooldown: 8, wait: 0, asked: false })
            }
            Step::Black => {
                self.image.fill(AREA.0, AREA.1, AREA.2, AREA.3, BLACK);
                return false;
            }
            Step::BlackScreen => {
                let (w, h) = (self.image.width, self.image.height);
                self.image.fill(0, 0, w, h, BLACK);
                return false;
            }
            Step::Title(pict) => {
                if let Some(p) = data.world_pictures.get(&pict) {
                    let x = ((448 - p.width as i32) >> 1) + 16;
                    let y = (448 - p.height as i32) >> 1;
                    draw_at(&mut self.image, p, x, y);
                }
                return false;
            }
            Step::Whole(pict) => {
                if let Some(p) = data.world_pictures.get(&pict) {
                    draw_at(&mut self.image, p, 0, 0);
                    self.texts = false;
                }
                return false;
            }
            Step::Wait(t) => {
                if t == 0 {
                    return false;
                }
                Some(Running::Wait(t))
            }
            Step::Music(id) => {
                self.music.push(MusicCommand::Play(id));
                return false;
            }
            Step::FadeMusic { out, ticks } => {
                if out == self.music_faded {
                    return false;
                }
                self.music_faded = out;
                self.music.push(MusicCommand::Fade { out, ticks });
                Some(Running::Wait(24 * u32::from(ticks.max(1))))
            }
        };
        true
    }
}

/// `FrameRect` in black: the rectangle's 1-px border (right and bottom
/// exclusive).
fn frame_rect(c: &mut Canvas, l: i32, t: i32, r: i32, b: i32) {
    for x in l..r {
        c.set(x, t, BLACK);
        c.set(x, b - 1, BLACK);
    }
    for y in t..b {
        c.set(l, y, BLACK);
        c.set(r - 1, y, BLACK);
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

/// One step of `ScrollPlot`: rows `top` … `top` + 448 of the picture (400
/// wide) to (40, 16) – (440, 464); rows outside the picture are not drawn,
/// so the screen keeps what it had there.
fn scroll_copy(c: &mut Canvas, p: &Image, top: i32) {
    let (from, to) = (top.max(0), (top + 448).min(p.height as i32));
    for row in from..to {
        let y = 16 + row - top;
        for x in 0..400.min(p.width) {
            if let Some(px) = p.get(x, row as usize) {
                c.set(40 + x as i32, y, px);
            }
        }
    }
}

/// The steps of `IntroLevel` for a level: fade out, the game area black,
/// the title card, fade in, a second's wait.
pub fn intro_level(title: i16, fade_music: bool) -> Vec<Step> {
    let mut s = vec![Step::FadeOut(8), Step::Black];
    if title != 0 {
        s.push(Step::Title(title));
    }
    s.push(Step::FadeIn(8));
    // for any level but the world's first the music fades out here
    if fade_music {
        s.push(Step::FadeMusic { out: true, ticks: 2 });
    }
    s.push(Step::Wait(60));
    s
}

/// `ScrollPlot(pict, music)`: with music (the world's scroll music, if it
/// has one) the music changes to it around the picture.
fn scroll_plot(pict: i16, music: i16) -> Vec<Step> {
    if music == 0 {
        return vec![Step::Scroll(pict)];
    }
    vec![
        Step::FadeMusic { out: true, ticks: 2 },
        Step::Music(music),
        Step::FadeMusic { out: false, ticks: 2 },
        Step::Scroll(pict),
        Step::FadeMusic { out: true, ticks: 2 },
    ]
}

/// The level's music fading in, as `GameLoop` and `UpLevel` start it.
fn level_music(level: &harry_core::Level) -> [Step; 2] {
    [Step::Music(level.music), Step::FadeMusic { out: false, ticks: 2 }]
}

/// `GameLoop`'s start: the music fades out; with `intro` and a story
/// picture before the level (info #6), it scrolls between fades; then
/// `IntroLevel` and the level's music.
pub fn game_start(level: &harry_core::Level, intro: bool, world: &harry_data::WorldInfo) -> Vec<Step> {
    let mut s = vec![Step::FadeMusic { out: true, ticks: 4 }];
    if intro && level.info[6] != 0 {
        s.push(Step::FadeIn(8));
        s.extend(scroll_plot(level.info[6], world.scroll_music));
        s.push(Step::FadeOut(8));
    }
    s.extend(intro_level(level.info[8], level.number != world.first_level));
    s.extend(level_music(level));
    s
}

/// `UpLevel` to the next level: the old level's story picture after it
/// (info #5), the game area black, the new level's `IntroLevel`, then its
/// story picture before it (info #6) - after the title card here, unlike
/// at `GameLoop`'s start - and its music.
pub fn up_level(old: &harry_core::Level, new: &harry_core::Level, world: &harry_data::WorldInfo) -> Vec<Step> {
    let mut s = vec![Step::EndLevel];
    if old.info[5] != 0 {
        s.extend(scroll_plot(old.info[5], world.scroll_music));
    }
    s.push(Step::Black);
    s.extend(intro_level(new.info[8], new.number != world.first_level));
    if new.info[6] != 0 {
        s.push(Step::Black);
        s.extend(scroll_plot(new.info[6], world.scroll_music));
    }
    s.extend(level_music(new));
    s
}

/// `UpLevel` past the last level: `EndLevelEffect`, then `Win`: the win
/// music, the world's picture scrolled (without its scroll music), a long
/// fade out, its win picture on the whole screen for its seconds, the
/// music and the screen fade out.
pub fn win(world: &harry_data::WorldInfo) -> Vec<Step> {
    let mut s = vec![Step::EndLevel];
    if world.win_music != 0 {
        s.extend([
            Step::FadeMusic { out: true, ticks: 6 },
            Step::Music(world.win_music),
            Step::FadeMusic { out: false, ticks: 6 },
        ]);
    }
    if world.win_scroll != 0 {
        s.push(Step::Scroll(world.win_scroll));
    }
    s.push(Step::FadeOut(75));
    if world.win_show != 0 {
        s.extend([
            Step::BlackScreen,
            Step::Whole(world.win_show),
            Step::FadeIn(75),
            Step::Wait(world.win_seconds.max(0) as u32 * 60),
            Step::FadeMusic { out: true, ticks: 6 },
            Step::FadeOut(75),
        ]);
    }
    s
}

/// The main menu after a game (`NewGame`'s end): the menu's music fades in.
pub fn menu_music() -> [Step; 2] {
    [Step::Music(600), Step::FadeMusic { out: false, ticks: 2 }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> Option<GameData> {
        harry_data::pack::read(crate::BUNDLED).ok()
    }

    /// Run to the end; the ticks it took. With HARRY_SCENE_DUMP=<dir> a
    /// picture every `every` ticks.
    /// `press_at`: a key pressed at that tick.
    fn run(data: &GameData, sc: &mut Scenes, press_at: Option<u32>, name: &str, every: u32) -> u32 {
        let dump = std::env::var("HARRY_SCENE_DUMP").ok();
        let mut t = 0;
        while !sc.done() {
            sc.tick(data, press_at == Some(t));
            t += 1;
            if let Some(d) = &dump
                && t % every == 0
            {
                let mut rgb = vec![0; sc.image.width * sc.image.height * 3];
                sc.image.to_rgb24(&harry_data::clut8(), &mut rgb);
                let path = std::path::Path::new(d).join(format!("{name}_{t:05}.bmp"));
                crate::render::write_bmp(&path, sc.image.width, sc.image.height, &rgb).unwrap();
            }
            assert!(t < 100_000, "{name} does not end");
        }
        t
    }

    fn screen() -> Canvas {
        Canvas::new(640, 480, 0)
    }

    #[test]
    fn the_end_of_a_level_blackens_the_area_in_114_ticks() {
        let Some(data) = pack() else { return };
        let mut sc = Scenes::new(screen(), false, vec![Step::EndLevel]);
        assert_eq!(run(&data, &mut sc, None, "endlevel", 10), 114);
        for y in 16..464 {
            for x in 16..464 {
                assert_eq!(sc.image.pixels[y * 640 + x], BLACK, "({x}, {y})");
            }
        }
        assert_eq!(sc.image.pixels[15 * 640 + 15], 0);
    }

    #[test]
    fn the_story_scrolls_four_ticks_a_row_and_a_key_ends_it() {
        let Some(data) = pack() else { return };
        // level 1: the story (PICT 200, 993 rows) between fades, the title
        let steps = game_start(&data.levels[0], true, &data.world);
        assert_eq!(steps[1..3], [Step::FadeIn(8), Step::FadeMusic { out: true, ticks: 2 }]);
        assert!(steps.contains(&Step::Music(620)) && steps.contains(&Step::Scroll(200)));
        assert_eq!(steps[steps.len() - 2..], [Step::Music(601), Step::FadeMusic { out: false, ticks: 2 }]);
        let mut sc = Scenes::new(screen(), true, vec![Step::Scroll(200)]);
        assert_eq!(run(&data, &mut sc, None, "scroll", 400), (993 + 449) * 4 - 3);
        // a key pressed at once: it ends with the 8th row
        let mut sc = Scenes::new(screen(), true, vec![Step::Scroll(200)]);
        assert_eq!(run(&data, &mut sc, Some(0), "scroll_skip", 1000), 7 * 4 + 1);
        assert_eq!(sc.image.pixels[100 * 640 + 100], BLACK);
        // later: with the next row
        let mut sc = Scenes::new(screen(), true, vec![Step::Scroll(200)]);
        assert_eq!(run(&data, &mut sc, Some(101), "scroll_skip", 1000), 26 * 4 + 1);
        // a press before the picture (during the squares) does not count
        let mut sc = Scenes::new(screen(), true, vec![Step::EndLevel, Step::Scroll(200)]);
        assert_eq!(run(&data, &mut sc, Some(50), "scroll_skip", 1000), 114 + (993 + 449) * 4 - 3);
    }

    #[test]
    fn a_title_card_fades_in_and_waits_a_second() {
        let Some(data) = pack() else { return };
        let mut sc = Scenes::new(screen(), false, intro_level(301, false));
        // fade out 16 + 1 ticks, fade in 8 + 1, wait 60
        assert_eq!(run(&data, &mut sc, None, "title", 5), 17 + 9 + 60);
        assert_eq!(sc.dark, 0);
        // PICT 301 (307 × 49) at ((448 − 307) / 2 + 16, (448 − 49) / 2)
        let p = &data.world_pictures[&301];
        let (x, y) = (86, 199);
        for yy in 0..p.height {
            for xx in 0..p.width {
                assert_eq!(Some(sc.image.pixels[(y + yy) * 640 + x + xx]), p.get(xx, yy));
            }
        }
    }

    #[test]
    fn the_win_shows_its_picture_for_twelve_seconds() {
        let Some(data) = pack() else { return };
        let w = &data.world;
        assert_eq!((w.win_scroll, w.win_show, w.win_seconds), (209, 400, 12));
        let mut sc = Scenes::new(screen(), false, win(w));
        let t = run(&data, &mut sc, None, "win", 600);
        // the squares, the music's fades (24 · 6 ticks each, the first a no-op
        // while the music plays), the scroll, three screen fades of 75 (257,
        // 86, 257 ticks), 12 s
        assert_eq!(t, 114 + 144 + 144 + (530 + 449) * 4 - 3 + 257 + 86 + 720 + 144 + 257);
        assert!(!sc.texts);
        assert_eq!(sc.dark, 256);
    }
}
