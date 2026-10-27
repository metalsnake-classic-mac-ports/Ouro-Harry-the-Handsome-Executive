//! `HandleKeys` with the inventory routines against the original,
//! recorded in Unicorn by `re/tools/emu_keys.py`.

use std::collections::HashMap;
use std::path::PathBuf;

use harry_core::air::Air;
use harry_core::bounce::Body;
use harry_core::frame::{Harry, PlayerGlobals};
use harry_core::keys::{Ammo, Bindings, KeyEvent, KeyInput, KeyMap, handle_keys};
use harry_core::moves::Moves;
use harry_core::physics::Speed;
use harry_core::player::{Player, TileState};
use harry_core::random::Random;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn list(s: &str) -> Vec<i16> {
    if s == "-" { Vec::new() } else { s.split(',').map(|x| x.parse().unwrap()).collect() }
}

fn inventory(s: &str) -> [(i16, i16); 12] {
    let mut inv = [(0, 0); 12];
    for (slot, e) in inv.iter_mut().zip(s.split(',')) {
        let (a, b) = e.split_once(':').unwrap();
        *slot = (a.parse().unwrap(), b.parse().unwrap());
    }
    inv
}

/// The events in the recorder's notation; a ripple shows as the
/// `BuildSineTable` call it makes (when turned on).
fn notation(e: &KeyEvent) -> Option<String> {
    Some(match *e {
        KeyEvent::Sound(s) => format!("p:{s}"),
        KeyEvent::ChargeSound(s) => format!("c:{s}"),
        KeyEvent::VoiceSound(s) => format!("v:{s}"),
        KeyEvent::StopCharge => "stop".into(),
        KeyEvent::Sound3D { snd, pitch } => format!("3d:{snd}:{pitch}"),
        KeyEvent::Explosion { h, v, flag } => format!("x:{h}:{v}:{flag}"),
        KeyEvent::Balloon(n) => format!("b:{n}"),
        KeyEvent::Shot { h, v, dir, speed } => format!("shot:{h}:{v}:{dir}:{}:{}", speed.v, speed.h),
        KeyEvent::Ripple(true) => "sine".into(),
        KeyEvent::Ripple(false) => return None,
    })
}

#[test]
fn keys_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/keys.txt")) else {
        eprintln!("skipped: keys.txt not found (run re/tools/emu_keys.py)");
        return;
    };
    let header = text.lines().next().unwrap();
    let (hin, hout) = header.split_once(": ").unwrap().1.split_once(" -> ").unwrap();
    let names_in: Vec<&str> = hin.split(' ').collect();
    let names_out: Vec<&str> = hout.split(' ').collect();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = names_in.iter().copied().zip(a.split(' ')).collect();
        let w = |k: &str| i[k].parse::<i64>().unwrap() as i16;
        let mut g = PlayerGlobals {
            moves: Moves { scoot: w("scoot"), kick: w("kick"), inch: w("inch"), ..Default::default() },
            tile: TileState { comfort_delta: w("comfortdelta"), ..Default::default() },
            inv_switch_delay: w("invswitch"),
            fire_delay: w("fire"),
            item_use_delay: w("itemuse"),
            recoil: w("recoil"),
            caffeine: w("caffeine"),
            cheat1: w("cheat1"),
            cheat2: w("cheat2"),
            wavy: w("wavy"),
            chatter: w("chatter"),
            comfort: w("comfort"),
            cur_slot: w("curslot"),
            inventory: inventory(i["inv"]),
            ammo: Ammo { staple: w("staple"), badass: w("badass"), shrapnel: w("shrapnel") },
            fire_hold: w("hold"),
            shot_kind: w("shotkind"),
            cheat1_step: w("step1"),
            cheat2_step: w("step2"),
            wavy_step: w("wavystep"),
            wavy_on: w("wavyon") != 0,
            ..Default::default()
        };
        let mut h = Harry {
            player: Player {
                body: Body { pos: (w("posV"), w("posH")), at: (w("atV"), w("atH")), fix: (0, 0), speed: Speed::default() },
                dir: w("dir"),
                face: 0,
                air: w("air"),
                hot_rect: [0; 4],
            },
            air: Air { air: w("air"), slide: 0, last_tile: (0, 0), flip: 0, scale: w("scale0") },
            frozen: false,
            dead: false,
            blast_cooldown: w("blast"),
        };
        let kb = list(i["bindings"]);
        let bindings = Bindings {
            left: kb[0],
            right: kb[1],
            scoot: kb[2],
            inch: kb[3],
            kick: kb[4],
            select_item: kb[5],
            use_item: kb[6],
        };
        let mut keys = KeyMap::default();
        for c in list(i["keys"]) {
            keys.press(c);
        }
        let input = KeyInput {
            keys: &keys,
            bindings: &bindings,
            debug: w("debug") != 0,
            gulp_playing: w("gulpbusy") != 0,
            voice_playing: w("voicebusy") != 0,
        };
        let mut rng = Random::new(i["seed"].parse().unwrap());
        let mut events = Vec::new();
        handle_keys(&mut h, &mut g, &input, &mut rng, &mut events);
        // the ripple fields and phase as the toggle leaves them
        let mut ripple = [w("ripple0"), w("ripple1"), w("ripple2"), w("ripplephase")];
        for e in &events {
            if let KeyEvent::Ripple(on) = *e {
                ripple = if on { [100, 16, 0x300, 0] } else { [0, 0, 0, 0] };
            }
        }
        let p = &h.player;
        let m = &g.moves;
        let mut got: Vec<String> = [
            m.scoot, m.kick, m.inch, g.inv_switch_delay, g.fire_delay, g.item_use_delay, g.recoil, g.caffeine, g.cheat1,
            g.cheat2, g.wavy, g.tile.comfort_delta, g.ammo.staple, g.ammo.badass, g.ammo.shrapnel, g.fire_hold,
            g.cheat1_step, g.cheat2_step, g.wavy_step, ripple[3], g.comfort, g.shot_kind, g.cur_slot, p.body.pos.0,
            p.body.pos.1, p.body.at.0, p.body.at.1, p.dir, p.air, h.air.scale, h.air.scale, h.blast_cooldown,
            i16::from(g.wavy_on), ripple[0], ripple[1], ripple[2],
        ]
        .iter()
        .map(|x| x.to_string())
        .collect();
        got.push(g.inventory.iter().map(|(a, b)| format!("{a}:{b}")).collect::<Vec<_>>().join(","));
        let ev: Vec<String> = events.iter().filter_map(notation).collect();
        got.push(if ev.is_empty() { "-".into() } else { ev.join(",") });
        let want: Vec<&str> = b.split(' ').collect();
        n += 1;
        let diffs: Vec<String> = names_out
            .iter()
            .zip(want.iter().zip(&got))
            .filter(|(_, (w, g))| *w != g)
            .map(|(name, (w, g))| format!("{name}: want {w} got {g}"))
            .collect();
        if !diffs.is_empty() {
            bad += 1;
            if bad <= 10 {
                eprintln!("{a}\n  {}", diffs.join("\n  "));
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
