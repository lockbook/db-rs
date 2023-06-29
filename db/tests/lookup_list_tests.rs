use db_rs::Db;
use db_rs::{Config, LookupList};
use db_rs_derive::Schema;
use std::fs;

#[derive(Schema)]
pub struct LookupSchema {
    table1: LookupList<u8, String>,
    table2: LookupList<u8, String>,
    table3: LookupList<u8, String>,
    table4: LookupList<u8, String>,
    table5: LookupList<u8, String>,
}

#[test]
fn test() {
    let dir = "/tmp/o/";
    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.push(5, "test".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    let target = vec!["test".to_string()];
    assert_eq!(db.table1.get().get(&5).unwrap(), &target);
    drop(fs::remove_dir_all(dir));
}

#[test]
fn test2() {
    let dir = "/tmp/p/";

    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.push(1, "test1".to_string()).unwrap();
    assert!(db.table1.get().get(&1).is_some());
    db.table1.push(2, "test2".to_string()).unwrap();
    db.table1.push(3, "test3".to_string()).unwrap();
    db.table1.push(4, "test4".to_string()).unwrap();
    db.table1.push(5, "test5".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    assert!(db
        .table1
        .get()
        .get(&1)
        .unwrap()
        .contains(&"test1".to_string()));
    assert!(db
        .table1
        .get()
        .get(&2)
        .unwrap()
        .contains(&"test2".to_string()));
    assert!(db
        .table1
        .get()
        .get(&3)
        .unwrap()
        .contains(&"test3".to_string()));
    assert!(db
        .table1
        .get()
        .get(&4)
        .unwrap()
        .contains(&"test4".to_string()));
    assert!(db
        .table1
        .get()
        .get(&5)
        .unwrap()
        .contains(&"test5".to_string()));

    drop(fs::remove_dir_all(dir));
}

#[test]
fn test3() {
    let dir = "/tmp/q/";

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
