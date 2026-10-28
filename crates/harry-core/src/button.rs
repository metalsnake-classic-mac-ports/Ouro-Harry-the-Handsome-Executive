//! Buttons (9501–9531 on walls, 9541 a floor plate) and the sliding doors
//! (951, 961) they open. A button keeps its state in its level entry: info
//! #1 is 1 while it is up, 0 while it is down. A sliding door's info #0
//! names the entry it follows (negative: the opposite of that entry).

use crate::level::Level;
use crate::sprite::{HitTask, Sprite, SpriteEvent, Task};
use crate::tiles::{KICK_RECTS, offset, sect};

/// `snd` ids.
pub mod snd {
    pub const SLIDING_CLOSE: i16 = 4709;
    pub const SLIDING_OPEN: i16 = 4708;
    pub const BUTTON_UP: i16 = 4707;
    pub const BUTTON_DOWN: i16 = 4706;
    pub const PLATE_UP: i16 = 4705;
    pub const PLATE_DOWN: i16 = 4704;
}

/// The floor plate; the wall buttons are below it.
pub const PLATE: i16 = 9541;

/// Faces: four per wall button (up … down, cicn 9501–9534), five for the
/// plate (PICT 9541).
fn faces(kind: i16) -> Option<i16> {
    match kind {
        9501 => Some(1701),
        9511 => Some(1711),
        9521 => Some(1721),
        9531 => Some(1731),
        PLATE => Some(1741),
        _ => None,
    }
}

/// The level entry at `index` - the original reads it without a range
/// check where the comments say so; the port gives none then.
fn entry(level: &Level, index: i16) -> Option<&crate::level::LevelSprite> {
    usize::try_from(index).ok().and_then(|i| level.sprites.get(i))
}

/// `SetupButtonSprite`: layer 1, up (dir 0), not yet started (+0x8b
/// clear). Hot rects: 9501 / 9511 (−8, 0, 8, 24), 9521 / 9531 (0, −8, 24,
/// 8), the plate (0, 0, 18, 18).
pub fn setup_button(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Button);
    sp.hit_task = Some(HitTask::Button);
    sp.hit_tile_task = None;
    sp.layer = 1;
    match sp.kind {
        9501 | 9511 => sp.hot_rect = [-8, 0, 8, 24],
        9521 | 9531 => sp.hot_rect = [0, -8, 24, 8],
        PLATE => sp.hot_rect = [0, 0, 18, 18],
        _ => {}
    }
    sp.at_from_pos();
    sp.b8b = false;
}

/// `HandleButtonSprite`. On its first frame a button whose entry has info
/// #1 set starts down (dir 9; the entry is read without a range check).
/// `dir` is how far it is pressed (0–9): it springs back one step a frame
/// unless it is fully down (8 or more) and its entry's info #0 is 1 (it
/// stays down). Coming up past 5 plays its release sound (the plate its
/// own). Its face shows dir / 2 (the wall buttons' last one from 8 on),
/// and its entry's info #1 is 1 below 7, else 0. `f6e` keeps the last dir.
pub fn handle_button(sp: &mut Sprite, level: &mut Level, events: &mut Vec<SpriteEvent>) {
    if !sp.b8b {
        sp.b8b = true;
        if entry(level, sp.level_index).is_some_and(|e| e.info[1] != 0) {
            sp.dir = 9;
        }
    }
    if sp.frozen {
        return;
    }
    if sp.dir < 8 || entry(level, sp.level_index).map_or(0, |e| e.info[0]) != 1 {
        sp.dir -= 1;
    }
    sp.dir = sp.dir.clamp(0, 9);
    if i32::from(sp.dir) < sp.f6e && sp.f6e == 5 {
        events.push(SpriteEvent::Sound3D(if sp.kind < PLATE { snd::BUTTON_UP } else { snd::PLATE_UP }));
    }
    if let Some(first) = faces(sp.kind) {
        sp.face = if sp.kind == PLATE {
            first + sp.dir / 2
        } else {
            first + if sp.dir < 8 { sp.dir >> 1 } else { 3 }
        };
    }
    if let Some(e) = usize::try_from(sp.level_index).ok().filter(|&i| i < 256).and_then(|i| level.sprites.get_mut(i)) {
        e.info[1] = i16::from(sp.dir < 7);
    }
    sp.f6e = i32::from(sp.dir);
}

/// `HitButtonSprite`. Harry walking (not kicking) onto it, a plant,
/// barrel or chair (robots, reversors and mail guys too): pressed by 2 a
/// frame (sound when it starts down). A shot pushes a wall button fully
/// down (dir 9). Harry's kick reaching a wall button: down by 10.
pub fn hit_button(sp: &mut Sprite, other: &Sprite, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || other.frozen {
        return;
    }
    let player = other.task == Some(Task::Player);
    let down = if sp.kind < PLATE { snd::BUTTON_DOWN } else { snd::PLATE_DOWN };
    if (player && other.hot_rect[1] != 0) || matches!(other.task, Some(Task::Plant | Task::Barrel | Task::Chair)) {
        if sp.dir == 0 {
            events.push(SpriteEvent::Sound3D(down));
        }
        sp.dir += 2;
    } else if matches!(other.task, Some(Task::EnemyShot | Task::PlayerShot)) {
        if sp.kind < PLATE {
            events.push(SpriteEvent::Sound3D(down));
            sp.dir = 9;
        }
    } else if player && sp.kind < PLATE {
        let kick = offset(KICK_RECTS[other.dir as usize], other.at.0, other.at.1);
        if sect(kick, offset(sp.hot_rect, sp.at.0, sp.at.1)) {
            events.push(SpriteEvent::Sound3D(down));
            sp.dir += 10;
        }
    }
}

/// The sliding doors' faces: 951 (across, PICT 851) and 961 (upright,
/// PICT 861), eight frames each.
const SLIDING_FACES: [i16; 2] = [4401, 4451];

/// `SetupSlidingDoorSprite`: layer 5, closed. 951 lies across (hot rect
/// (0, 0, 6, 32)), 961 stands upright ((0, 0, 32, 6)).
pub fn setup_sliding_door(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::SlidingDoor);
    sp.hit_task = Some(HitTask::SlidingDoor);
    sp.hit_tile_task = None;
    sp.layer = 5;
    match sp.kind {
        961 => sp.hot_rect = [0, 0, 32, 6],
        951 => sp.hot_rect = [0, 0, 6, 32],
        _ => {}
    }
    sp.at_from_pos();
    sp.b8b = true;
}

/// `HandleSlidingDoorSprite` (only with a level entry): it follows the
/// entry its info #0 names - info #1 of that entry 0 (the button down):
/// it opens a step a frame up to 14 (sound as it starts), else it closes
/// (sound as it starts back); a negative info #0 follows the opposite of
/// entry −info #0. (Other indexes than 0–255 read past the list; the port
/// takes info #1 = 0 there.) Face dir / 2; from 8 on it lets everything
/// through (no hot rect).
pub fn handle_sliding_door(sp: &mut Sprite, level: &Level, events: &mut Vec<SpriteEvent>) {
    if sp.frozen || !(0..256).contains(&sp.level_index) {
        return;
    }
    let link = entry(level, sp.level_index).map_or(0, |e| e.info[0]);
    let state = |i: i16| entry(level, i).map_or(0, |e| e.info[1]);
    let closed = if link < 0 { i16::from(state(link.wrapping_neg()) == 0) } else { state(link) };
    if closed == 0 {
        if sp.dir < 14 {
            if sp.dir == 0 {
                events.push(SpriteEvent::Sound3D(snd::SLIDING_OPEN));
            }
            sp.dir += 1;
        }
    } else if sp.dir > 0 {
        if sp.dir == 14 {
            events.push(SpriteEvent::Sound3D(snd::SLIDING_CLOSE));
        }
        sp.dir -= 1;
    }
    let (first, rect) = match sp.kind {
        961 => (SLIDING_FACES[1], [0, 0, 32, 6]),
        951 => (SLIDING_FACES[0], [0, 0, 6, 32]),
        _ => return,
    };
    sp.face = first + (sp.dir >> 1);
    sp.hot_rect = if sp.dir < 8 { rect } else { [0; 4] };
}
