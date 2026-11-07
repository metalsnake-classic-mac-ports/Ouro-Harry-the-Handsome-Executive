//! The boss (2201, level 15): a spinning machine that turns about, sweeps
//! the room with shots and, now and then, aims at Harry and fires three
//! at once. 150 hit points; below 100 it pulses and shoots homing ones,
//! below 50 it pulses wildly and lets out ghosts. Its turning state lives
//! in globals (`BossState`).

use crate::physics::Speed;
use crate::random::Random;
use crate::robot::toward_harry;
use crate::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};
use crate::tiles::{KICK_RECTS, offset, sect};

/// `snd` ids.
pub mod snd {
    /// The spin-up as it changes phase, and its death cries.
    pub const ROAR: i16 = 4901;
    pub const SHOT: i16 = 300;
    pub const VOLLEY: i16 = 1002;
    pub const HURT: [i16; 2] = [1000, 1001];
    pub const KICKED: i16 = 600;
    pub const EXPLODE: i16 = 602;
}

/// 36 faces (PICT 2201).
const FACES: i16 = 4902;

/// The boss's globals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BossState {
    /// `_DAT_1006c350`: 1 (150–100 points), 2 (below 100), 3 (below 50).
    pub phase: i16,
    /// `DAT_1006c352`: the pulse is growing.
    pub growing: bool,
    /// `PTR_DAT_10065f80`: −100 at a phase change, counting up (it spins
    /// in place, the view shaking).
    pub change: i16,
    /// `PTR_DAT_10065f7c`: 0–7, for the drawing.
    pub anim: i16,
    /// `PTR_DAT_10065f5c`: frames into the current sweep (negative while
    /// it fires a volley).
    pub count: i16,
    /// `PTR_DAT_10065f58`: turning speed (1/256 direction a frame).
    pub spin: i16,
    /// `PTR_DAT_10065f54`: direction × 256.
    pub angle: i32,
    /// `PTR_DAT_10065f3c`: frames until the next sweep shot.
    pub cooldown: i16,
    /// `PTR_DAT_10065f44`: frames until an explosion can hurt it again.
    pub blast_cooldown: i16,
}

const UNIT: [f64; 10] = [1.0, 0.985, 0.94, 0.866, 0.766, 0.643, 0.5, 0.342, 0.174, 0.0];

/// `AssignBossShotSpeed`: `m` along `dir` added to `speed`.
fn add_speed(speed: &mut Speed, dir: i16, m: i16) {
    if !(0..36).contains(&dir) {
        return;
    }
    let m = f64::from(m);
    let k = (dir % 9) as usize;
    let (along, across) = (UNIT[k] * m, UNIT[9 - k] * m);
    let (dh, dv) = match dir / 9 {
        0 => (along, -across),
        1 => (-across, -along),
        2 => (-along, across),
        _ => (across, along),
    };
    speed.h = (f64::from(speed.h) + dh) as i32 as i16;
    speed.v = (f64::from(speed.v) + dv) as i32 as i16;
}

/// Two to one side, wrapping with 35 (as the original).
fn side(dir: i16, d: i16) -> i16 {
    let x = dir + d;
    if x > 35 {
        x - 35
    } else if x < 0 {
        x + 35
    } else {
        x
    }
}

/// `SetupBossSprite`: a random direction and wait, layer 7, hot rect (8,
/// 8, 56, 56), 150 hit points; the sweep reset, phase 1. (A record that
/// was on layer 10 gets +0x8a set first.)
pub fn setup_boss(sp: &mut Sprite, rng: &mut Random, boss: &mut BossState) {
    if sp.layer == 10 {
        sp.b8a = true;
    }
    sp.init();
    sp.dir = rng.below(35);
    sp.task = Some(Task::Boss);
    sp.hit_task = Some(HitTask::Boss);
    sp.hit_tile_task = Some(HitTileTask::Boss);
    sp.hot_rect = [8, 8, 56, 56];
    sp.layer = 7;
    sp.speed = Speed::default();
    sp.at_from_pos();
    sp.fix_from_pos();
    sp.f6e = 4;
    sp.timer = rng.below(90);
    sp.state = 150;
    sp.face = FACES + sp.dir;
    sp.b8b = true;
    boss.count = 0;
    boss.spin = 0;
    boss.angle = 0;
    boss.phase = 1;
    boss.change = 0;
    sp.timer = 0;
}

/// The shot it sweeps with: from at + 26, 5 frames out along `out`, then
/// flying 1500 along `along` (`fix` set, `pos` from the spawn).
fn launch(sp: &Sprite, out: i16, along: i16) -> (Speed, (i32, i32)) {
    let (h, v) = (sp.at.1.wrapping_add(26), sp.at.0.wrapping_add(26));
    let mut s = Speed::default();
    add_speed(&mut s, out, 1500);
    let fix = ((i32::from(h) << 8) + 5 * i32::from(s.h), (i32::from(v) << 8) + 5 * i32::from(s.v));
    let mut speed = Speed::default();
    add_speed(&mut speed, along, 1500);
    (speed, fix)
}

/// The three-shot volley (shot 2291): one from at + 26 sent 5 frames out to
/// one side (two directions down) and flying straight along its direction,
/// two more from where it ends up, two to each side; phase 3 shoots twice as
/// fast.
fn volley(sp: &Sprite, fast: bool, last_dir_zero: bool, events: &mut Vec<SpriteEvent>) {
    let (h, v) = (sp.at.1.wrapping_add(26), sp.at.0.wrapping_add(26));
    let k = if fast { 2 } else { 1 };
    let mut out = Speed::default();
    add_speed(&mut out, side(sp.dir, -2), 1500);
    let fix = ((i32::from(h) << 8) + 5 * i32::from(out.h), (i32::from(v) << 8) + 5 * i32::from(out.v));
    let mut speed = Speed::default();
    add_speed(&mut speed, sp.dir, 1500);
    speed = Speed { h: speed.h.wrapping_mul(k), v: speed.v.wrapping_mul(k) };
    events.push(SpriteEvent::Shot { kind: 2291, h, v, speed, dir: sp.dir, fix: Some(fix), to_fix: false, timer: None, f7c: None });
    let (h2, v2) = ((fix.0 >> 8) as i16, (fix.1 >> 8) as i16);
    for (n, d) in [side(sp.dir, 2), side(sp.dir, -2)].into_iter().enumerate() {
        let mut s = Speed::default();
        add_speed(&mut s, d, 1500);
        s = Speed { h: s.h.wrapping_mul(k), v: s.v.wrapping_mul(k) };
        let dir = if n == 1 && last_dir_zero { 0 } else { d };
        events.push(SpriteEvent::Shot { kind: 2291, h: h2, v: v2, speed: s, dir, fix: None, to_fix: false, timer: None, f7c: None });
    }
}

/// `HandleBossSprite`. Each frame: half pipes (centre + 16), jumps; the
/// flash counter `f6e`; no speed of its own. Out of hit points it starts
/// dying (`HandleBossSpriteDeath`). The phase follows its hit points; a
/// change spins it in place for 100 frames, the view shaking, with a roar.
/// Otherwise it sweeps: the turning speed drifts one way (a coin at the
/// start of a sweep) and builds up to ±600 over 60 frames, settling at
/// ±256 after 100; while it turns fast (beyond ±200) it fires - phase 1
/// the shot 2281, phase 2 homing shots, phase 3 ghosts (5011); after 120
/// frames, facing Harry, it stops and fires 3 (phase 3: 5) volleys of
/// three, then (phases 2 and 3) turns to face him and starts a new sweep.
/// Phases 2 and 3 pulse in size (not when far left of the view's
/// middle). `view` is the view's corner (v, h), `player` Harry's top-left
/// (h, v), `shake` the view shake counter.
#[allow(clippy::too_many_arguments)]
pub fn handle_boss(
    sp: &mut Sprite,
    boss: &mut BossState,
    level: &crate::level::Level,
    view: (i16, i16),
    player: (i16, i16),
    rng: &mut Random,
    shake: &mut i16,
    events: &mut Vec<SpriteEvent>,
) {
    if sp.frozen {
        return;
    }
    let mut body = sp.body();
    let mut air = sp.air_state();
    let mut s = Vec::new();
    crate::air::half_pipe(&mut body, &mut air, 16, false, level, &mut s);
    if air.air > 0 {
        let mut unused_comfort = 0;
        crate::air::jump_scale(&mut body, &mut air, &mut sp.dir, 16, false, level, rng, &mut unused_comfort, 0, &mut s);
    }
    sp.set_body(body);
    sp.set_air_state(air);
    sp.f6e += 1;
    let flash = (i32::from(sp.state) + 2) % 51;
    if flash < sp.f6e {
        if flash < 45 {
            sp.f6e = -2;
        }
        sp.b8a = true;
    }
    sp.speed = Speed::default();
    if boss.blast_cooldown > 0 {
        boss.blast_cooldown -= 1;
    }
    if sp.state < 0 {
        sp.task = Some(Task::BossDeath);
        sp.timer = 0;
        return;
    }
    let old = boss.phase;
    boss.phase = if sp.state < 50 {
        3
    } else if sp.state < 100 {
        2
    } else {
        1
    };
    if boss.phase != old {
        boss.change = -100;
    }
    boss.anim = (boss.anim + 1) % 8;
    if boss.change < 0 {
        sp.dir += 2;
        if sp.dir > 35 {
            sp.dir -= 35;
        }
        if boss.change == -98 {
            events.push(SpriteEvent::Sound3DPitched { id: snd::ROAR, at: sp.at, rate: 32000 });
        }
        boss.change += 1;
        *shake = (boss.change & 1) + 16;
        sp.f6e = -1;
    } else {
        let phase = boss.phase;
        if phase >= 2 {
            let (step, top) = if phase == 2 { (4, 0x130) } else { (12, 0x1c0) };
            if !boss.growing {
                sp.scale[0] -= step;
                sp.scale[1] -= step;
                if sp.scale[0] < 0x100 {
                    boss.growing = true;
                }
            } else {
                sp.scale[0] += step;
                sp.scale[1] += step;
                if sp.scale[0] > top {
                    boss.growing = false;
                }
            }
            if i32::from(view.1) + 224 - i32::from(sp.at.1) > 200 {
                sp.scale[0] = 0x100;
                sp.scale[1] = 0x100;
            }
        }
        if boss.count == 0 {
            if rng.below(100) < 50 {
                boss.spin -= 1;
            } else {
                boss.spin += 1;
            }
        }
        boss.count += 1;
        if i32::from(boss.spin).abs() > 200 && boss.cooldown == 0 {
            match phase {
                1 => {
                    let (speed, fix) = launch(sp, side(sp.dir, 2), sp.dir);
                    let (h, v) = (sp.at.1.wrapping_add(26), sp.at.0.wrapping_add(26));
                    events.push(SpriteEvent::Shot { kind: 2281, h, v, speed, dir: sp.dir, fix: Some(fix), to_fix: false, timer: None, f7c: None });
                    boss.cooldown = rng.below(5) + 6;
                }
                2 => {
                    let (speed, fix) = launch(sp, side(sp.dir, 2), sp.dir);
                    let (h, v) = (sp.at.1.wrapping_add(26), sp.at.0.wrapping_add(26));
                    events.push(SpriteEvent::Shot { kind: 1211, h, v, speed, dir: 0, fix: Some(fix), to_fix: false, timer: Some(0), f7c: Some(1) });
                    boss.cooldown = rng.below(25) + 30;
                }
                _ => {
                    events.push(SpriteEvent::Ghost { kind: 5011, h: sp.at.1.wrapping_add(12), v: sp.at.0.wrapping_add(12), dir: sp.dir });
                    boss.cooldown = rng.below(25) + 30;
                }
            }
            let pitch = rng.below(6000) as u32;
            events.push(SpriteEvent::Sound3DPitched { id: snd::SHOT, at: sp.at, rate: pitch + 62000 });
        }
        if boss.cooldown > 0 {
            boss.cooldown -= 1;
        }
        if boss.spin != 0 {
            if boss.spin < 1 {
                if boss.count < 60 && boss.spin > -600 {
                    boss.spin -= 12;
                } else if boss.count > 100 && boss.spin < -256 {
                    boss.spin += 12;
                    if boss.spin > -256 {
                        boss.spin = -256;
                    }
                }
            } else if boss.count < 60 && boss.spin < 600 {
                boss.spin += 12;
            } else if boss.count > 100 && boss.spin > 256 {
                boss.spin -= 12;
                if boss.spin < 256 {
                    boss.spin = 256;
                }
            }
        }
        if phase >= 2 && boss.count < 0 {
            sp.dir = toward_harry(sp, player);
            boss.angle = i32::from(sp.dir) << 8;
            boss.spin = 0;
        }
        if boss.count > 120 && toward_harry(sp, player) == sp.dir {
            boss.spin = 0;
            boss.count = -50;
        }
        let volleys: &[i16] = match phase {
            1 => &[-45, -30, -15],
            2 => &[-45, -35, -25, -15],
            _ => &[-45, -35, -25, -15, -5],
        };
        if volleys.contains(&boss.count) {
            volley(sp, phase == 3, phase == 1, events);
            let pitch = rng.below(6000) as u32;
            events.push(SpriteEvent::Sound3DPitched { id: snd::VOLLEY, at: sp.at, rate: pitch + 62000 });
        }
    }
    turn(sp, boss);
}

/// The direction from the angle, the face, the (zero) move.
fn turn(sp: &mut Sprite, boss: &mut BossState) {
    boss.angle += i32::from(boss.spin);
    while boss.angle > 0x23ff {
        boss.angle -= 0x2400;
    }
    while boss.angle < 0 {
        boss.angle += 0x2400;
    }
    sp.dir = ((boss.angle as u32) >> 8) as i16;
    sp.dir = sp.dir.clamp(0, 35);
    sp.face = FACES + sp.dir;
    sp.integrate();
}

/// `HandleBossSpriteDeath`: it grows to 1.5 × (12 a frame) and spins
/// (800). Frame 0: 10 000 points, every ghost loses its hit points, the
/// music fades, it can no longer be hit, a roar (34000); roars at 8 and
/// 16; twelve explosions about it from frame 50 to 96 (sound 602 at 50000,
/// the last one stopped first); at 114 the big blast - two explosions
/// twice the size on top of everything, three more, the view shaking 48
/// frames; gone after 120.
pub fn handle_boss_death(sp: &mut Sprite, boss: &mut BossState, rng: &mut Random, shake: &mut i16, events: &mut Vec<SpriteEvent>) {
    for s in [0, 1] {
        if sp.scale[s] < 0x180 {
            sp.scale[s] += 12;
        }
    }
    for s in [0, 1] {
        if sp.scale[s] > 0x180 {
            sp.scale[s] -= 12;
        }
    }
    let blast = |rng: &mut Random, sp: &Sprite, events: &mut Vec<SpriteEvent>| {
        let dv = rng.below(64);
        let dh = rng.below(64);
        events.push(SpriteEvent::Explosion { h: sp.at.1.wrapping_add(dh).wrapping_sub(32), v: sp.at.0.wrapping_add(dv).wrapping_sub(32), a: 0, b: 0 });
    };
    match sp.timer {
        0 => {
            events.push(SpriteEvent::Score(10000));
            events.push(SpriteEvent::GhostsDie);
            events.push(SpriteEvent::FadeMusic);
            sp.hit_task = None;
            sp.hit_tile_task = None;
            boss.spin = 800;
            events.push(SpriteEvent::Sound3DPitched { id: snd::ROAR, at: sp.at, rate: 34000 });
        }
        8 => events.push(SpriteEvent::Sound3DPitched { id: snd::ROAR, at: sp.at, rate: 40000 }),
        16 => events.push(SpriteEvent::Sound3DPitched { id: snd::ROAR, at: sp.at, rate: 24000 }),
        50 | 54 | 58 | 66 | 70 | 74 | 78 | 80 | 84 | 88 | 92 | 96 => {
            events.push(SpriteEvent::StopSound(snd::EXPLODE));
            events.push(SpriteEvent::Sound3DPitched { id: snd::EXPLODE, at: sp.at, rate: 50000 });
            blast(rng, sp, events);
        }
        114 => {
            events.push(SpriteEvent::Sound3DPitched { id: snd::EXPLODE, at: sp.at, rate: 24000 });
            events.push(SpriteEvent::BigBlast { h: sp.at.1.wrapping_sub(2), v: sp.at.0.wrapping_sub(2) });
            events.push(SpriteEvent::BigBlast { h: sp.at.1.wrapping_add(2), v: sp.at.0.wrapping_add(2) });
            for _ in 0..3 {
                blast(rng, sp, events);
            }
            *shake = 0x30;
        }
        t if t > 120 => sp.frozen = true,
        _ => {}
    }
    sp.timer += 1;
    turn(sp, boss);
}

/// `HitBossSprite`. An explosion costs 6 hit points (then not again for
/// 30 frames). A player shot: gone, a point, the flash; every 8th point
/// (state ≡ 1) it cries out (1000 or 1001; only when neither plays,
/// `cry_playing`). Harry's kick: sound 600, a point, layer 10, kick
/// contact 64.
#[allow(clippy::too_many_arguments)]
pub fn hit_boss(
    sp: &mut Sprite,
    other: &mut Sprite,
    boss: &mut BossState,
    rng: &mut Random,
    kick_contact: &mut i16,
    cry_playing: bool,
    events: &mut Vec<SpriteEvent>,
) {
    if sp.frozen || other.frozen {
        return;
    }
    match other.task {
        Some(Task::Effect) => {
            if other.hot_rect != [0; 4] && boss.blast_cooldown == 0 {
                sp.state -= 6;
                boss.blast_cooldown = 30;
            }
        }
        Some(Task::PlayerShot) => {
            other.frozen = true;
            sp.state -= 1;
            sp.f6e = -1;
            sp.b8a = true;
            if sp.state % 8 == 1 && !cry_playing {
                let id = if rng.below(100) < 50 { snd::HURT[0] } else { snd::HURT[1] };
                events.push(SpriteEvent::sound_3d(id, sp.centre()));
            }
        }
        Some(Task::Player) if other.hot_rect[1] == 0 => {
            let kick = offset(KICK_RECTS[other.dir as usize], other.at.0, other.at.1);
            if sect(kick, offset(sp.hot_rect, sp.at.0, sp.at.1)) {
                events.push(SpriteEvent::sound_3d(snd::KICKED, sp.centre()));
                sp.state -= 1;
                sp.layer = 10;
                *kick_contact = 0x40;
            }
        }
        _ => {}
    }
}
