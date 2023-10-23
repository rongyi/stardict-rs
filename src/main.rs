mod parser;
mod sqlite;

use crate::sqlite::Db;
use std::error::Error;

fn dump_stardict() -> Result<(), Box<dyn Error>> {
    let idx = parser::Index::new(
        "testdata/stardict-oxford-gb-formated-2.4.2/oxford-gb-formated.idx",
        "m",
    )?;
    let mut dict = parser::Dictionary::new(
        "testdata/stardict-oxford-gb-formated-2.4.2/oxford-gb-formated.dict.dz",
        "m",
    )?;

    let mut db = Db::new("./test.db")?;
    db.create_table()?;

    for w in idx.into_iter() {
        let meaning = dict.get_oxford(w.offset as u64, w.sz as u64)?;
        db.insert_word(&w.word, &meaning)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    dump_stardict()?;

    Ok(())
}
