//! Whole frames of the levels of Episode 1 against the original, recorded in Unicorn by
//! `re/tools/emu_game.py`.
//!
//! Debugging: `GAME_TRIAGE=1` prints, per differing case, the first list
//! entry that differs; `GAME_DUMP=<seed>` prints that case's frames as
//! `emu_game.py` does with `SEED=<seed>` (and `KIND=<kind>` its sprites of
//! that kind, `SOUNDS=1` the sounds), so the two can be diffed.

mod common;

use std::collections::HashMap;

use common::{Fields, crc32, dump, installed, levels_of, recording};
use harry_core::frame::PlayerGlobals;
use harry_core::savegame::SaveGame;
use harry_core::sound::SoundEvent;
use harry_core::game::{Assets, FrameEnd, Game, LevelStart, Prefs, UpLevel};
use harry_core::keys::{Bindings, KeyMap};
use harry_core::kinds::LevelCounter;
use harry_core::player::Masks;
use harry_core::random::Random;
use harry_core::view::View;
use harry_core::world::World;
use harry_original::{hrsp, masks};

/// The saved game a run continues from, as `emu_game.py`'s `record`
/// builds it.
fn record(i: &Fields, level: &harry_core::Level) -> SaveGame {
    let w = |k: &str| i.i64(k);
    let mut s = SaveGame {
        world: harry_core::savegame::DEFAULT_WORLD,
        copier: w("cont") == 1,
        level: w("level") as i16,
        score: w("score") as i32,
        comfort: w("comfort") as i16,
        surface: w("surface") as i16,
        favor: w("favor") as i16,
        pos: (w("h") as i16, w("v") as i16),
        dir: w("dir") as i16,
        ammo: harry_core::keys::Ammo { staple: w("staple") as i16, badass: w("badass") as i16, shrapnel: w("shrapnel") as i16 },
        entries: level.sprites.clone(),
        custom: level.rest[harry_core::savegame::CUSTOM].to_vec(),
        world_name: b"Episode 1".to_vec(),
        ..SaveGame::default()
    };
    s.inventory = [(-1, 0); 12];
    if i.str("inv") != "-" {
        for (slot, e) in s.inventory.iter_mut().zip(i.str("inv").split(',')) {
            let (a, b) = e.split_once(':').unwrap();
            *slot = (a.parse().unwrap(), b.parse().unwrap());
        }
    }
    if i.str("seen") != "-" {
        for spot in i.str("seen").split(',') {
            let (th, tv) = spot.split_once(':').unwrap();
            let (th, tv): (usize, usize) = (th.parse().unwrap(), tv.parse().unwrap());
            for x in th.saturating_sub(7)..(th + 7).min(128) {
                for y in tv.saturating_sub(7)..(tv + 7).min(128) {
                    s.track[x * 128 + y] = 1;
                }
            }
        }
    }
    if i.str("gone") != "-" {
        for k in i.str("gone").split(',') {
            s.entries[k.parse::<usize>().unwrap()].present = false;
        }
    }
    s
}

#[test]
fn episode_1_frames_match_the_original() {
    let Some(text) = recording("game", "emu_game.py") else { return };
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
    let player_shot = masks::from_face(&face(4021));
    let robot = masks::from_face(&face(4251));
    let blob = masks::from_face(&face(1451));
    let desks = [masks::from_face(&face(6021)), masks::from_face(&face(6022))];
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
        player_shot: &player_shot,
        robot: &robot,
        blob: &blob,
        desks: [&desks[0], &desks[1]],
        face_bounds: &face_bounds,
    };
    let levels = levels_of(&ep);
    let words = harry_original::words::words(&app).unwrap();
    let bindings = Bindings { left: 0x7B, right: 0x7C, scoot: 0x7D, inch: 0x7E, kick: 0x31, select_item: 0x3B, use_item: 0x3A };
    let mut bad = 0;
    let mut n = 0;
    let mut stale = 0;
    // runs whose sounds differ (counted apart: GAME_TRIAGE names them)
    let mut sound_bad = 0;
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
        let i = Fields::parse(a);
        let w = |k: &str| i.i64(k);
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
        if i.str("inv") != "-" {
            for (slot, e) in g.inventory.iter_mut().zip(i.str("inv").split(',')) {
                let (a, b) = e.split_once(':').unwrap();
                *slot = (a.parse().unwrap(), b.parse().unwrap());
            }
        }
        g.cur_slot = w("curslot") as i16;
        g.ammo.staple = w("staple") as i16;
        g.ammo.badass = w("badass") as i16;
        g.ammo.shrapnel = w("shrapnel") as i16;
        // the data section's value (HandlePlayerSprite sets it each frame)
        g.tile.floor_kind = 1;
        let number = w("level") as i16;
        let level = &levels[number as usize - 1];
        let mut game = Game {
            world: World::default(),
            physics: level.effective_physics(),
            level: level.clone(),
            g,
            view: View::default(),
            rng: Random::new(w("seed") as u32),
            prefs: Prefs { scroll: w("scroll") as i16, even_v: false, blit: false, bindings, autosave: false },
            counter: LevelCounter { current: number, previous: 0 },
            seen_level: number,
            plant_flag: false,
            robot_flags: (false, false),
            reversor_flags: (false, false),
            blob_flags: (false, false),
            magnet: 2,
            gun: 0,
            harry: None,
            words: words.clone(),
            sounds: Vec::new(),
            // the recorder stubs the mixer: nothing ever plays (the sounds
            // are compared where they reach it)
            audio: harry_core::sound::Audio { silent: true, ..Default::default() },
            level_start: LevelStart::default(),
            track: Default::default(),
            keep_track: false,
            blits: Vec::new(),
            meters: Default::default(),
            session: Default::default(),
        };
        let missing = if w("cont") >= 0 {
            let save = record(&i, level);
            game.continue_game(&save, &levels, &assets).unwrap()
        } else {
            game.start(w("h") as i16, w("v") as i16, w("dir") as i16, &assets)
        };
        assert!(missing.is_empty(), "level {number}: {missing:?}");
        let debug = std::env::var("GAME_DUMP").is_ok_and(|s| s == i.str("seed"));
        let kind = std::env::var("KIND").ok();
        if debug {
            eprintln!("start {} rand {}", dump(game.world.rec(game.harry.unwrap())), game.rng.seed);
        }
        let mut log = Vec::new();
        let mut harry = Vec::new();
        let (mut nblits, mut blit_crc) = (0, 0);
        let (mut nsounds, mut sound_crc) = (0, 0);
        let mut answers: Vec<bool> = i.str("balloons").chars().rev().map(|c| c == '1').collect();
        let mut copier: Vec<bool> = i.str("copier").chars().rev().map(|c| c == '1').collect();
        game.prefs.autosave = w("autosave") != 0;
        for keys in i.str("keys").split(',') {
            let mut km = KeyMap::default();
            for c in keys.chars() {
                match c {
                    'l' => km.press(0x7B),
                    'r' => km.press(0x7C),
                    's' => km.press(0x7D),
                    'i' => km.press(0x7E),
                    'k' => km.press(0x31),
                    'e' => km.press(0x3B),
                    'u' => km.press(0x3A),
                    '1' => km.press(0x12),
                    '2' => km.press(0x13),
                    '3' => km.press(0x14),
                    '4' => km.press(0x15),
                    'W' => km.press(0x0D),
                    'A' => km.press(0x00),
                    'V' => km.press(0x09),
                    'Y' => km.press(0x10),
                    'G' => km.press(0x05),
                    'R' => km.press(0x0F),
                    'P' => km.press(0x23),
                    'H' => km.press(0x04),
                    'Z' => km.press(0x06),
                    'B' => km.press(0x0B),
                    'E' => km.press(0x0E),
                    'N' => km.press(0x2D),
                    _ => {}
                }
            }
            let balloons = std::cell::RefCell::new(Vec::new());
            let mut show = |_: &harry_core::game::Balloon| {
                let ans = answers.pop().unwrap_or(false);
                balloons.borrow_mut().push(if ans { "balloon1" } else { "balloon0" });
                ans
            };
            let mut save = || {
                let ans = copier.pop().unwrap_or(false);
                balloons.borrow_mut().push(if ans { "save1" } else { "save0" });
                ans
            };
            let end = game.frame(&km, &assets, &mut show, &mut save);
            log.extend(balloons.into_inner());
            let mut b: String = game.blits.iter().map(|(_, b)| format!("{}:{}:{};", b.face, b.mode, b.arg)).collect();
            if debug && std::env::var("BLITS").is_ok() {
                eprintln!("  blits {b}");
            }
            nblits += game.blits.len();
            b.push('|');
            blit_crc = crc32(blit_crc, b.as_bytes());
            // CheckGameEvents calls UpLevel at the end of the frame
            let won = end == FrameEnd::LevelDone && {
                log.push("uplevel");
                let (up, missing) = game.up_level(&levels, &assets);
                assert!(missing.is_empty(), "level {}: {missing:?}", game.counter.current);
                up == UpLevel::Won
            };
            // the sounds that reached the mixer this frame (UpLevel's too)
            let mut s = String::new();
            for e in &game.sounds {
                if let SoundEvent::Play { id, left, right, rate, .. } = e {
                    s += &format!("{id}:{left}:{right}:{rate};");
                    nsounds += 1;
                }
            }
            if debug && std::env::var("SOUNDS").is_ok() {
                eprintln!("  sounds {s}");
            }
            s.push('|');
            sound_crc = crc32(sound_crc, s.as_bytes());
            // his record, as the recorder reads it (after the death he may have left the list)
            let p = game.harry.unwrap();
            let at = game.world.rec(p).at;
            harry.push(format!("{}:{}", at.0, at.1));
            if debug {
                let kinds: Vec<String> = game.world.sprites.list.iter().map(|&r| game.world.rec(r).kind.to_string()).collect();
                eprintln!("{} {keys} {} comfort {} rand {} log {log:?} list {}", harry.len() - 1, dump(game.world.rec(p)), game.g.comfort, game.rng.seed, kinds.join(","));
                for &r in &game.world.sprites.list {
                    if kind.as_deref() == Some(game.world.rec(r).kind.to_string().as_str()) {
                        eprintln!("    {}", dump(game.world.rec(r)));
                    }
                }
            }
            if won {
                log.push("won");
                break;
            }
            if end == FrameEnd::Died {
                log.push("died");
                break;
            }
        }
        let gl = &game.g;
        let list: Vec<String> =
            game.world.sprites.list.iter().map(|&r| format!("{r}/{}", dump(game.world.rec(r)).replace(' ', ":"))).collect();
        let free: Vec<String> = game.world.sprites.free.iter().rev().map(|r| r.to_string()).collect();
        let got = format!(
            "frames={} seed={} view={}:{} target={}:{} meters={}:{}:{}:{}:{}:{} contact={} counter={} log={} inv={} curslot={} ammo={}:{}:{} score={} start={} track={} save={} ripple={} blits={}:{} sounds={}:{} meterdraw={} harry={} list={} free={}",
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
            gl.inventory.iter().map(|(a, b)| format!("{a}:{b}")).collect::<Vec<_>>().join(","),
            gl.cur_slot,
            gl.ammo.staple,
            gl.ammo.badass,
            gl.ammo.shrapnel,
            gl.score,
            {
                let st = &game.level_start;
                let mut f = vec![st.comfort.to_string(), st.surface.to_string(), st.favor.to_string(), st.score.to_string()];
                f.extend([st.ammo.staple, st.ammo.badass, st.ammo.shrapnel].map(|a| a.to_string()));
                f.extend(st.inventory.iter().map(|(a, b)| format!("{a}/{b}")));
                f.join(":")
            },
            {
                let t = &game.track;
                let seen: Vec<usize> = (0..t.cells.len()).filter(|&i| t.cells[i] != 0).collect();
                format!("{}:{}:{}:{}", seen.len(), seen.iter().map(|i| i + 1).sum::<usize>(), t.last.0, t.last.1)
            },
            {
                let sv = &game.session.save;
                let d = sv.to_bytes();
                let sum = d.iter().enumerate().fold(0u32, |a, (i, &b)| a.wrapping_add((i as u32 + 1).wrapping_mul(u32::from(b))));
                format!("{}:{}:{}:{sum}", u8::from(sv.copier), sv.level, sv.score)
            },
            {
                let info = &game.level.info;
                let table = harry_core::ripple::sine_table(&mut game.level.clone());
                let sum: i64 = table.iter().enumerate().map(|(i, &x)| (i as i64 + 1) * i64::from(x)).sum();
                format!("{}:{}:{}:{}:{sum}", game.session.ripple_phase, info[10], info[11], info[12])
            },
            nblits,
            blit_crc,
            nsounds,
            sound_crc,
            game.meters.iter().map(|d| format!("{}:{}", d.len, u8::from(d.end))).collect::<Vec<_>>().join(","),
            harry.join(","),
            list.join(";"),
            if free.is_empty() { "-".to_string() } else { free.join(",") }
        );
        n += 1;
        let sounds = |s: &str| s.split(' ').find(|x| x.starts_with("sounds=")).unwrap_or("").to_string();
        let (want_sounds, got_sounds) = (sounds(b), sounds(&got));
        if want_sounds != got_sounds {
            sound_bad += 1;
            if std::env::var("GAME_TRIAGE").is_ok() {
                eprintln!("SOUNDS level={} seed={} {want_sounds}/{got_sounds}", i.str("level"), i.str("seed"));
            }
        }
        let rest_want = b.replace(&format!(" {want_sounds}"), "");
        let b = rest_want.as_str();
        let got = got.replace(&format!(" {got_sounds}"), "");
        if debug {
            eprintln!("WANT {b}\nGOT  {got}");
        }
        if got != b && std::env::var("GAME_TRIAGE").is_ok() {
            // the first list entry that differs: its kind and fields
            let list = |s: &str| s.split(' ').find(|x| x.starts_with("list=")).unwrap_or("").to_string();
            let (wl, gl) = (list(b), list(&got));
            let we: Vec<&str> = wl.trim_start_matches("list=").split(';').collect();
            let ge: Vec<&str> = gl.trim_start_matches("list=").split(';').collect();
            let other = |s: &str| s.split(' ').filter(|x| !x.starts_with("list=") && !x.starts_with("harry=")).collect::<Vec<_>>().join(" ");
            let first = we.iter().zip(ge.iter()).position(|(x, y)| x != y);
            let fields = first.map(|k| {
                let wf: Vec<&str> = we[k].split(':').collect();
                let gf: Vec<&str> = ge[k].split(':').collect();
                let d: Vec<String> = wf.iter().zip(gf.iter()).filter(|(x, y)| x != y).map(|(x, y)| format!("{x}/{y}")).collect();
                format!("{} {}", wf[0], d.join(","))
            });
            let o: Vec<String> = other(b).split(' ').zip(other(&got).split(' ')).filter(|(x, y)| x != y).map(|(x, y)| format!("{x}/{y}")).collect();
            eprintln!("TRIAGE level={} seed={} len={}/{} {:?} {}", i.str("level"), i.str("seed"), we.len(), ge.len(), fields, o.join(" "));
        }
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
                    let ks: Vec<&str> = i.str("keys").split(',').collect();
                    eprintln!("  keys {:?}", &ks[lo..(f + 3).min(ks.len())]);
                }
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    assert_eq!(sound_bad, 0, "the sounds of {sound_bad} of {n} cases differ");
    eprintln!("{n} cases ({stale} more kick a locked door), sounds included");
}
