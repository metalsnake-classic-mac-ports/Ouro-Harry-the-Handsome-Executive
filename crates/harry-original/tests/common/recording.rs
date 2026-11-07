//! The recordings and original files the differential tests read, and the
//! tally that compares the port's results with them.
//!
//! A recording is `re/work/diff/<name>.txt`, written by a recorder in
//! `re/tools/` that runs the original's routines in Unicorn. Two formats:
//! - numbers: `<inputs> -> <outputs>`, whitespace-separated integers,
//!   `#` lines are comments (`numeric_cases`);
//! - named: a header line, then `k=v k=v … -> <expected>` (`named_cases`),
//!   the expected text usually the recorder's sprite dump.
//!
//! Without the recordings (they derive from the original files and stay in
//! the gitignored `re/work/`) a test is skipped with a note.

use std::collections::HashMap;
use std::path::PathBuf;

use harry_core::Level;
use harry_original::hlvl;
use harry_original::resfork::{MacBinary, Resources};

/// The repository's root.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A path under `re/work/`.
pub fn work(rel: &str) -> PathBuf {
    root().join("re/work").join(rel)
}

/// The text of `re/work/<rel>`, or none with a "skipped" note naming the
/// recorder (`re/tools/<tool>`) that writes it.
pub fn work_text(rel: &str, tool: &str) -> Option<String> {
    let text = std::fs::read_to_string(work(rel)).ok();
    if text.is_none() {
        eprintln!("skipped: {rel} not found (run re/tools/{tool})");
    }
    text
}

/// The recording `re/work/diff/<name>.txt` (see `work_text`).
pub fn recording(name: &str, tool: &str) -> Option<String> {
    work_text(&format!("diff/{name}.txt"), tool)
}

/// The resource fork of an installed original file
/// (`re/work/installed/<name>`, MacBinary), or none with a "skipped" note.
pub fn installed(name: &str) -> Option<Resources> {
    let Ok(bytes) = std::fs::read(work("installed").join(name)) else {
        eprintln!("skipped: installed/{name} not found");
        return None;
    };
    let mb = MacBinary::parse(&bytes).expect("MacBinary");
    Some(Resources::parse(&mb.rsrc).expect("resource fork"))
}

/// The 15 levels of a world's resources (`Hlvl` 1–15).
pub fn levels_of(world: &Resources) -> Vec<Level> {
    (1..=15).map(|i| hlvl::read(world.get(b"Hlvl", i).unwrap()).unwrap()).collect()
}

/// The levels of Episode 1.
pub fn episode1_levels() -> Option<Vec<Level>> {
    installed("Episode 1.bin").map(|ep| levels_of(&ep))
}

/// Whitespace-separated integers.
pub fn numbers(s: &str) -> Vec<i32> {
    s.split_whitespace().map(|x| x.parse().unwrap()).collect()
}

/// The cases of a numbers recording: (line, inputs, outputs).
pub fn numeric_cases(text: &str) -> impl Iterator<Item = (&str, Vec<i32>, Vec<i32>)> {
    text.lines().filter(|l| !l.starts_with('#')).map(|line| {
        let (a, b) = line.split_once("->").unwrap();
        (line, numbers(a), numbers(b))
    })
}

/// The `k=v` inputs of a named case.
pub struct Fields<'a>(HashMap<&'a str, &'a str>);

impl<'a> Fields<'a> {
    pub fn parse(s: &'a str) -> Fields<'a> {
        Fields(s.split(' ').map(|kv| kv.split_once('=').unwrap()).collect())
    }

    /// From names and values given apart (a header names the columns).
    pub fn zip(names: &[&'a str], values: &'a str) -> Fields<'a> {
        Fields(names.iter().copied().zip(values.split(' ')).collect())
    }

    pub fn has(&self, k: &str) -> bool {
        self.0.contains_key(k)
    }

    pub fn str(&self, k: &str) -> &'a str {
        self.0.get(k).unwrap_or_else(|| panic!("no field {k}"))
    }

    pub fn i64(&self, k: &str) -> i64 {
        self.str(k).parse().unwrap_or_else(|_| panic!("field {k} is not a number"))
    }

    /// The value cut to 32 bits.
    pub fn i32(&self, k: &str) -> i32 {
        self.i64(k) as i32
    }

    /// The value cut to 16 bits (as the original's fields hold it).
    pub fn i16(&self, k: &str) -> i16 {
        self.i64(k) as i16
    }

    pub fn u32(&self, k: &str) -> u32 {
        self.i64(k) as u32
    }

    /// Non-zero.
    pub fn flag(&self, k: &str) -> bool {
        self.i64(k) != 0
    }
}

/// The cases of a named recording (the header line skipped): (inputs as
/// written, inputs, expected).
pub fn named_cases(text: &str) -> impl Iterator<Item = (&str, Fields<'_>, &str)> {
    text.lines().skip(1).map(|line| {
        let (a, b) = line.split_once(" -> ").unwrap();
        (a, Fields::parse(a), b)
    })
}

/// Counts the cases and the ones that differ, reports the first few and
/// fails at the end if any differed.
pub struct Tally {
    name: String,
    shown: usize,
    bad: usize,
    n: usize,
}

impl Tally {
    /// `name` labels the messages; the first `shown` differing cases are
    /// reported.
    pub fn new(name: &str, shown: usize) -> Tally {
        Tally { name: name.to_string(), shown, bad: 0, n: 0 }
    }

    /// One case: `ok`, else the report `what` (made only when shown).
    pub fn case(&mut self, ok: bool, what: impl FnOnce() -> String) -> bool {
        self.n += 1;
        if !ok {
            self.bad += 1;
            if self.bad <= self.shown {
                eprintln!("{}: {}", self.name, what());
            }
        }
        ok
    }

    /// One case compared as text: the inputs and the words that differ.
    pub fn words(&mut self, input: &str, want: &str, got: &str) -> bool {
        self.case(got == want, || format!("{input}\n  {}", word_diff(want, got)))
    }

    /// One case compared as text, every word that differs on its own line.
    pub fn words_lines(&mut self, input: &str, want: &str, got: &str) -> bool {
        self.case(got == want, || format!("{input}\n  {}", word_diffs(want, got).join("\n  ")))
    }

    /// One case of a numbers recording.
    pub fn numbers<T: PartialEq + std::fmt::Debug>(&mut self, line: &str, want: &[T], got: &[T]) -> bool {
        self.case(got == want, || format!("{line}\n  port {got:?}"))
    }

    /// Cases counted so far.
    pub fn count(&self) -> usize {
        self.n
    }

    /// Fails if a case differed; else notes how many passed.
    pub fn finish(self) {
        let Tally { name, bad, n, .. } = self;
        assert_eq!(bad, 0, "{name}: {bad} of {n} cases differ");
        eprintln!("{name}: {n} cases");
    }
}

/// Every case of the named recording `name` (by `re/tools/<tool>`): `run`
/// makes the port's text from the inputs, compared word by word.
pub fn check_named(name: &str, tool: &str, mut run: impl FnMut(&Fields) -> String) {
    let Some(text) = recording(name, tool) else { return };
    let mut tally = Tally::new(name, 8);
    for (a, i, b) in named_cases(&text) {
        let got = run(&i);
        tally.words(a, b, &got);
    }
    tally.finish();
}

/// Every case of the numbers recording `name` (by `re/tools/<tool>`):
/// `run` makes the port's outputs from the inputs.
pub fn check_numbers(name: &str, tool: &str, mut run: impl FnMut(&[i32]) -> Vec<i32>) {
    let Some(text) = recording(name, tool) else { return };
    let mut tally = Tally::new(name, 10);
    for (line, input, want) in numeric_cases(&text) {
        let got = run(&input);
        tally.numbers(line, &want, &got);
    }
    tally.finish();
}

/// The words (space-separated) that differ: "want x got y".
pub fn word_diffs(want: &str, got: &str) -> Vec<String> {
    want.split(' ').zip(got.split(' ')).filter(|(x, y)| x != y).map(|(x, y)| format!("want {x} got {y}")).collect()
}

/// `word_diffs` on one line.
pub fn word_diff(want: &str, got: &str) -> String {
    word_diffs(want, got).join(", ")
}
