//! Shared helpers of the sprite tests: the recorder's dump of a sprite
//! (`re/tools/sprites.py`).
#![allow(dead_code)]

use harry_core::sprite::{HitTask, HitTileTask, Sprite, SpriteEvent, Task};

pub fn task_name(t: Option<Task>) -> &'static str {
    match t {
        None => "-",
        Some(Task::Player) => "HandlePlayerSprite",
        Some(Task::PlayerDeath) => "HandlePlayerSpriteDeath",
        Some(Task::HeldWeapon) => "HandleHeldWeaponSprite",
        Some(Task::PlayerShot) => "HandlePlayerShotSprite",
        Some(Task::Bonus) => "HandleBonusSprite",
        Some(Task::Item) => "HandleItemSprite",
        Some(Task::Exit) => "HandleExitSprite",
        Some(Task::Door) => "HandleDoorSprite",
        Some(Task::EnemyShot) => "HandleEnemyShotSprite",
        Some(Task::Effect) => "HandleEffectSprite",
        Some(Task::Cooler) => "HandleCoolerSprite",
        Some(Task::Plant) => "HandlePlantSprite",
        Some(Task::Message) => "HandleMessageSprite",
        Some(Task::Collider) => "HandleColliderSprite",
        Some(Task::DartEnemy) => "HandleDartEnemySprite",
        Some(Task::Ramp) => "HandleRampSprite",
        Some(Task::Barrel) => "HandleBarrelSprite",
        Some(Task::Chair) => "HandleChairSprite",
        Some(Task::Stairs) => "HandleStairsSprite",
        Some(Task::Button) => "HandleButtonSprite",
        Some(Task::SlidingDoor) => "HandleSlidingDoorSprite",
        Some(Task::CoWorker) => "HandleCoWorkerSprite",
        Some(Task::MailGuy) => "HandleMailGuySprite",
        Some(Task::Robot) => "HandleRobotSprite",
    }
}

pub fn hit_name(t: Option<HitTask>) -> &'static str {
    match t {
        None => "-",
        Some(HitTask::Player) => "HitPlayerSprite",
        Some(HitTask::PlayerShot) => "HitPlayerShotSprite",
        Some(HitTask::Bonus) => "HitBonusSprite",
        Some(HitTask::Item) => "HitItemSprite",
        Some(HitTask::Exit) => "HitExitSprite",
        Some(HitTask::Door) => "HitDoorSprite",
        Some(HitTask::Cooler) => "HitCoolerSprite",
        Some(HitTask::Plant) => "HitPlantSprite",
        Some(HitTask::Message) => "HitMessageSprite",
        Some(HitTask::Collider) => "HitColliderSprite",
        Some(HitTask::EnemyShot) => "HitEnemyShotSprite",
        Some(HitTask::DartEnemy) => "HitDartEnemySprite",
        Some(HitTask::Ramp) => "HitRampSprite",
        Some(HitTask::Barrel) => "HitBarrelSprite",
        Some(HitTask::Chair) => "HitChairSprite",
        Some(HitTask::Stairs) => "HitStairsSprite",
        Some(HitTask::Button) => "HitButtonSprite",
        Some(HitTask::SlidingDoor) => "HitSlidingDoorSprite",
        Some(HitTask::MailGuy) => "HitMailGuySprite",
        Some(HitTask::Robot) => "HitRobotSprite",
    }
}

pub fn tile_name(t: Option<HitTileTask>) -> &'static str {
    match t {
        None => "-",
        Some(HitTileTask::Player) => "HitPlayerTileSprite",
        Some(HitTileTask::PlayerShot) => "HitPlayerShotTileSprite",
        Some(HitTileTask::Plant) => "HitPlantTileSprite",
        Some(HitTileTask::EnemyShot) => "HitEnemyShotTileSprite",
        Some(HitTileTask::Ramp) => "HitRampTileSprite",
        Some(HitTileTask::Barrel) => "HitBarrelTileSprite",
        Some(HitTileTask::Chair) => "HitChairTileSprite",
        Some(HitTileTask::MailGuy) => "HitMailGuyTileSprite",
        Some(HitTileTask::Robot) => "HitRobotTileSprite",
    }
}

/// The recorders' event names: p:<snd>, 3d:<snd>, 3d:<snd>:<rate>
/// (pitched), x:<h>:<v>:<a>:<b> and x:<h>:<v>:<speed h>:<speed v>:<a>:<b>
/// (EasyExplosion), n:<kind>:<h>:<v>:<speed v>:<speed h> (SATNewSprite).
pub fn event(e: &SpriteEvent) -> String {
    match *e {
        SpriteEvent::Sound(s) => format!("p:{s}"),
        SpriteEvent::Sound3D(s) => format!("3d:{s}"),
        SpriteEvent::Sound3DPitched { id, rate } => format!("3d:{id}:{rate}"),
        SpriteEvent::Explosion { h, v, a, b } => format!("x:{h}:{v}:{a}:{b}"),
        SpriteEvent::MovingExplosion { h, v, speed, a, b } => format!("x:{h}:{v}:{}:{}:{a}:{b}", speed.h, speed.v),
        SpriteEvent::DynamicSound { id, volume, speed } => format!("dyn:{id}:{volume}:{}:{}", speed.v, speed.h),
        SpriteEvent::Score(n) => format!("score:{n}"),
        SpriteEvent::Shot { kind, h, v, speed, dir, fix } => {
            let (fh, fv) = fix.unwrap_or((i32::from(h) << 8, i32::from(v) << 8));
            format!("shot:{kind}:{h}:{v}:{}:{}:{dir}:{fh}:{fv}", speed.v, speed.h)
        }
        SpriteEvent::Generate { kind, h, v } => format!("gen:{kind}:{h}:{v}"),
        SpriteEvent::Forget(i) => format!("forget:{i}"),
        SpriteEvent::Spawn { kind, h, v, speed } => {
            let s = speed.unwrap_or_default();
            format!("n:{kind}:{h}:{v}:{}:{}", s.v, s.h)
        }
    }
}

/// The events joined by commas, "-" for none.
pub fn events(events: &[SpriteEvent]) -> String {
    let ev: Vec<String> = events.iter().map(event).collect();
    if ev.is_empty() { "-".into() } else { ev.join(",") }
}

/// The recorder's dump (`re/tools/sprites.py`).
pub fn dump(s: &Sprite) -> String {
    let b = |x: bool| i32::from(x);
    let fields: Vec<(&str, i64)> = vec![
        ("kind", s.kind.into()),
        ("posV", s.pos.0.into()),
        ("posH", s.pos.1.into()),
        ("hotT", s.hot_rect[0].into()),
        ("hotL", s.hot_rect[1].into()),
        ("hotB", s.hot_rect[2].into()),
        ("hotR", s.hot_rect[3].into()),
        ("layer", s.layer.into()),
        ("speedV", s.speed.v.into()),
        ("speedH", s.speed.h.into()),
        ("scale0", s.scale[0].into()),
        ("scale1", s.scale[1].into()),
        ("scale2", s.scale[2].into()),
        ("scale3", s.scale[3].into()),
        ("atV", s.at.0.into()),
        ("atH", s.at.1.into()),
        ("fixH", s.fix.0.into()),
        ("fixV", s.fix.1.into()),
        ("dir", s.dir.into()),
        ("levelIndex", s.level_index.into()),
        ("f6e", s.f6e.into()),
        ("state", s.state.into()),
        ("timer", s.timer.into()),
        ("idleIndex", s.idle_index.into()),
        ("f78", s.f78.into()),
        ("f7c", s.f7c.into()),
        ("f82", s.f82.into()),
        ("f84", s.f84.into()),
        ("f86", s.f86.into()),
        ("b8a", b(s.b8a).into()),
        ("b8b", b(s.b8b).into()),
        ("b8c", b(s.b8c).into()),
        ("air", s.air.into()),
        ("flip", s.flip.into()),
        ("frozen", b(s.frozen).into()),
        ("f94", s.f94.into()),
        ("blast", s.blast_cooldown.into()),
        ("lastTileH", s.last_tile.0.into()),
        ("lastTileV", s.last_tile.1.into()),
        ("slide", s.slide.into()),
        ("face", s.face.into()),
    ];
    let mut out: Vec<String> = fields.iter().map(|(n, v)| format!("{n}={v}")).collect();
    out.push(format!("task={}", task_name(s.task)));
    out.push(format!("hit={}", hit_name(s.hit_task)));
    out.push(format!("tile={}", tile_name(s.hit_tile_task)));
    out.join(" ")
}

