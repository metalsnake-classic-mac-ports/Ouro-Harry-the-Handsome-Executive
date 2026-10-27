//! SAT's sprite list handling (`HandleSprites`) against the original,
//! recorded in Unicorn by `re/tools/emu_sat.py` with dummy sprites.

use std::path::PathBuf;

use harry_core::sat::{absolute, collisions, sort_pass};
use harry_core::tiles::sect;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Dummy {
    kind: i16,
    layer: i16,
    pos: (i16, i16),
    at: (i16, i16),
    hot: [i16; 4],
    task: i64,
    hit: bool,
    act: i64,
}

fn join(v: &[String]) -> String {
    if v.is_empty() { "-".into() } else { v.join(",") }
}

#[test]
fn sprite_list_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/sat.txt")) else {
        eprintln!("skipped: sat.txt not found (run re/tools/emu_sat.py)");
        return;
    };
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let mut s: Vec<Dummy> = a
            .split(' ')
            .nth(1)
            .unwrap()
            .split(',')
            .enumerate()
            .map(|(i, d)| {
                let f: Vec<i64> = d.split(':').map(|x| x.parse().unwrap()).collect();
                let w = |k: usize| f[k] as i16;
                Dummy {
                    kind: 100 + i as i16,
                    layer: w(0),
                    pos: (w(1), w(2)),
                    at: (w(3), w(4)),
                    hot: [w(5), w(6), w(7), w(8)],
                    task: f[9],
                    hit: f[10] != 0,
                    act: f[11],
                }
            })
            .collect();
        let mut order: Vec<usize> = (0..s.len()).collect();
        // SATRun: tasks, then each sprite's absolute hot rect
        let mut tasks = Vec::new();
        let mut rect2 = vec![[0i16; 4]; s.len()];
        for &i in &order {
            if s[i].task != 0 {
                tasks.push(i.to_string());
                match s[i].act {
                    1 => s[i].task = 0,
                    2 => s[i].pos.0 += 40,
                    _ => {}
                }
            }
            rect2[i] = absolute(s[i].hot, s[i].pos);
        }
        sort_pass(&mut order, |i| s[i].layer);
        let mut hits = Vec::new();
        collisions(&order, |i| s[i].layer, |i| rect2[i], |x, y| {
            if s[x].hit {
                hits.push(format!("{x}>{y}"));
            }
            if s[y].hit {
                hits.push(format!("{y}>{x}"));
            }
        });
        for d in s.iter_mut().filter(|d| d.task == 0) {
            d.kind = 0;
            d.hit = false;
        }
        // CheckPlayerCollision
        let mut phits = Vec::new();
        if let Some(&p) = order.iter().find(|&&i| s[i].task == 2) {
            let me = absolute(s[p].hot, s[p].at);
            for &o in &order {
                if s[o].task == 1 && sect(me, absolute(s[o].hot, s[o].at)) {
                    phits.push(o.to_string());
                }
            }
        }
        let got = format!(
            "{} {} {} {} {}",
            join(&tasks),
            join(&order.iter().map(|x| x.to_string()).collect::<Vec<_>>()),
            join(&hits),
            join(&phits),
            join(&s.iter().map(|d| d.kind.to_string()).collect::<Vec<_>>())
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
