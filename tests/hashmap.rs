use db_rs::{View, config::Config, views::hashmap::DbHashMap};

#[test]
fn round_trip() {
    let config = Config::test();
    let log_location = config.log_location.clone();

    {
        let mut db = DbHashMap::<String, u64>::init(&config).unwrap();
        {
            let mut view = db.write_tx().unwrap();
            view.insert("one".into(), 1).unwrap();
            view.insert("two".into(), 2).unwrap();
            view.insert("three".into(), 3).unwrap();
        }
    }

    let config = Config::default().log_location(log_location);
    let db = DbHashMap::<String, u64>::init(&config).unwrap();
    let view = db.read_tx().unwrap();
    assert_eq!(view.get("one"), Some(&1));
    assert_eq!(view.get("two"), Some(&2));
    assert_eq!(view.get("three"), Some(&3));
}
