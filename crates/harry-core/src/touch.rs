//! Harry touches another sprite (`HitPlayerSprite`, PPC 0x10029e74, called
//! by `CheckPlayerCollision` for every sprite whose hot rect overlaps his).
//! One long chain by the other sprite's task; the branches come with the
//! sprite kinds.

use crate::bounce::{Body, rect_bounce};
use crate::frame::{Harry, PlayerGlobals};
use crate::inventory::{current_item, get_item, item, remove_item};
use crate::kinds::{door_group, door_open};
use crate::level::Level;
use crate::physics::Physics;
use crate::player::HOT_RECT;
use crate::ramp::{self, Rider, handle_ramp_collision};
use crate::sat::absolute;
use crate::sprite::{Sprite, Task};
use crate::tiles::{KICK_RECTS, offset, sect};

/// What touching asks of the rest of the game, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchEvent {
    /// `STPlayRegSound`.
    Sound(i16),
    /// A sound that becomes the voice (only when no voice is playing).
    VoiceSound(i16),
    /// `HaltTalkingSounds`: stop all voice samples.
    HaltTalking,
    /// Stop the last remark (`uRam10068a98`) and play this one in its place.
    Remark(i16),
    /// `Ouch`: halt the voices and, unless caffeine works, play one of
    /// three (`SATRand(100)`: below 43, below 86, else).
    Ouch,
}

/// `snd` ids.
pub mod snd {
    pub const VOICE_COMFORT_10: i16 = 210;
    pub const VOICE_COMFORT_8: i16 = 211;
    pub const VOICE_COMFORT_20: i16 = 212;
    pub const PICK_UP: i16 = 214;
    pub const AMMO: i16 = 216;
    pub const COMFORT_FULL: i16 = 218;
    pub const AMMO_FULL: i16 = 219;
    pub const DOOR_KICK: i16 = 204;
    pub const DOOR_LOCKED: i16 = 205;
    pub const UNLOCK: i16 = 240;
    /// The gun shot hitting Harry (251), other enemy shots (223).
    pub const GUN_SHOT_HIT: i16 = 251;
    pub const SHOT_HIT: i16 = 223;
}

/// What else `HitPlayerSprite` reads and changes.
pub struct TouchWorld<'a> {
    pub level: &'a mut Level,
    pub physics: &'a Physics,
    /// Is a voice sample playing (then the voice of a bonus is skipped)?
    pub voice_playing: bool,
}

/// `HitPlayerSprite`.
pub fn touch(h: &mut Harry, g: &mut PlayerGlobals, other: &mut Sprite, w: &mut TouchWorld, events: &mut Vec<TouchEvent>) {
    if h.frozen || other.frozen {
        return;
    }
    let on_floor = h.player.air == 0;
    let voice_playing = w.voice_playing;
    match other.task {
        Some(Task::Ramp) if on_floor && h.player.hot_rect[1] != 0 => {
            let p = &mut h.player;
            let mut r = Rider { body: &mut p.body, air: &mut p.air, cooldown: &mut h.ramp_cooldown, player: true };
            if handle_ramp_collision(&mut r, 32, HOT_RECT, other, w.physics.bounce) {
                events.push(TouchEvent::Sound(ramp::LAUNCH_SOUND));
            }
        }
        Some(Task::Door) => door(h, g, other, w, events),
        Some(Task::Collider) if on_floor && !matches!(other.kind, 3100 | 3101) => collider(h, g, other, w.physics),
        Some(Task::DartEnemy) if on_floor => collider(h, g, other, w.physics),
        Some(Task::EnemyShot) if on_floor => enemy_shot(h, g, other, events),
        Some(Task::Effect) if on_floor => effect(h, g, other, events),
        Some(Task::Bonus) if on_floor => bonus(g, other, voice_playing, events),
        Some(Task::Item) if on_floor => pick_up(g, other, events),
        _ => {}
    }
}

/// Bonuses: 700–703 comfort (only when not full; 704–710 vanish without
/// effect), 711–713 ammo for the staple gun, B.A.D.A.S.S., shrapnel gun
/// (+20 while below 100, so up to 119). When full, a remark (at most every
/// 200 frames, `t24`).
fn bonus(g: &mut PlayerGlobals, other: &mut Sprite, voice_playing: bool, events: &mut Vec<TouchEvent>) {
    let full = |g: &mut PlayerGlobals, remark: i16, events: &mut Vec<TouchEvent>| {
        if g.t24 == 0 {
            g.t24 = 200;
            events.push(TouchEvent::HaltTalking);
            events.push(TouchEvent::Sound(remark));
        }
    };
    if other.kind < 711 {
        if g.comfort >= 144 {
            full(g, snd::COMFORT_FULL, events);
            return;
        }
        other.frozen = true;
        let (voice, gain) = match other.kind {
            700 | 701 => (snd::VOICE_COMFORT_10, 10),
            702 => (snd::VOICE_COMFORT_20, 20),
            703 => (snd::VOICE_COMFORT_8, 8),
            _ => return,
        };
        if !voice_playing {
            events.push(TouchEvent::VoiceSound(voice));
        }
        g.comfort = g.comfort.wrapping_add(gain);
        return;
    }
    let ammo = match other.kind {
        711 => &mut g.ammo.staple,
        712 => &mut g.ammo.badass,
        713 => &mut g.ammo.shrapnel,
        _ => return,
    };
    if *ammo < 100 {
        *ammo = ammo.wrapping_add(20);
        events.push(TouchEvent::HaltTalking);
        events.push(TouchEvent::Sound(snd::AMMO));
        other.frozen = true;
    } else {
        full(g, snd::AMMO_FULL, events);
    }
}

/// Items: a gun Harry already has gives 50 ammo (up to 100) and vanishes;
/// anything else goes into the inventory if there is room.
fn pick_up(g: &mut PlayerGlobals, other: &mut Sprite, events: &mut Vec<TouchEvent>) {
    let have = g.inventory.iter().any(|e| e.0 == other.kind);
    let mut take = true;
    let ammo = match other.kind {
        item::STAPLE_GUN => Some(&mut g.ammo.staple),
        item::BADASS => Some(&mut g.ammo.badass),
        item::SHRAPNEL_GUN => Some(&mut g.ammo.shrapnel),
        _ => None,
    };
    if let Some(ammo) = ammo {
        other.frozen = true;
        take = !have;
        *ammo = ammo.wrapping_add(50).min(100);
    }
    if take && get_item(g, other.kind) {
        other.frozen = true;
        events.push(TouchEvent::Sound(snd::PICK_UP));
    }
}

/// Enemy shots: most stop on Harry; unless he was hurt within the last
/// few frames (the ouch cooldown) he says ouch, loses comfort and takes
/// some of the shot's speed. Darts and paper (below 1100): −12, speed /
/// 16, 3 frames; the can (9610, not stopped): −7, / 4, 12; the gun shot
/// (9450): −10, / 4, 12, with its sound; robot and turret shots: −10,
/// / 16, 12, no ouch; a homing shot (1101–1299) bursts (timer −16, no
/// cooldown): −10, / 16; the boss's 2281: favor −4, / 32, 5; 2291: −7,
/// / 32, 5; other 2001–2999: −4, / 32, 3.
fn enemy_shot(h: &mut Harry, g: &mut PlayerGlobals, other: &mut Sprite, events: &mut Vec<TouchEvent>) {
    let s = &mut h.player.body.speed;
    let (oh, ov) = (other.speed.h, other.speed.v);
    let mut push = |shift: u32| {
        s.h = s.h.wrapping_add(oh >> shift);
        s.v = s.v.wrapping_add(ov >> shift);
    };
    let st = &mut g.tile;
    let kind = other.kind;
    if kind < 1100 {
        other.frozen = true;
        if st.ouch_cooldown == 0 {
            st.ouch_cooldown = 3;
            events.push(TouchEvent::Ouch);
            st.comfort_delta = st.comfort_delta.wrapping_sub(12);
            push(4);
        }
    } else if !(2001..=2999).contains(&kind) {
        match kind {
            9610 => {
                if st.ouch_cooldown == 0 {
                    st.ouch_cooldown = 12;
                    events.push(TouchEvent::Ouch);
                    st.comfort_delta = st.comfort_delta.wrapping_sub(7);
                    push(2);
                }
            }
            9450 => {
                other.frozen = true;
                if st.ouch_cooldown == 0 {
                    st.ouch_cooldown = 12;
                    events.push(TouchEvent::Ouch);
                    events.push(TouchEvent::Sound(snd::GUN_SHOT_HIT));
                    st.comfort_delta = st.comfort_delta.wrapping_sub(10);
                    push(2);
                }
            }
            1101..=1299 => {
                if other.timer == 0 {
                    other.timer = -16;
                    events.push(TouchEvent::Ouch);
                    st.comfort_delta = st.comfort_delta.wrapping_sub(10);
                    push(4);
                }
            }
            _ => {
                other.frozen = true;
                if st.ouch_cooldown == 0 {
                    st.ouch_cooldown = 12;
                    events.push(TouchEvent::Sound(snd::SHOT_HIT));
                    st.comfort_delta = st.comfort_delta.wrapping_sub(10);
                    push(4);
                }
            }
        }
    } else {
        other.frozen = true;
        if st.ouch_cooldown == 0 {
            events.push(TouchEvent::Ouch);
            match kind {
                2281 => {
                    st.ouch_cooldown = 5;
                    g.favor = g.favor.wrapping_sub(4);
                }
                2291 => {
                    st.ouch_cooldown = 5;
                    st.comfort_delta = st.comfort_delta.wrapping_sub(7);
                }
                _ => {
                    st.ouch_cooldown = 3;
                    st.comfort_delta = st.comfort_delta.wrapping_sub(4);
                }
            }
            push(5);
        }
    }
}

/// Effects: an explosion pushes Harry (`HandleExplodeRadius`, half 32)
/// and, in its frames 6–19 and unless he was hurt lately, costs 8 comfort
/// (ouch, cooldown 15). A flame (`f7c` from 4200) in its frames 7–24
/// costs 10 (cooldown 15) and is set to `f6e` 3.
fn effect(h: &mut Harry, g: &mut PlayerGlobals, other: &mut Sprite, events: &mut Vec<TouchEvent>) {
    let st = &mut g.tile;
    if other.f7c < crate::effect::FIRE {
        let b = &mut h.player.body;
        crate::effect::explode_radius(b.at, &mut b.speed, &mut h.blast_cooldown, 32, other);
        if st.ouch_cooldown == 0 && 5 < other.dir && other.dir < 20 {
            st.comfort_delta = st.comfort_delta.wrapping_sub(8);
            st.ouch_cooldown = 15;
            events.push(TouchEvent::Ouch);
        }
    } else if 6 < other.dir && other.dir < 25 && st.ouch_cooldown == 0 {
        st.comfort_delta = st.comfort_delta.wrapping_sub(10);
        st.ouch_cooldown = 15;
        other.f6e = 3;
        events.push(TouchEvent::Ouch);
    }
}

/// Colliders other than the phone and dart enemies (and scientists and
/// co-workers, which come with their kinds): Harry bounces off the hot
/// rect (his normal one, the level's bounce) - once a frame (`flag50`),
/// whatever he touches after that in the same frame does nothing here.
/// Then: the gate (9800) with the gate key selected takes it, starts
/// rising and gives 5000 points; a kick at the vending machine (9600)
/// with its middle above Harry's starts its timer, else (and at the
/// copier) the kick hits something solid.
fn collider(h: &mut Harry, g: &mut PlayerGlobals, other: &mut Sprite, physics: &Physics) {
    if g.flag50 {
        return;
    }
    let p = &mut h.player;
    rect_bounce(&mut p.body, 32, i32::from(physics.bounce), HOT_RECT, other.at, other.hot_rect);
    g.flag50 = true;
    let kick_hits = |p: &crate::player::Player, other: &Sprite| {
        let kick = offset(KICK_RECTS[p.dir as usize], p.body.at.0, p.body.at.1);
        p.hot_rect[1] == 0 && sect(kick, absolute(other.hot_rect, other.at))
    };
    match other.kind {
        9800 => {
            if g.cur_slot != -1 && g.inventory[g.cur_slot as usize].0 == item::GATE_KEY {
                remove_item(g, g.cur_slot);
                other.timer = other.timer.wrapping_add(1);
                g.score = g.score.wrapping_add(5000);
            }
        }
        9600 if kick_hits(p, other) => {
            let mid = i32::from(other.at.0) + ((i32::from(other.hot_rect[2]) + i32::from(other.hot_rect[0])) >> 1);
            if mid < i32::from(p.body.at.0) + 32 {
                if other.timer == 0 {
                    other.timer = 3;
                }
            } else {
                g.tile.kick_contact = 0x100;
            }
        }
        9199 | 9200 if kick_hits(p, other) => g.tile.kick_contact = 0x100,
        _ => {}
    }
}

/// The key that opens a locked door: item 11399 − the door's info #0.
fn door_key(level: &Level, door: &Sprite) -> i16 {
    let info = usize::try_from(door.level_index).ok().and_then(|i| level.sprites.get(i)).map_or(0, |e| e.info[0]);
    (0x2c87 - i32::from(info)) as i16
}

/// Unlock a locked door with the selected item if it is its key: the
/// key is used up, the door (and its entry in the level) becomes closed.
fn unlock(g: &mut PlayerGlobals, door: &mut Sprite, level: &mut Level, events: &mut Vec<TouchEvent>) -> bool {
    if door_key(level, door) != current_item(g) {
        return false;
    }
    events.push(TouchEvent::Sound(snd::UNLOCK));
    remove_item(g, g.cur_slot);
    door.kind -= 2;
    if let Some(e) = usize::try_from(door.level_index).ok().and_then(|i| level.sprites.get_mut(i)) {
        e.id = e.id.wrapping_sub(2);
    }
    true
}

/// Doors. Kicking a closed door from the side it opens to, facing the
/// right way, swings it open (and the kick stops); from the other side,
/// facing it, it is solid (kick contact); otherwise Harry is pushed out.
/// Walking into it pushes him back with the level's bounce.
fn door(h: &mut Harry, g: &mut PlayerGlobals, other: &mut Sprite, w: &mut TouchWorld, events: &mut Vec<TouchEvent>) {
    let Some(group) = door_group(other.kind).filter(|_| !door_open(other.kind)) else { return };
    let locked = other.kind % 10 == 3;
    let p = &mut h.player;
    if p.hot_rect[1] != 0 {
        if locked {
            if unlock(g, other, w.level, events) {
                other.b8a = true;
            } else if g.t28 == 0 {
                g.t28 = 25;
                events.push(TouchEvent::Remark(snd::DOOR_LOCKED));
            }
        }
        door_separate(&mut p.body, p.hot_rect, other);
        let bounce = i32::from(w.physics.bounce);
        p.body.speed.h = (((bounce * i32::from(p.body.speed.h)) >> 8) as i16).wrapping_neg();
        p.body.speed.v = (((bounce * i32::from(p.body.speed.v)) >> 8) as i16).wrapping_neg();
        return;
    }
    // the kick
    let me = (g.player_pos.0.wrapping_add(32), g.player_pos.1.wrapping_add(32));
    // the hinge; for a locked door the original reads two never-set
    // locals here (BUGS.md) - the port takes the closed door's values
    let (dh, dv) = [(30, 35), (31, 17), (34, 19), (5, 18)][group as usize];
    let hinge = (other.pos.1.wrapping_add(dh), other.pos.0.wrapping_add(dv));
    let d = p.dir;
    let (front, facing_in, facing_out) = match group {
        0 => (hinge.1 < me.1, 0 < d && d < 18, 18 < d && d < 35),
        1 => (me.1 < hinge.1, 18 < d && d < 35, 0 < d && d < 18),
        2 => (hinge.0 < me.0, 9 < d && d < 27, d < 9 || 27 < d),
        _ => (me.0 < hinge.0, d < 9 || 27 < d, 9 < d && d < 27),
    };
    let mut open = front && facing_in;
    let solid = !front && facing_out;
    let sound = open || solid;
    if locked && !unlock(g, other, w.level, events) {
        open = false;
    }
    if solid {
        g.tile.kick_contact = 0x100;
    }
    if open {
        g.tile.kick_contact = 0;
        g.kick_reset = true;
        other.dir = other.dir.wrapping_add(1);
    } else if !solid {
        // the kick's hot rect is not put back (BUGS.md)
        p.hot_rect = HOT_RECT;
        rect_separate(&mut p.body, p.hot_rect, other);
    }
    if sound {
        events.push(TouchEvent::Sound(snd::DOOR_KICK));
    }
}

/// `RectSeparate` (PPC 0x10053c6c): move the sprite (`pos`) out of the
/// other one's hot rect along the shortest of the four distances (ties:
/// the first). Returns the side (0 up, 1 down, 2 right, 3 left).
pub fn rect_separate(body: &mut Body, hot: crate::bounce::Rect, other: &Sprite) -> usize {
    let a = crate::sat::absolute(hot, body.pos);
    let b = crate::sat::absolute(other.hot_rect, other.pos);
    let d = [b[0].wrapping_sub(a[2]), b[2].wrapping_sub(a[0]), b[3].wrapping_sub(a[1]), b[1].wrapping_sub(a[3])];
    let abs = |x: i16| if x > 0 { i32::from(x) } else { -i32::from(x) };
    let mut best = 0;
    let mut min = abs(d[0]) as i16;
    for (i, &x) in d.iter().enumerate().skip(1) {
        if abs(x) < i32::from(min) {
            best = i;
            min = abs(x) as i16;
        }
    }
    if best < 2 {
        body.pos.0 = body.pos.0.wrapping_add(d[best]);
    } else {
        body.pos.1 = body.pos.1.wrapping_add(d[best]);
    }
    best
}

/// `DoorSeparate` (PPC 0x10029c08): put Harry back on his side of a door
/// (decided by his centre before the move), from his fixed position.
fn door_separate(body: &mut Body, hot: crate::bounce::Rect, door: &Sprite) {
    let centre = (body.pos.0.wrapping_add(32), body.pos.1.wrapping_add(32));
    let r = crate::sat::absolute(door.hot_rect, door.pos);
    body.pos = ((body.fix.1 >> 8) as i16, (body.fix.0 >> 8) as i16);
    let old = body.pos;
    let mid = |base: i16, a: i16, b: i16| base.wrapping_add(((i32::from(a) + i32::from(b)) >> 1) as i16);
    if door.kind < 920 {
        if centre.0 < mid(door.pos.0, door.hot_rect[0], door.hot_rect[2]) {
            if i32::from(r[0]) - i32::from(hot[2]) < i32::from(body.pos.0) {
                body.pos.0 = r[0].wrapping_sub(hot[2]);
            }
        } else if i32::from(body.pos.0) < i32::from(r[2]) - i32::from(hot[0]) {
            body.pos.0 = r[2].wrapping_sub(hot[0]);
        }
    } else if centre.1 < mid(door.pos.1, door.hot_rect[1], door.hot_rect[3]) {
        if i32::from(r[1]) - i32::from(hot[3]) < i32::from(body.pos.1) {
            body.pos.1 = r[1].wrapping_sub(hot[3]);
        }
    } else if i32::from(body.pos.1) < i32::from(r[3]) - i32::from(hot[1]) {
        body.pos.1 = r[3].wrapping_sub(hot[1]);
    }
    if old.1 != body.pos.1 {
        body.fix.0 = i32::from(body.pos.1) << 8;
    }
    if old.0 != body.pos.0 {
        body.fix.1 = i32::from(body.pos.0) << 8;
    }
}
