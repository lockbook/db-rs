use db_rs::Db;
use db_rs::{Config, LookupSet};
use db_rs_derive::Schema;
use std::collections::HashSet;
use std::fs;

#[derive(Schema)]
pub struct LookupSchema {
    table1: LookupSet<u8, String>,
    table2: LookupSet<u8, String>,
    table3: LookupSet<u8, String>,
    table4: LookupSet<u8, String>,
    table5: LookupSet<u8, String>,
}

#[test]
fn test() {
    let dir = "/tmp/l/";
    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.insert(5, "test".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    let mut target = HashSet::new();
    target.insert("test".to_string());
    assert_eq!(db.table1.get().get(&5).unwrap(), &target);
    drop(fs::remove_dir_all(dir));
}

#[test]
fn test2() {
    let dir = "/tmp/m/";

    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.insert(1, "test1".to_string()).unwrap();
    assert!(db.table1.get().get(&1).is_some());
    db.table1.insert(2, "test2".to_string()).unwrap();
    db.table1.insert(3, "test3".to_string()).unwrap();
    db.table1.insert(4, "test4".to_string()).unwrap();
    db.table1.insert(5, "test5".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    assert!(db.table1.get().get(&1).unwrap().contains("test1"));
    assert!(db.table1.get().get(&2).unwrap().contains("test2"));
    assert!(db.table1.get().get(&3).unwrap().contains("test3"));
    assert!(db.table1.get().get(&4).unwrap().contains("test4"));
    assert!(db.table1.get().get(&5).unwrap().contains("test5"));

    drop(fs::remove_dir_all(dir));
}

#[test]
fn test3() {
    let dir = "/tmp/n/";

    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.create_key(1).unwrap();
    assert!(db.table1.get().get(&1).is_some());
    assert!(db.table1.get().get(&1).unwrap().is_empty());

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    assert!(db.table1.get().get(&1).is_some());
    assert!(db.table1.get().get(&1).unwrap().is_empty());

    drop(fs::remove_dir_all(dir));
}
