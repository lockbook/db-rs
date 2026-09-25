use db_rs::{View, config::Config, errors::Result, views::hashmap::DbHashMap};

fn main() -> Result<()> {
    let config = Config::default().log_location(".");
    let mut db = DbHashMap::<String, i32>::init(&config)?;

    let tx = db.write_tx()?;
    db.insert("one".to_owned(), 1)?;
    db.insert("two".to_owned(), 2)?;
    db.insert("three".to_owned(), 3)?;
    tx.end_tx(&mut db)?;

    let config = Config::default().log_location(".");
    let db = DbHashMap::<String, i32>::init(&config)?;
    let view = db.read_tx()?;
    assert_eq!(view.get("one"), Some(&1));
    assert_eq!(view.get("two"), Some(&2));
    assert_eq!(view.get("three"), Some(&3));
    Ok(())
}
