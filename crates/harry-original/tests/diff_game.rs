//! Whole frames of level 1 against the original, recorded in Unicorn by
//! `re/tools/emu_game.py`.

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use common::dump;
use harry_core::frame::PlayerGlobals;
use harry_core::game::{Assets, FrameEnd, Game, Prefs};
use harry_core::keys::{Bindings, KeyMap};
use harry_core::kinds::LevelCounter;
use harry_core::player::Masks;
use harry_core::random::Random;
use harry_core::view::View;
use harry_core::world::World;
use harry_original::resfork::{MacBinary, Resources};
use harry_original::{hlvl, hrsp, masks};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn installed(name: &str) -> Option<Resources> {
    let bytes = std::fs::read(root().join("re/work/installed").join(name)).ok()?;
    Some(Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap())
}

#[test]
fn level_1_frames_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/game.txt")) else {
        eprintln!("skipped: game.txt not found (run re/tools/emu_game.py)");
        return;
    };
    let (Some(app), Some(gfx), Some(ep)) = (installed("Harry 1.0.1.bin"), installed("Harry Graphics.bin"), installed("Episode 1.bin"))
    else {
        return;
    };
    let face = |id: i16| hrsp::decode(gfx.get(b"HrSp", id).unwrap()).unwrap();
    let player = Masks {
        body: masks::from_cicn(&gfx, 299).unwrap(),
        obstacle: masks::from_cicn(&gfx, 290).unwrap(),
        walls: masks::walls(&app).unwrap(),
        faces: (0..144).map(|f| (f >= 107).then(|| masks::from_face(&face(1101 + f as i16)))).collect(),
    };
    let plant = masks::from_cicn(&gfx, 599).unwrap();
    let barrel = masks::from_cicn(&gfx, 598).unwrap();
    let cooler = masks::from_face(&face(2002));
    let shot = masks::from_cicn(&gfx, 1299).unwrap();
    let ramps: Vec<_> = (571..=574).map(|id| masks::from_cicn(&gfx, id).unwrap()).collect();
    let bounds: HashMap<i16, [i16; 4]> =
        gfx.ids(b"HrSp").into_iter().map(|id| (id, hrsp::decode(gfx.get(b"HrSp", id).unwrap()).unwrap().bounds)).collect();
    let face_bounds = |f: i16| bounds.get(&f).copied().unwrap_or([0; 4]);
    let assets = Assets {
        player: &player,
        plant: &plant,
        barrel: &barrel,
        cooler: &cooler,
        shot: &shot,
        ramps: [&ramps[0], &ramps[1], &ramps[2], &ramps[3]],
        face_bounds: &face_bounds,
    };
    let level1 = hlvl::read(ep.get(b"Hlvl", 1).unwrap()).unwrap();
    let bindings = Bindings { left: 0x7B, right: 0x7C, scoot: 0x7D, inch: 0x7E, kick: 0x31, select_item: 0x3B, use_item: 0x3A };
    let mut bad = 0;
    let mut n = 0;
    let mut stale = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        // kicking a locked door reads stale stack values in the original
        // (BUGS.md): such runs cannot be compared
        if b.contains(" staledoor=1") {
            stale += 1;
            continue;
        }
        let b = b.replace(" staledoor=0", "");
        let b = b.as_str();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap();
        let mut g = PlayerGlobals {
            comfort: w("comfort") as i16,
            comfort_shown: w("comfort") as i16,
            favor: w("favor") as i16,
            favor_shown: w("favor") as i16,
            surface: 75,
            surface_shown: 75,
            cur_slot: -1,
            ..Default::default()
        };
        g.inventory = [(-1, 0); 12];
        // the data section's value (HandlePlayerSprite sets it each frame)
        g.tile.floor_kind = 1;
        let mut game = Game {
            world: World::default(),
            physics: level1.effective_physics(),
            level: level1.clone(),
            g,
            view: View::default(),
            rng: Random::new(w("seed") as u32),
            prefs: Prefs { scroll: w("scroll") as i16, even_v: false, blit: false, bindings },
            counter: LevelCounter { current: 1, previous: 0 },
            seen_level: 1,
            plant_flag: false,
            magnet: 0,
            gun: 0,
        };
        let missing = game.start(w("h") as i16, w("v") as i16, w("dir") as i16, &assets);
        assert!(missing.is_empty());
        let mut log = Vec::new();
        let mut harry = Vec::new();
        for keys in i["keys"].split(',') {
            let mut km = KeyMap::default();
            for c in keys.chars() {
                match c {
                    'l' => km.press(0x7B),
                    'r' => km.press(0x7C),
                    's' => km.press(0x7D),
                    'i' => km.press(0x7E),
                    'k' => km.press(0x31),
                    _ => {}
                }
            }
            let balloons = std::cell::RefCell::new(Vec::new());
            let mut show = |_: &harry_core::sprite::Sprite, _, _| {
                balloons.borrow_mut().push("balloon");
                false
            };
            let mut save = || {
                balloons.borrow_mut().push("save");
                false
            };
            let end = game.frame(&km, &assets, &mut show, &mut save);
            log.extend(balloons.into_inner());
            let p = game.world.sprites.list.iter().copied().find(|&r| game.world.rec(r).kind == 69).unwrap();
            let at = game.world.rec(p).at;
            harry.push(format!("{}:{}", at.0, at.1));
            if end == FrameEnd::LevelDone {
                log.push("uplevel");
                break;
            }
        }
        let gl = &game.g;
        let list: Vec<String> =
            game.world.sprites.list.iter().map(|&r| format!("{r}/{}", dump(game.world.rec(r)).replace(' ', ":"))).collect();
        let free: Vec<String> = game.world.sprites.free.iter().rev().map(|r| r.to_string()).collect();
        let got = format!(
            "frames={} seed={} view={}:{} target={}:{} meters={}:{}:{}:{}:{}:{} contact={} counter={} log={} harry={} list={} free={}",
            harry.len(),
            game.rng.seed,
            game.view.corner.0,
            game.view.corner.1,
            game.view.target.0,
            game.view.target.1,
            gl.comfort,
            gl.comfort_shown,
            gl.surface,
            gl.surface_shown,
            gl.favor,
            gl.favor_shown,
            gl.tile.kick_contact,
            game.counter.current,
            if log.is_empty() { "-".to_string() } else { log.join(",") },
            harry.join(","),
            list.join(";"),
            if free.is_empty() { "-".to_string() } else { free.join(",") }
        );
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 3 {
                let diffs: Vec<String> = b
                    .split([' ', ';', ',', ':'])
                    .zip(got.split([' ', ';', ',', ':']))
                    .enumerate()
                    .filter(|(_, (x, y))| x != y)
                    .take(6)
                    .map(|(k, (x, y))| format!("#{k} want {x} got {y}"))
                    .collect();
                eprintln!("{}\n  {}", &a[..a.len().min(200)], diffs.join("\n  "));
                let field = |s: &str, k: &str| s.split(' ').find(|x| x.starts_with(k)).unwrap_or("").to_string();
                let (wh, gh) = (field(b, "harry="), field(&got, "harry="));
                let first = wh.split(',').zip(gh.split(',')).position(|(x, y)| x != y);
                eprintln!("  first different frame: {first:?}");
                if let Some(f) = first {
                    let ws: Vec<&str> = wh.split(',').collect();
                    let gs: Vec<&str> = gh.split(',').collect();
                    let lo = f.saturating_sub(4);
                    eprintln!("  want {:?}\n  got  {:?}", &ws[lo..(f + 3).min(ws.len())], &gs[lo..(f + 3).min(gs.len())]);
                    let ks: Vec<&str> = i["keys"].split(',').collect();
                    eprintln!("  keys {:?}", &ks[lo..(f + 3).min(ks.len())]);
                }
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases ({stale} more kick a locked door)");
}
