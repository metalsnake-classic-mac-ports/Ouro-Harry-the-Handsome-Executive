//! The view (`FindUpperLeftCorner`) against the original, recorded in
//! Unicorn by `re/tools/emu_view.py`.

use std::collections::HashMap;
use std::path::PathBuf;

use harry_core::view::{View, find_upper_left_corner};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn view_matches_the_original() {
    let Ok(text) = std::fs::read_to_string(root().join("re/work/diff/view.txt")) else {
        eprintln!("skipped: view.txt not found (run re/tools/emu_view.py)");
        return;
    };
    let mut bad = 0;
    let mut n = 0;
    for line in text.lines().skip(1) {
        let (a, b) = line.split_once(" -> ").unwrap();
        let i: HashMap<&str, &str> = a.split(' ').map(|kv| kv.split_once('=').unwrap()).collect();
        let w = |k: &str| i[k].parse::<i16>().unwrap();
        let mut view = View {
            corner: (w("vv"), w("vh")),
            target: (w("tv"), w("th")),
            steps: w("steps"),
            settle: w("settle"),
            page_dir: w("dir"),
            shake: w("shake"),
        };
        let frames: Vec<String> = i["path"]
            .split(',')
            .map(|p| {
                let (h, v) = p.split_once(':').unwrap();
                let r = find_upper_left_corner(&mut view, (h.parse().unwrap(), v.parse().unwrap()), w("mode"), w("p2") != 0, w("p3") != 0);
                format!(
                    "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                    view.corner.0,
                    view.corner.1,
                    view.target.0,
                    view.target.1,
                    view.steps,
                    view.settle,
                    view.page_dir,
                    view.shake,
                    u8::from(r.area),
                    u8::from(r.full),
                    u8::from(r.black_lines)
                )
            })
            .collect();
        let got = frames.join(";");
        n += 1;
        if got != b {
            bad += 1;
            if bad <= 6 {
                let d: Vec<String> = b.split(';').zip(got.split(';')).filter(|(x, y)| x != y).take(2).map(|(x, y)| format!("want {x} got {y}")).collect();
                eprintln!("{a}\n  {}", d.join(", "));
            }
        }
    }
    assert_eq!(bad, 0, "{bad} of {n} cases differ");
    eprintln!("{n} cases");
}
