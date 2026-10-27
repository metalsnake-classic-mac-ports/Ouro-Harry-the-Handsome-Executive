//! Shared helpers of the sprite tests: the recorder's dump of a sprite
//! (`re/tools/sprites.py`).
#![allow(dead_code)]

use harry_core::sprite::{HitTask, HitTileTask, Sprite, Task};

pub fn task_name(t: Option<Task>) -> &'static str {
    match t {
        None => "-",
        Some(Task::Player) => "HandlePlayerSprite",
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
    }
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
    out.push(format!(
        "tile={}",
        match s.hit_tile_task {
            None => "-",
            Some(HitTileTask::Player) => "HitPlayerTileSprite",
            Some(HitTileTask::PlayerShot) => "HitPlayerShotTileSprite",
            Some(HitTileTask::Plant) => "HitPlantTileSprite",
        }
    ));
    out.join(" ")
}

