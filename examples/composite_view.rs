use db_rs::{
    View,
    config::Config,
    errors::Result,
    views::{
        composite_view::{Composite, Schema},
        hashmap::DbHashMap,
        option::DbOption,
    },
};

#[derive(Default)]
struct AppSchema {
    users: DbHashMap<String, String>,
    settings: DbOption<String>,
}

impl Schema for AppSchema {
    fn views_mut(&mut self) -> impl AsMut<[&mut dyn View]> {
        let views: [&mut dyn View; 2] = [&mut self.users, &mut self.settings];
        views
    }
}

fn main() -> Result<()> {
    let config = Config::default().log_location("composite-view-data");
    std::fs::create_dir_all(&config.log_location)?;

    {
        let mut db = Composite::<AppSchema>::init(&config)?;

        let mut tx = db.write_tx()?;
        tx.schema.users.insert("alice".into(), "Alice".into())?;
        tx.schema.users.insert("bob".into(), "Bob".into())?;
        tx.schema.settings.replace("dark".into())?;
        tx.end_tx()?;

        db.snapshot()?;
    }

    let db = Composite::<AppSchema>::init(&config)?;
    let tx = db.read_tx()?;
    assert_eq!(
        tx.schema.users.get("alice").map(String::as_str),
        Some("Alice")
    );
    assert_eq!(tx.schema.users.get("bob").map(String::as_str), Some("Bob"));
    assert_eq!(
        tx.schema.settings.as_ref().map(String::as_str),
        Some("dark")
    );

    println!(
        "Users and settings restored from {}",
        config.log_location.display()
    );
    Ok(())
}
