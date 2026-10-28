//! The memo and phone sentences against the original, recorded in Unicorn
//! by `re/tools/emu_words.py`.

use std::path::PathBuf;

use harry_core::random::Random;
use harry_original::words::words;
use harry_original::resfork::{MacBinary, Resources};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn sentences_match_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/words.txt")) else {
        eprintln!("skipped: words.txt not found (run re/tools/emu_words.py)");
        return;
    };
    let Ok(bytes) = std::fs::read(root().join("re/work/installed/Harry 1.0.1.bin")) else { return };
    let app = Resources::parse(&MacBinary::parse(&bytes).unwrap().rsrc).unwrap();
    let base = words(&app).unwrap();
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let f: Vec<&str> = a.split(' ').map(|kv| kv.split_once('=').unwrap().1).collect();
        let mut rng = Random::new(f[0].parse().unwrap());
        let mut w = base.clone();
        for (k, v) in f[1].split(':').enumerate() {
            w.last[k] = v.parse().unwrap();
        }
        let texts: Vec<String> = f[2]
            .chars()
            .map(|c| {
                let t = if c == 'b' { w.buzzwords(&mut rng) } else { w.phonewords(&mut rng) };
                if t.is_empty() { "-".into() } else { t.iter().map(|x| format!("{x:02x}")).collect() }
            })
            .collect();
        let last: Vec<String> = w.last.iter().map(|x| x.to_string()).collect();
        let got = format!("{} seed={} last={}", texts.join(","), rng.seed, last.join(":"));
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 5 {
                eprintln!("{a}\n  want {b}\n  got  {got}");
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
