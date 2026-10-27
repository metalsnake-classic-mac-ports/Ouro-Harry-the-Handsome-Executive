//! Simple sprite kinds: bonuses, items, exits, doors. Each has a setup (run when
//! the sprite is made), a task (each frame) and a hit task (SAT's
//! collision pass); what Harry does when he touches them is in
//! `HitPlayerSprite`.

use crate::level::Level;
use crate::sprite::{HitTask, Sprite, Task};


/// `SetupBonusSprite` (700–799: the bonus objects Harry collects).
pub fn setup_bonus(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Bonus);
    sp.hit_task = Some(HitTask::Bonus);
    sp.hit_tile_task = None;
    sp.layer = 1;
    sp.hot_rect = [-6, -6, 22, 22];
    match sp.kind {
        700 | 701 => sp.face = 1501,
        702 => {
            sp.face = 1503;
            sp.hot_rect = [-6, -6, 36, 36];
        }
        703 => sp.face = 1504,
        711..=713 => {
            sp.hot_rect = [-10, -10, 26, 26];
            sp.face = 1505 + (sp.kind - 711);
        }
        _ => {}
    }
    sp.at_from_pos();
    sp.fix_from_at();
    sp.b8b = true;
}

/// `HandleBonusSprite`: 700 and 701 blink between two faces.
pub fn handle_bonus(sp: &mut Sprite) {
    if sp.frozen || !matches!(sp.kind, 700 | 701) {
        return;
    }
    if sp.dir == 0 {
        sp.dir = 1;
        sp.face = 1502;
    } else {
        sp.dir = 0;
        sp.face = 1501;
    }
}

/// `SetupItemSprite` (11001–19999: items to pick up; the object id is the
/// inventory id).
pub fn setup_item(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Item);
    sp.hit_task = Some(HitTask::Item);
    sp.hit_tile_task = None;
    sp.hot_rect = [-6, -6, 20, 20];
    sp.face = match sp.kind {
        11699 => 0,
        11398 => 3651,
        11397 => 3652,
        // faces of 11199, 11299, … 12099 (none for 11699 and 12099); ids
        // outside read past the original's table
        k => match (k - 11199) / 100 {
            i @ (0..=4 | 6..=8) => 3601 + i,
            _ => 0,
        },
    };
    sp.layer = 7;
    sp.f6e = 0;
    sp.at_from_pos();
    sp.b8b = true;
}

/// `HandleItemSprite`: the mystery vial (11699) bubbles through 8 faces,
/// two frames each.
pub fn handle_item(sp: &mut Sprite) {
    if sp.frozen || sp.kind != 11699 {
        return;
    }
    sp.dir += 1;
    if sp.dir > 15 {
        sp.dir = 0;
    }
    sp.face = 3661 + sp.dir / 2;
}

/// `SetupExitSprite` (9990–9999). 9991–9994 are the four directions,
/// each with its face and a hot rect beside it.
pub fn setup_exit(sp: &mut Sprite) {
    sp.init();
    sp.timer = 0;
    sp.dir = 0;
    sp.task = Some(Task::Exit);
    sp.hit_task = Some(HitTask::Exit);
    sp.hit_tile_task = None;
    (sp.face, sp.hot_rect) = match sp.kind {
        9991 => (3201, [96, 0, 100, 64]),
        9992 => (3202, [-40, 0, -32, 64]),
        9993 => (3203, [0, 40, 64, 100]),
        9994 => (3204, [0, -32, 64, 0]),
        _ => (0, [0, 0, 32, 32]),
    };
    sp.layer = 1;
    sp.at_from_pos();
    sp.b8b = false;
}

/// The level counter and the one before (`*_DAT_100659cc`,
/// `_DAT_10068568`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LevelCounter {
    pub current: i16,
    pub previous: i16,
}

/// `HitExitSprite`: Harry reaches the exit - the next level is the exit's
/// info #0, or the following one when that is 0. Works once (timer 69,
/// never counted down).
pub fn hit_exit(sp: &mut Sprite, other: &Sprite, level: &Level, counter: &mut LevelCounter) {
    if sp.frozen || other.frozen || other.task != Some(Task::Player) || sp.timer != 0 {
        return;
    }
    counter.previous = counter.current;
    let dest = level.sprites.get(sp.level_index as usize).map_or(0, |s| s.info[0]);
    counter.current = if dest == 0 { counter.current.wrapping_add(1) } else { dest };
    sp.timer = 69;
}

/// Door faces: 36 frames from HrSp 2601, nine per door kind (opening).
const DOOR_FACES: i16 = 2601;

/// The four door kinds (hinged at different sides): 901–903, 911–913,
/// 921–923, 931–933; …1 closed, …2 open, …3 locked (a key opens it). Index
/// 0–3 of the kind, or `None`.
pub fn door_group(kind: i16) -> Option<i16> {
    match kind {
        901..=903 => Some(0),
        911..=913 => Some(1),
        921..=923 => Some(2),
        931..=933 => Some(3),
        _ => None,
    }
}

/// An open door (902, 912, 922, 932).
pub fn door_open(kind: i16) -> bool {
    matches!(kind, 902 | 912 | 922 | 932)
}

/// `SetupDoorSprite`. An open door starts at frame 16 (fully open).
pub fn setup_door(sp: &mut Sprite) {
    sp.init();
    sp.dir = 0;
    sp.task = Some(Task::Door);
    sp.hit_task = Some(HitTask::Door);
    sp.hit_tile_task = None;
    sp.layer = 5;
    if door_open(sp.kind) {
        sp.dir = 16;
    }
    if let Some(g) = door_group(sp.kind) {
        sp.hot_rect = [[34, 15, 37, 46], [15, 16, 18, 47], [3, 33, 34, 36], [1, 4, 32, 7]][g as usize];
        sp.face = if door_open(sp.kind) { DOOR_FACES + 9 * g + 8 } else { DOOR_FACES + 9 * g + sp.dir };
    }
    sp.at_from_pos();
    sp.b8b = true;
}

/// `HandleDoorSprite`: a door Harry kicked open (dir 1–16) swings one
/// step per frame (a face every two) and is open from its first step on -
/// also in the level's sprite list, so it stays open.
pub fn handle_door(sp: &mut Sprite, level: &mut Level) {
    if sp.frozen {
        return;
    }
    if sp.hit_task.is_none() {
        sp.hot_rect = [0; 4];
    }
    if let Some(g) = door_group(sp.kind) {
        sp.face = DOOR_FACES + 9 * g + sp.dir / 2;
    }
    if 0 < sp.dir && sp.dir < 17 {
        sp.dir += 1;
        if let Some(g) = door_group(sp.kind).filter(|_| !door_open(sp.kind)) {
            sp.kind = 902 + 10 * g;
        }
        if let Some(e) = usize::try_from(sp.level_index).ok().and_then(|i| level.sprites.get_mut(i)) {
            e.id = sp.kind;
        }
    }
}

/// `HitDoorSprite`: a closed door stops shots.
pub fn hit_door(sp: &Sprite, other: &mut Sprite) {
    if !sp.frozen
        && !other.frozen
        && matches!(other.task, Some(Task::EnemyShot | Task::PlayerShot))
        && !door_open(sp.kind)
    {
        other.frozen = true;
    }
}
