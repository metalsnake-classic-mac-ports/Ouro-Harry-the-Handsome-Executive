//! Lists the sprite ids each level of the pack uses: `cargo run -p
//! harry-data --example kinds [pack]`.

use std::collections::BTreeMap;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "re/work/harry-data.zip".into());
    let data = harry_data::pack::read(&std::fs::read(path).expect("pack")).expect("read pack");
    let mut all: BTreeMap<i16, Vec<usize>> = BTreeMap::new();
    for (n, level) in data.levels.iter().enumerate() {
        for e in level.sprites.iter().filter(|e| e.present && e.id != 0) {
            let l = all.entry(e.id).or_default();
            if l.last() != Some(&(n + 1)) {
                l.push(n + 1);
            }
        }
    }
    for (id, levels) in all {
        println!("{id:6}  {levels:?}");
    }
}
