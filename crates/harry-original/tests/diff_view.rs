//! The view (`FindUpperLeftCorner`) against the original, recorded in
//! Unicorn by `re/tools/emu_view.py`.

mod common;

use common::{Tally, named_cases, recording};
use harry_core::view::{View, find_upper_left_corner};

#[test]
fn view_matches_the_original() {
    let Some(text) = recording("view", "emu_view.py") else { return };
    let mut tally = Tally::new("view", 8);
    for (a, i, b) in named_cases(&text) {
        let w = |k: &str| i.i16(k);
        let mut view = View {
            corner: (w("vv"), w("vh")),
            target: (w("tv"), w("th")),
            steps: w("steps"),
            settle: w("settle"),
            page_dir: w("dir"),
            shake: w("shake"),
        };
        let frames: Vec<String> = i.str("path")
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
        tally.words(a, b, &got);
    }
    tally.finish();
}
