use db_rs::compacter::BackgroundCompacter;
use db_rs::{CancelSig, Config, Db, LookupTable, Single};
use db_rs_derive::Schema;
use std::fs::{remove_dir_all, remove_file, OpenOptions};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Schema)]
pub struct LogTests {
    table1: LookupTable<u8, String>,
    table2: Single<Vec<u128>>,
}

#[test]
fn log_compaction() {
    let dir = "/tmp/e";
    drop(remove_dir_all(dir));

    let mut db = LogTests::init(Config::in_folder(dir)).unwrap();
    for i in 0..u8::MAX {
        db.table1
            .insert(i, format!("{i} * {i} = {}", i as usize * i as usize))
            .unwrap();
        let mut data = db.table2.data().cloned().unwrap_or_default();
        data.push(i as u128 * i as u128);
        db.table2.insert(data).unwrap();
    }

    assert!(log_size(&db) > 500000);
    db.table1.clear().unwrap();
    db.compact_log().unwrap();
    assert!(log_size(&db) < (256 * (128 / 8)) + 1 + 4 + 100);

    let db = LogTests::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.table1.data().get(&4), None);
    assert_eq!(db.table2.data().unwrap().len() as u8, u8::MAX);

    drop(remove_dir_all(dir));
}

#[test]
fn inter_log() {
    let dir = "/tmp/f";
    drop(remove_dir_all(dir));

    let mut db = LogTests::init(Config::in_folder(dir)).unwrap();
    assert!(!db.incomplete_write().unwrap());
    for i in 0..u8::MAX {
        db.table1
            .insert(i, format!("{i} * {i} = {}", i as usize * i as usize))
            .unwrap();
        let mut data = db.table2.data().cloned().unwrap_or_default();
        data.push(i as u128 * i as u128);
        db.table2.insert(data).unwrap();
    }

    let mut buf = vec![];
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(db.config().unwrap().db_location().unwrap())
        .unwrap();

    file.read_to_end(&mut buf).unwrap();

    buf = buf[0..1000].to_vec();
    remove_file(db.config().unwrap().db_location().unwrap()).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(db.config().unwrap().db_location().unwrap())
        .unwrap();
    file.write_all(&buf).unwrap();

    let db = LogTests::init(Config::in_folder(dir)).unwrap();
    assert!(db.incomplete_write().unwrap());
    assert_eq!(db.table1.data().get(&0).unwrap(), "0 * 0 = 0");
    drop(remove_dir_all(dir));
}

#[test]
#[ignore] // ignored so tests don't get stuck here
fn auto_log_compacter() {
    let dir = "/tmp/fa";
    drop(remove_dir_all(dir));
    let db = Arc::new(Mutex::new(LogTests::init(Config::in_folder(dir)).unwrap()));
    let cancel = CancelSig::default();
    let handle = db.begin_compacter(Duration::from_secs(1), cancel.clone());
    thread::sleep(Duration::from_millis(2500));
    cancel.cancel();
    assert_eq!(handle.join().unwrap().unwrap(), 2);

    drop(remove_dir_all(dir));
}

fn log_size<D: Db>(db: &D) -> usize {
    let mut buf = vec![];
    OpenOptions::new()
        .read(true)
        .open(db.config().unwrap().db_location().unwrap())
        .unwrap()
        .read_to_end(&mut buf)
        .unwrap();

    buf.len()
}
