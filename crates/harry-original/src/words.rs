//! The sentence generators' word lists (`STR#` 6900–6905 and 6800–6801
//! of the application).

use harry_core::words::{Words, parse_str_list};

use crate::resfork::{Error, Resources};

pub fn words(app: &Resources) -> Result<Words, Error> {
    let list = |id: i16| app.get(b"STR#", id).map(parse_str_list);
    let mut buzz: [Vec<Vec<u8>>; 6] = Default::default();
    for (i, b) in buzz.iter_mut().enumerate() {
        *b = list(6900 + i as i16)?;
    }
    Ok(Words { buzz, phone: [list(6800)?, list(6801)?], last: [0; 8] })
}
