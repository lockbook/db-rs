use db_rs::{Config, Db, LookupTable, Single};
use db_rs_derive::Schema;
use std::fs::{remove_dir_all, remove_file, OpenOptions};
use std::io::{Read, Write};

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
        let mut data = db.table2.get().cloned().unwrap_or_default();
        data.push(i as u128 * i as u128);
        db.table2.insert(data).unwrap();
    }

    assert!(log_size(&db) > 500000);
    db.table1.clear().unwrap();
    db.compact_log().unwrap();
    assert!(log_size(&db) < (256 * (128 / 8)) + 1 + 4 + 100);

    let db = LogTests::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.table1.get(&4), None);
    assert_eq!(db.table2.get().unwrap().len() as u8, u8::MAX);

    drop(remove_dir_all(dir));
}

#[test]
fn inter_log() {
    let dir = "/tmp/f";
    drop(remove_dir_all(dir));

    let mut db = LogTests::init(Config::in_folder(dir)).unwrap();
    assert!(!db.incomplete_write());
    for i in 0..u8::MAX {
        db.table1
            .insert(i, format!("{i} * {i} = {}", i as usize * i as usize))
            .unwrap();
        let mut data = db.table2.get().cloned().unwrap_or_default();
        data.push(i as u128 * i as u128);
        db.table2.insert(data).unwrap();
    }

    let mut buf = vec![];
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(db.config().db_location().unwrap())
        .unwrap();

    file.read_to_end(&mut buf).unwrap();

    buf = buf[0..1000].to_vec();
    remove_file(db.config().db_location().unwrap()).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(db.config().db_location().unwrap())
        .unwrap();
    file.write_all(&buf).unwrap();

    let db = LogTests::init(Config::in_folder(dir)).unwrap();
    assert!(db.incomplete_write());
    assert_eq!(db.table1.get(&0).unwrap(), "0 * 0 = 0");
    drop(remove_dir_all(dir));
}

fn log_size<D: Db>(db: &D) -> usize {
    let mut buf = vec![];
    OpenOptions::new()
        .read(true)
        .open(db.config().db_location().unwrap())
        .unwrap()
        .read_to_end(&mut buf)
        .unwrap();

    buf.len()
}
