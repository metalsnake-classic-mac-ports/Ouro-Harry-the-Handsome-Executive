//! Harry touching other sprites (`HitPlayerSprite`) against the original,
//! recorded in Unicorn by `re/tools/emu_playerhit.py`.

use std::path::PathBuf;

use harry_core::air::Air;
use harry_core::bounce::Body;
use harry_core::frame::{Harry, PlayerGlobals};
use harry_core::keys::Ammo;
use harry_core::kinds::{setup_bonus, setup_item};
use harry_core::player::Player;
use harry_core::sprite::Sprite;
use harry_core::touch::{TouchEvent, touch};

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

#[test]
fn touching_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/playerhit.txt")) else {
        eprintln!("skipped: playerhit.txt not found (run re/tools/emu_playerhit.py)");
        return;
    };
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let f: Vec<&str> = a.split(' ').collect();
        let w = |k: usize| f[k].parse::<i16>().unwrap();
        let kind = w(0);
        let mut h = Harry {
            player: Player { body: Body::default(), dir: 0, face: 0, air: w(1), hot_rect: [0; 4] },
            air: Air { air: w(1), slide: 0, last_tile: (0, 0), flip: 0, scale: 0x100 },
            frozen: w(2) != 0,
            dead: false,
            blast_cooldown: 0,
        };
        let mut g = PlayerGlobals {
            comfort: w(4),
            t24: w(5),
            ammo: Ammo { staple: w(6), badass: w(7), shrapnel: w(8) },
            cur_slot: w(9),
            inventory: inventory(f[10]),
            ..Default::default()
        };
        let mut other = Sprite::new(kind, 0, 0);
        if kind < 800 { setup_bonus(&mut other) } else { setup_item(&mut other) }
        other.frozen = w(3) != 0;
        let mut events = Vec::new();
        touch(&mut h, &mut g, &mut other, w(11) != 0, &mut events);
        let ev: Vec<String> = events
            .iter()
            .map(|e| match *e {
                TouchEvent::Sound(s) => format!("p:{s}"),
                TouchEvent::VoiceSound(s) => format!("v:{s}"),
                TouchEvent::HaltTalking => "halt".into(),
            })
            .collect();
        let inv: Vec<String> = g.inventory.iter().map(|(a, b)| format!("{a}:{b}")).collect();
        let got = format!(
            "{} {} {} {} {} {} {} {} {}",
            u8::from(other.frozen),
            g.comfort,
            g.t24,
            g.ammo.staple,
            g.ammo.badass,
            g.ammo.shrapnel,
            g.cur_slot,
            inv.join(","),
            if ev.is_empty() { "-".into() } else { ev.join(",") }
        );
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 10 {
                eprintln!("{a}\n  want {b}\n  got  {got}");
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
