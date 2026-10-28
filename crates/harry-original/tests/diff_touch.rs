//! Harry touching other sprites (`HitPlayerSprite`) against the original,
//! recorded in Unicorn by `re/tools/emu_playerhit.py`.

use std::collections::HashMap;
use std::path::PathBuf;

use harry_core::Physics;
use harry_core::air::Air;
use harry_core::bounce::Body;
use harry_core::frame::{Harry, PlayerGlobals};
use harry_core::keys::Ammo;
use harry_core::collider::setup_collider;
use harry_core::dart_enemy::setup_dart_enemy;
use harry_core::ramp::setup_ramp;
use harry_core::kinds::{setup_bonus, setup_door, setup_item};
use harry_core::physics::Speed;
use harry_core::player::{HOT_RECT, KICK_HOT_RECT, Player, TileState};
use harry_core::random::Random;
use harry_core::sprite::Sprite;
use harry_core::touch::{TouchEvent, TouchWorld, touch};
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn inventory(s: &str) -> [(i16, i16); 12] {
    let mut inv = [(0, 0); 12];
    for (slot, e) in inv.iter_mut().zip(s.split(',')) {
        let (a, b) = e.split_once(':').unwrap();
        *slot = (a.parse().unwrap(), b.parse().unwrap());
    }
    inv
}

fn fields(s: &str) -> HashMap<&str, &str> {
    s.split(' ').map(|kv| kv.split_once('=').unwrap()).collect()
}

#[test]
fn touching_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/playerhit.txt")) else {
        eprintln!("skipped: playerhit.txt not found (run re/tools/emu_playerhit.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Episode 1.bin")) else { return };
    let res = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let mut level = hlvl::read(res.get(b"Hlvl", 1).unwrap()).unwrap();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i = fields(a);
        let w = |k: &str| i[k].parse::<i32>().unwrap() as i16;
        let kind = w("kind");
        let mut h = Harry {
            player: Player {
                body: Body {
                    pos: (w("pv"), w("ph")),
                    at: (w("pav"), w("pah")),
                    fix: (i["fixh"].parse().unwrap(), i["fixv"].parse().unwrap()),
                    speed: Speed { h: w("speedh"), v: w("speedv") },
                },
                dir: w("dir"),
                face: 0,
                air: w("air"),
                hot_rect: if w("kicking") != 0 { KICK_HOT_RECT } else { HOT_RECT },
            },
            air: Air { air: w("air"), slide: 0, last_tile: (0, 0), flip: 0, scale: 0x100 },
            frozen: w("pfrozen") != 0,
            dead: false,
            blast_cooldown: 0,
            ramp_cooldown: w("pf94"),
        };
        let mut g = PlayerGlobals {
            comfort: w("comfort"),
            t24: w("t24"),
            t28: w("t28"),
            ammo: Ammo { staple: w("staple"), badass: w("badass"), shrapnel: w("shrapnel") },
            cur_slot: w("curslot"),
            inventory: inventory(i["inv"]),
            tile: TileState { kick_contact: w("contact"), ..Default::default() },
            kick_reset: w("kickreset") != 0,
            player_pos: (w("playerh"), w("playerv")),
            flag50: w("bounced") != 0,
            score: i["score"].parse().unwrap(),
            ..Default::default()
        };
        let mut other = Sprite::new(kind, w("oh"), w("ov"));
        match kind {
            ..800 => setup_bonus(&mut other),
            900..1000 => setup_door(&mut other),
            1000..1050 => setup_dart_enemy(&mut other),
            9700..9800 => setup_ramp(&mut other),
            11000.. => setup_item(&mut other),
            _ => setup_collider(&mut other, &mut Random::new(1)),
        }
        other.timer = w("otimer");
        let li = w("levelindex");
        other.level_index = li;
        level.sprites[li as usize].id = kind;
        level.sprites[li as usize].info[0] = w("info0");
        other.frozen = w("ofrozen") != 0;
        let physics = Physics { bounce: w("bounce"), ..Physics::default() };
        let mut world = TouchWorld { level: &mut level, physics: &physics, voice_playing: w("voicebusy") != 0 };
        let mut events = Vec::new();
        touch(&mut h, &mut g, &mut other, &mut world, &mut events);
        let ev: Vec<String> = events
            .iter()
            .map(|e| match *e {
                TouchEvent::Sound(s) => format!("p:{s}"),
                TouchEvent::VoiceSound(s) => format!("v:{s}"),
                TouchEvent::Remark(s) => format!("r:{s}"),
                TouchEvent::HaltTalking => "halt".into(),
            })
            .collect();
        let inv: Vec<String> = g.inventory.iter().map(|(a, b)| format!("{a}:{b}")).collect();
        let p = &h.player;
        let r = p.hot_rect;
        let got = [
            ("ofrozen", u8::from(other.frozen).to_string()),
            ("comfort", g.comfort.to_string()),
            ("t24", g.t24.to_string()),
            ("t28", g.t28.to_string()),
            ("staple", g.ammo.staple.to_string()),
            ("badass", g.ammo.badass.to_string()),
            ("shrapnel", g.ammo.shrapnel.to_string()),
            ("contact", g.tile.kick_contact.to_string()),
            ("kickreset", u8::from(g.kick_reset).to_string()),
            ("curslot", g.cur_slot.to_string()),
            ("inv", inv.join(",")),
            ("pv", p.body.pos.0.to_string()),
            ("ph", p.body.pos.1.to_string()),
            ("fixh", p.body.fix.0.to_string()),
            ("fixv", p.body.fix.1.to_string()),
            ("pav", p.body.at.0.to_string()),
            ("pah", p.body.at.1.to_string()),
            ("bounced", u8::from(g.flag50).to_string()),
            ("score", g.score.to_string()),
            ("otimer", other.timer.to_string()),
            ("pf94", h.ramp_cooldown.to_string()),
            ("pair", h.player.air.to_string()),
            ("ospeedv", other.speed.v.to_string()),
            ("ospeedh", other.speed.h.to_string()),
            ("opv", other.pos.0.to_string()),
            ("oph", other.pos.1.to_string()),
            ("speedv", p.body.speed.v.to_string()),
            ("speedh", p.body.speed.h.to_string()),
            ("hot", format!("{}:{}:{}:{}", r[0], r[1], r[2], r[3])),
            ("okind", other.kind.to_string()),
            ("odir", other.dir.to_string()),
            ("ob8a", u8::from(other.b8a).to_string()),
            ("record", level.sprites[li as usize].id.to_string()),
            ("events", if ev.is_empty() { "-".into() } else { ev.join(",") }),
        ]
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(" ");
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 10 {
                let diffs: Vec<String> =
                    b.split(' ').zip(got.split(' ')).filter(|(x, y)| x != y).map(|(x, y)| format!("want {x} got {y}")).collect();
                eprintln!("{a}\n  {}", diffs.join(", "));
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
