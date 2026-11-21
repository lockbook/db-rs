use std::time::{SystemTime, UNIX_EPOCH};

use db_rs::{View, config::Config, errors::Error, views::hashmap::DbHashMap};

type Map = DbHashMap<u64, u64>;

const YEAR: u64 = 365 * 24 * 60 * 60 * 1000;

fn now() -> u64 {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    u64::try_from(elapsed.as_millis()).unwrap()
}

fn commit_at(db: &mut Map, seq_no: u64) {
    let tx = db.write_tx().unwrap();
    db.insert(seq_no, seq_no).unwrap();
    db.flush_pending(seq_no).unwrap();
    assert_eq!(tx.end_tx(db).unwrap(), seq_no);
}

#[test]
fn sequence_is_the_time_of_the_transaction() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();

    let before = now();
    let tx = db.write_tx().unwrap();
    db.insert(1, 10).unwrap();
    let seq_no = tx.end_tx(&mut db).unwrap();
    let after = now();

    assert!((before..=after).contains(&seq_no));
    let reopened = Map::init(&config).unwrap();
    assert_eq!(reopened.last_modified(), seq_no);
}

#[test]
fn sequences_increase_within_one_clock_reading() {
    let mut db = Map::init(&Config::in_memory()).unwrap();
    let mut previous = 0;

    for value in 0..10_000 {
        let tx = db.write_tx().unwrap();
        db.insert(0, value).unwrap();
        let seq_no = tx.end_tx(&mut db).unwrap();
        assert!(seq_no > previous);
        previous = seq_no;
    }

    assert!(previous > now());
}

#[test]
fn sequence_counts_up_from_a_log_ahead_of_the_clock() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();
    let mut other = Map::init(&config).unwrap();
    let ahead = now() + YEAR;
    commit_at(&mut db, ahead);

    let tx = other.write_tx().unwrap();
    assert_eq!(tx.seq_no, ahead + 1);
    other.insert(2, 20).unwrap();
    assert_eq!(tx.end_tx(&mut other).unwrap(), ahead + 1);

    let tx = db.write_tx().unwrap();
    assert_eq!(tx.seq_no, ahead + 2);
    db.insert(3, 30).unwrap();
    assert_eq!(tx.end_tx(&mut db).unwrap(), ahead + 2);
    db.snapshot().unwrap();

    let reopened = Map::init(&config).unwrap();
    assert_eq!(reopened.last_modified(), ahead + 2);
    assert_eq!(reopened.get(&ahead), Some(&ahead));
    assert_eq!(reopened.get(&2), Some(&20));
    assert_eq!(reopened.get(&3), Some(&30));
}

#[test]
fn sequence_joins_the_clock_from_a_counter() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();
    for seq_no in 1..=3 {
        commit_at(&mut db, seq_no);
    }

    let before = now();
    let tx = db.write_tx().unwrap();
    db.insert(4, 40).unwrap();
    let seq_no = tx.end_tx(&mut db).unwrap();

    assert!(seq_no >= before);
    let reopened = Map::init(&config).unwrap();
    assert_eq!(reopened.last_modified(), seq_no);
    assert_eq!(reopened.len(), 4);
}

#[test]
fn last_sequence_cannot_be_followed() {
    let mut db = Map::init(&Config::in_memory()).unwrap();
    commit_at(&mut db, u64::MAX);

    assert!(matches!(db.write_tx(), Err(Error::SequenceExhausted)));
}
