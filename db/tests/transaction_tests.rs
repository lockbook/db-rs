use db_rs::{Config, Db, LookupTable};
use db_rs_derive::Schema;
use std::fs::{remove_dir_all, remove_file, OpenOptions};
use std::io::{Read, Write};

#[derive(Schema)]
struct TxTest {
    table: LookupTable<u8, String>,
}

#[test]
fn simple_tx() {
    let dir = "/tmp/g";
    drop(remove_dir_all(dir));

    let mut db = TxTest::init(Config::in_folder(dir)).unwrap();
    let tx = db.begin_transaction();
    db.table.insert(43, "test".to_string()).unwrap();
    assert_eq!(db.table.data().get(&43), Some(&"test".to_string()));

    {
        let db = TxTest::init(Config::in_folder(dir)).unwrap();
        assert_eq!(db.table.data().get(&43), None);
    }
    drop(tx);
    {
        let db = TxTest::init(Config::in_folder(dir)).unwrap();
        assert_eq!(db.table.data().get(&43), Some(&"test".to_string()));
    }

    drop(remove_dir_all(dir));
}

#[test]
fn tx_log_corrupt() {
    let dir = "/tmp/h";
    drop(remove_dir_all(dir));

    let mut db = TxTest::init(Config::in_folder(dir)).unwrap();
    for _ in 0..10 {
        db.table.insert(41, "test".to_string()).unwrap();
    }

    let tx = db.begin_transaction();
    for _ in 0..20 {
        db.table.insert(43, "test".to_string()).unwrap();
    }
    drop(tx);

    // cut out half the log
    let mut buf = vec![];
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(db.config().db_location().unwrap())
        .unwrap();
    file.read_to_end(&mut buf).unwrap();

    buf = buf[0..buf.len() / 2].to_vec();

    remove_file(db.config().db_location().unwrap()).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(db.config().db_location().unwrap())
        .unwrap();
    file.write_all(&buf).unwrap();

    let db = TxTest::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.table.data().get(&41), Some(&"test".to_string()));
    assert_eq!(db.table.data().get(&43), None);

    drop(remove_dir_all(dir));
}

#[test]
fn snapshot_inter() {
    let dir = "/tmp/i";
    drop(remove_dir_all(dir));

    let mut db = TxTest::init(Config::in_folder(dir)).unwrap();

    for i in 0..u8::MAX {
        db.table.insert(i, "test".to_string()).unwrap();
    }

    db.compact_log().unwrap();

    // cut out half the log
    let mut buf = vec![];
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(db.config().db_location().unwrap())
        .unwrap();
    file.read_to_end(&mut buf).unwrap();

    buf = buf[0..buf.len() / 2].to_vec();

    remove_file(db.config().db_location().unwrap()).unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(db.config().db_location().unwrap())
        .unwrap();
    file.write_all(&buf).unwrap();

    let db = TxTest::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.table.data().get(&1), None);
    drop(remove_dir_all(dir));
}
