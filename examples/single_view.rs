use db_rs::{View, config::Config, errors::Result, views::hashmap::DbHashMap};

fn main() -> Result<()> {
    let config = Config::default().log_location(".");
    let mut db = DbHashMap::<String, i32>::init(&config)?;

    {
        let view = db.write_tx()?;
        view.insert("one".to_owned(), 1)?;
        view.insert("two".to_owned(), 2)?;
        view.insert("three".to_owned(), 3)?;
    }

    db.end_tx()?;
    drop(db);

    let config = Config::default().log_location(".");
    let db = DbHashMap::<String, i32>::init(&config)?;
    let view = db.read_tx()?;
    assert_eq!(view.get("one"), Some(&1));
    assert_eq!(view.get("two"), Some(&2));
    assert_eq!(view.get("three"), Some(&3));
    Ok(())
}
