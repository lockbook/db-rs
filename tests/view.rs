use std::fs::{self, OpenOptions, TryLockError};

use db_rs::{View, config::Config, views::hashmap::DbHashMap};

#[test]
fn read_transactions_release_their_locks_independently() {
    let config = Config::test();
    let db = DbHashMap::<String, u64>::init(&config).unwrap();
    let first = db.read_tx().unwrap();
    let second = db.read_tx().unwrap();
    let writer = OpenOptions::new()
        .read(true)
        .write(true)
        .open(config.log_location.join("db.lock"))
        .unwrap();

    drop(first);
    assert!(matches!(writer.try_lock(), Err(TryLockError::WouldBlock)));

    drop(second);
    writer.try_lock().unwrap();
    writer.unlock().unwrap();
}

#[test]
fn write_transaction_sequence() {
    let config = Config::test();
    let mut db = DbHashMap::<String, u64>::init(&config).unwrap();
    let mut other = DbHashMap::<String, u64>::init(&config).unwrap();

    let tx = db.write_tx().unwrap();
    assert_eq!(tx.seq_no, 1);
    assert_eq!(tx.end_tx(&mut db).unwrap(), 0);

    let tx = db.write_tx().unwrap();
    assert_eq!(tx.seq_no, 1);
    db.insert("key".into(), 42).unwrap();
    assert_eq!(tx.end_tx(&mut db).unwrap(), 1);
    assert_eq!(db.last_modified(), 1);
    db.snapshot().unwrap();

    let tx = other.write_tx().unwrap();
    assert_eq!(tx.seq_no, 2);
    other.insert("key".into(), 43).unwrap();
    assert_eq!(tx.end_tx(&mut other).unwrap(), 2);
    assert_eq!(other.last_modified(), 2);

    let reopened = DbHashMap::<String, u64>::init(&config).unwrap();
    assert_eq!(reopened.last_modified(), 2);
    assert_eq!(reopened.get("key"), Some(&43));
}

#[test]
fn dropping_write_transaction_does_not_flush_or_roll_back() {
    let config = Config::test();
    let mut db = DbHashMap::<String, u64>::init(&config).unwrap();
    let tx = db.write_tx().unwrap();
    db.insert("key".into(), 42).unwrap();
    drop(tx);

    assert_eq!(db.get("key"), Some(&42));
    let reopened = DbHashMap::<String, u64>::init(&config).unwrap();
    assert_eq!(reopened.get("key"), None);

    db.write_tx().unwrap().end_tx(&mut db).unwrap();
    let reopened = DbHashMap::<String, u64>::init(&config).unwrap();
    assert_eq!(reopened.get("key"), Some(&42));
}

#[test]
fn write_transaction_catches_up_a_stale_view() {
    let config = Config::test();
    let mut first = DbHashMap::<String, u64>::init(&config).unwrap();
    let mut second = DbHashMap::<String, u64>::init(&config).unwrap();

    let tx = first.write_tx().unwrap();
    first.insert("key".into(), 42).unwrap();
    tx.end_tx(&mut first).unwrap();
    first.snapshot().unwrap();
    let tx = first.write_tx().unwrap();
    first.insert("new-key".into(), 43).unwrap();
    tx.end_tx(&mut first).unwrap();

    let read = second.read_tx().unwrap();
    assert_eq!(read.get("key"), None);
    drop(read);

    second.write_tx().unwrap().end_tx(&mut second).unwrap();

    let read = second.read_tx().unwrap();
    assert_eq!(read.get("key"), Some(&42));
    assert_eq!(read.get("new-key"), Some(&43));
}

#[test]
fn catch_up_crosses_each_snapshot() {
    let config = Config::test();
    let mut writer = DbHashMap::<String, u64>::init(&config).unwrap();

    {
        let tx = writer.write_tx().unwrap();
        writer.insert("removed".into(), 1).unwrap();
        writer.insert("retained".into(), 2).unwrap();
        tx.end_tx(&mut writer).unwrap();
    }
    let mut stale = DbHashMap::<String, u64>::init(&config).unwrap();
    assert_eq!(stale.get("removed"), Some(&1));
    writer.snapshot().unwrap();
    let tx = writer.write_tx().unwrap();
    writer.remove(&"removed".to_owned()).unwrap();
    tx.end_tx(&mut writer).unwrap();
    writer.snapshot().unwrap();
    let tx = writer.write_tx().unwrap();
    writer.insert("latest".into(), 3).unwrap();
    tx.end_tx(&mut writer).unwrap();

    stale.write_tx().unwrap().end_tx(&mut stale).unwrap();
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
        let tx = db.write_tx().unwrap();
        db.insert("key".into(), value).unwrap();
        tx.end_tx(&mut db).unwrap();
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
