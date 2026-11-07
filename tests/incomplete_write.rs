use std::{
    fs::{self, OpenOptions},
    path::PathBuf,
};

use db_rs::{View, config::Config, log::Log, views::hashmap::DbHashMap};

type Map = DbHashMap<String, u64>;

#[test]
fn interrupted_write_keeps_earlier_transactions() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();
    write(&mut db, "one", 1);
    let committed = latest_log_len(&config);
    write(&mut db, "two", 2);
    drop(db);
    let log = fs::read(latest_log(&config)).unwrap();

    for cut in committed + 1..log.len() {
        let config = Config::test();
        fs::write(config.log_location.join("db.0.log"), &log[..cut]).unwrap();

        let mut db = Map::init(&config).unwrap();
        assert_eq!(db.get("one"), Some(&1));
        assert_eq!(db.get("two"), None);
        assert_eq!(db.log_mut().take_incomplete_write(), Some(cut - committed));
        assert_eq!(db.log_mut().take_incomplete_write(), None);
        assert_eq!(latest_log_len(&config), committed);
    }
}

#[test]
fn writes_after_an_interrupted_write_survive_reopening() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();
    write(&mut db, "one", 1);
    write(&mut db, "two", 2);
    drop(db);
    interrupt_last_write(&config);

    let mut db = Map::init(&config).unwrap();
    write(&mut db, "three", 3);
    drop(db);

    let mut db = Map::init(&config).unwrap();
    assert_eq!(db.get("one"), Some(&1));
    assert_eq!(db.get("two"), None);
    assert_eq!(db.get("three"), Some(&3));
    assert_eq!(db.log_mut().take_incomplete_write(), None);
}

#[test]
fn open_views_catch_up_past_an_interrupted_write() {
    let config = Config::test();
    let mut first = Map::init(&config).unwrap();
    let mut second = Map::init(&config).unwrap();
    let mut interrupted = Map::init(&config).unwrap();
    write(&mut interrupted, "one", 1);
    write(&mut interrupted, "two", 2);
    drop(interrupted);
    interrupt_last_write(&config);

    first.write_tx().unwrap().end_tx(&mut first).unwrap();
    assert!(first.log_mut().take_incomplete_write().is_some());
    write(&mut second, "three", 3);
    first.write_tx().unwrap().end_tx(&mut first).unwrap();

    for db in [&mut first, &mut second] {
        assert_eq!(db.get("one"), Some(&1));
        assert_eq!(db.get("two"), None);
        assert_eq!(db.get("three"), Some(&3));
        assert_eq!(db.log_mut().take_incomplete_write(), None);
    }
}

#[test]
fn interrupted_write_after_a_snapshot() {
    let config = Config::test();
    let mut open = Map::init(&config).unwrap();
    let mut interrupted = Map::init(&config).unwrap();
    write(&mut interrupted, "one", 1);
    interrupted.snapshot().unwrap();
    write(&mut interrupted, "two", 2);
    drop(interrupted);
    interrupt_last_write(&config);

    write(&mut open, "three", 3);
    assert!(open.log_mut().take_incomplete_write().is_some());

    let db = Map::init(&config).unwrap();
    assert_eq!(db.get("one"), Some(&1));
    assert_eq!(db.get("two"), None);
    assert_eq!(db.get("three"), Some(&3));
}

fn write(db: &mut Map, key: &str, value: u64) {
    let tx = db.write_tx().unwrap();
    db.insert(key.into(), value).unwrap();
    tx.end_tx(db).unwrap();
}

fn latest_log(config: &Config) -> PathBuf {
    Log::find_latest(config).unwrap().unwrap()
}

fn latest_log_len(config: &Config) -> usize {
    fs::metadata(latest_log(config)).unwrap().len() as usize
}

fn interrupt_last_write(config: &Config) {
    let log = OpenOptions::new()
        .write(true)
        .open(latest_log(config))
        .unwrap();
    log.set_len(log.metadata().unwrap().len() - 1).unwrap();
}
