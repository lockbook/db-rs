use db_rs::{config::Config, db::Db, errors::Result, views::hashmap::SHashMap};

fn main() -> Result<()> {
    let config = Config::default().log_location(".");
    let mut db = Db::<SHashMap<String, i32>>::init(config)?;

    {
        let view = db.begin_tx();
        view.insert("one".to_owned(), 1)?;
        view.insert("two".to_owned(), 2)?;
        view.insert("three".to_owned(), 3)?;
    }

    db.end_tx();

    Ok(())
}
