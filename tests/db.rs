use std::fs;

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

#[test]
fn snapshot_reduces_log_size() {
    let config = Config::test();
    let log_location = config.log_location.clone();
    let mut db = DbHashMap::<String, u64>::init(&config).unwrap();

    for value in 0..100 {
        db.write_tx().unwrap().insert("key".into(), value).unwrap();
        db.flush_pending().unwrap();
    }

    let original_size = fs::metadata(log_location.join("db.0.log")).unwrap().len();
    assert!(original_size > 1_000);

    db.snapshot().unwrap();

    let snapshot_size = fs::metadata(log_location.join("db.100.log")).unwrap().len();
    assert!(snapshot_size < original_size / 10);

    drop(db);
    let config = Config::default().log_location(log_location);
    let db = DbHashMap::<String, u64>::init(&config).unwrap();
    assert_eq!(db.read_tx().unwrap().get("key"), Some(&99));
}
