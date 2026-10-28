//! Harry's frame (`HandlePlayerSprite`, keys stubbed) against the original,
//! recorded in Unicorn by `re/tools/emu_frame.py` on the real levels.

use std::collections::HashMap;
use std::path::PathBuf;

use harry_core::air::Air;
use harry_core::bounce::Body;
use harry_core::frame::{Harry, PlayerGlobals, handle_player};
use harry_core::physics::Speed;
use harry_core::player::Player;
use harry_core::random::Random;
use harry_core::{Physics, moves::Moves, player::TileState};
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn player_frame_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/frame.txt")) else {
        eprintln!("skipped: frame.txt not found (run re/tools/emu_frame.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Episode 1.bin")) else { return };
    let res = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let levels: Vec<_> = (1..=15).map(|i| hlvl::read(res.get(b"Hlvl", i).unwrap()).unwrap()).collect();
    let header = text.lines().next().unwrap();
    let (hin, hout) = header.split_once(": ").unwrap().1.split_once(" -> ").unwrap();
    let names_in: Vec<&str> = hin.split(' ').collect();
    let names_out: Vec<String> = hout.split(' ').map(|s| s.to_string()).collect();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once("->").unwrap();
        let vals: Vec<i64> = a.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let i: HashMap<&str, i64> = names_in.iter().copied().zip(vals).collect();
        let w = |k: &str| i[k] as i16;
        let want: Vec<i64> = b.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let mut inventory = [(-1i16, 0i16); 12];
        inventory[0].0 = w("inventory0");
        if i["curslot"] > 0 {
            inventory[i["curslot"] as usize].0 = w("inventorycur");
        }
        let mut g = PlayerGlobals {
            moves: Moves { scoot: w("scoot"), kick: w("kick"), inch: w("inch"), pose: w("pose"), impulse: w("impulse"), kick_power: w("power") },
            tile: TileState {
                wall_contact: i["wallcontact"] != 0,
                bg_contact: i["bgcontact"] != 0,
                ouch_cooldown: w("ouch"),
                comfort_delta: w("comfortdelta"),
                wet: w("wet"),
                floor_kind: w("floor"),
                kick_contact: w("contact"),
            },
            inv_switch_delay: w("invswitch"),
            fire_delay: w("fire"),
            t40: w("t40"),
            item_use_delay: w("itemuse"),
            recoil: w("recoil"),
            caffeine: w("caffeine"),
            cheat1: w("cheat1"),
            cheat2: w("cheat2"),
            wavy: w("wavy"),
            t28: w("t28"),
            t24: w("t24"),
            chatter: w("chatter"),
            shake: w("shake"),
            glide: w("glide"),
            flag5c: w("flag5c"),
            flag50: i["flag50"] != 0,
            kick_reset: i["kickreset"] != 0,
            comfort: w("comfort"),
            comfort_shown: w("comfortshown"),
            favor_shown: w("favorshown"),
            surface: w("surface"),
            cur_slot: w("curslot"),
            inventory,
            player_dir: 0x4444,
            player_pos: (0x4444, 0x4444),
            player_speed: (0x4444, 0x4444),
            ..Default::default()
        };
        let mut h = Harry {
            player: Player {
                body: Body {
                    pos: (w("posV"), w("posH")),
                    at: (w("atV"), w("atH")),
                    fix: (i["fixH"] as i32, i["fixV"] as i32),
                    speed: Speed { h: w("speedH"), v: w("speedV") },
                },
                dir: w("dir"),
                face: -999,
                air: w("air"),
                hot_rect: [0, 0, 0, 0],
            },
            air: Air { air: w("air"), slide: w("slide"), last_tile: (0, 0), flip: w("flip"), scale: w("scale0") },
            frozen: i["frozen"] != 0,
            dead: false,
            blast_cooldown: 0,
        };
        let level = &levels[i["level"] as usize - 1];
        let mut rng = Random::new(i["seed"] as u32);
        let mut sounds = Vec::new();
        let mut keys = 0;
        handle_player(&mut h, &mut g, level, &Physics::default(), &mut rng, |_, _, _| keys += 1, &mut sounds);
        let p = &h.player;
        let t = &g.tile;
        let m = &g.moves;
        let mut got: Vec<i64> = [
            m.scoot, m.kick, m.inch, g.inv_switch_delay, g.fire_delay, t.ouch_cooldown, g.t40, g.item_use_delay, g.recoil,
            g.caffeine, g.cheat1, g.cheat2, g.wavy, g.t28, g.t24, g.chatter, g.shake, m.pose, t.comfort_delta, t.wet,
            t.kick_contact, m.kick_power, g.glide, g.flag5c,
        ]
        .iter()
        .map(|&x| i64::from(x))
        .collect();
        got.extend([g.kick_reset, g.flag50, t.bg_contact, t.wall_contact].iter().map(|&b| i64::from(b)));
        got.extend(
            [
                m.impulse, g.comfort, g.comfort_shown, g.favor_shown, t.floor_kind, g.cur_slot, g.surface, g.player_dir,
                g.player_pos.0, g.player_pos.1, p.body.pos.0, p.body.pos.1, p.body.at.0, p.body.at.1, p.body.speed.v,
                p.body.speed.h, p.dir, p.air, h.air.slide, h.air.flip, h.air.scale, h.air.scale,
            ]
            .iter()
            .map(|&x| i64::from(x)),
        );
        got.extend([i64::from(p.body.fix.0), i64::from(p.body.fix.1), i64::from(p.face)]);
        got.extend(p.hot_rect.iter().map(|&x| i64::from(x)));
        got.extend([i64::from(h.dead), i64::from(g.player_speed.0), i64::from(g.player_speed.1), keys, sounds.len() as i64]);
        n += 1;
        if got != want {
            bad += 1;
            if bad <= 6 {
                let diffs: Vec<String> = names_out
                    .iter()
                    .flat_map(|s| if s == "hotRect(t" { vec!["hot.t".into()] } else { vec![s.clone()] })
                    .zip(got.iter().zip(&want))
                    .filter(|(_, (a, b))| a != b)
                    .map(|(name, (a, b))| format!("{name}: port {a} original {b}"))
                    .collect();
                eprintln!("frame case {n}: {}", diffs.join(", "));
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} frame cases differ");
}
