use db_rs::Db;
use db_rs::{Config, LookupTable};
use db_rs_derive::Schema;
use std::fs;

#[derive(Schema)]
pub struct LookupSchema {
    table1: LookupTable<u8, String>,
    table2: LookupTable<u8, String>,
    table3: LookupTable<u8, String>,
    table4: LookupTable<u8, String>,
    table5: LookupTable<u8, String>,
}

#[test]
fn test() {
    let dir = "/tmp/a/";
    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.insert(5, "test".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.table1.get(&5).unwrap(), "test");
    drop(fs::remove_dir_all(dir).unwrap());
}

#[test]
fn test2() {
    let dir = "/tmp/b/";

    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.insert(5, "test".to_string()).unwrap();
    db.table1.insert(5, "test".to_string()).unwrap();
    db.table1.insert(1, "test1".to_string()).unwrap();
    db.table1.insert(2, "tes2".to_string()).unwrap();
    db.table1.insert(3, "test3".to_string()).unwrap();
    db.table1.insert(5, "test5".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.table1.get(&1).unwrap(), "test1");
    assert_eq!(db.table1.get(&2).unwrap(), "tes2");
    assert_eq!(db.table1.get(&3).unwrap(), "test3");
    assert_eq!(db.table1.get(&5).unwrap(), "test5");
    drop(fs::remove_dir_all(dir).unwrap());
}
