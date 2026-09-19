use std::fs;

use db_rs::{View, config::Config, views::hashmap::DbHashMap};

#[test]
fn write_transaction_catches_up_a_stale_view() {
    let config = Config::test();
    let mut first = DbHashMap::<String, u64>::init(&config).unwrap();
    let mut second = DbHashMap::<String, u64>::init(&config).unwrap();

    first.write_tx().unwrap().insert("key".into(), 42).unwrap();
    first.snapshot().unwrap();
    first
        .write_tx()
        .unwrap()
        .insert("new-key".into(), 43)
        .unwrap();

    let read = second.read_tx().unwrap();
    assert_eq!(read.get("key"), None);
    drop(read);

    second.write_tx().unwrap().end_tx().unwrap();

    let read = second.read_tx().unwrap();
    assert_eq!(read.get("key"), Some(&42));
    assert_eq!(read.get("new-key"), Some(&43));
}

#[test]
fn catch_up_crosses_each_snapshot() {
    let config = Config::test();
    let mut writer = DbHashMap::<String, u64>::init(&config).unwrap();

    {
        let mut tx = writer.write_tx().unwrap();
        tx.insert("removed".into(), 1).unwrap();
        tx.insert("retained".into(), 2).unwrap();
    }
    let mut stale = DbHashMap::<String, u64>::init(&config).unwrap();
    assert_eq!(stale.get("removed"), Some(&1));
    writer.snapshot().unwrap();
    writer
        .write_tx()
        .unwrap()
        .remove(&"removed".to_owned())
        .unwrap();
    writer.snapshot().unwrap();
    writer
        .write_tx()
        .unwrap()
        .insert("latest".into(), 3)
        .unwrap();

    stale.write_tx().unwrap().end_tx().unwrap();
    let read = stale.read_tx().unwrap();
    assert_eq!(read.get("removed"), None);
    assert_eq!(read.get("retained"), Some(&2));
    assert_eq!(read.get("latest"), Some(&3));
    drop(read);

    let fresh = DbHashMap::<String, u64>::init(&config).unwrap();
    assert_eq!(fresh.get("removed"), None);
    assert_eq!(fresh.get("retained"), Some(&2));
    assert_eq!(fresh.get("latest"), Some(&3));
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
