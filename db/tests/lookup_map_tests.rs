use db_rs::utils::random_test_dir;
use db_rs::Db;
use db_rs::{Config, LookupMap};
use db_rs_derive::Schema;
use std::fs;

#[derive(Schema)]
pub struct LookupSchema {
    table1: LookupMap<u8, u8, String>,
    table2: LookupMap<u8, u8, String>,
    table3: LookupMap<u8, u8, String>,
    table4: LookupMap<u8, u8, String>,
    table5: LookupMap<u8, u8, String>,
}

#[test]
fn test() {
    let dir = &random_test_dir();
    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.insert(5, 0, "test".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    let target = ["test".to_string()];
    assert_eq!(db.table1.get().get(&5).unwrap().get(&0).unwrap(), &target[0]);
    drop(fs::remove_dir_all(dir));
}

#[test]
fn test2() {
    let dir = &random_test_dir();

    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.insert(1, 0, "test1".to_string()).unwrap();
    assert!(db.table1.get().get(&1).is_some());
    db.table1.insert(1, 1, "test2".to_string()).unwrap();
    db.table1.insert(1, 2, "test3".to_string()).unwrap();
    db.table1.insert(2, 0, "test4".to_string()).unwrap();
    db.table1.insert(3, 0, "test5".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    assert!(db
        .table1
        .get()
        .get(&1)
        .unwrap()
        .get(&0)
        .unwrap()
        .contains(&"test1".to_string()));
    assert!(db
        .table1
        .get()
        .get(&1)
        .unwrap()
        .get(&1)
        .unwrap()
        .contains(&"test2".to_string()));
    assert!(db
        .table1
        .get()
        .get(&1)
        .unwrap()
        .get(&2)
        .unwrap()
        .contains(&"test3".to_string()));
    assert!(db
        .table1
        .get()
        .get(&2)
        .unwrap()
        .get(&0)
        .unwrap()
        .contains(&"test4".to_string()));
    assert!(db
        .table1
        .get()
        .get(&3)
        .unwrap()
        .get(&0)
        .unwrap()
        .contains(&"test5".to_string()));

    drop(fs::remove_dir_all(dir));
}

#[test]
fn test3() {
    let dir = &random_test_dir();

    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.create_key(1).unwrap();
    assert!(db.table1.get().get(&1).is_some());
    assert!(db.table1.get().get(&1).unwrap().is_empty());

    drop(db);
    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    assert!(db.table1.get().get(&1).is_some());
    assert!(db.table1.get().get(&1).unwrap().is_empty());

    drop(fs::remove_dir_all(dir));
}

#[test]
fn test4() {
    let dir = &random_test_dir();

    drop(fs::remove_dir_all(dir));
    let mut db = LookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table1.insert(0, 0, "test1".to_string()).unwrap();
    db.table1.insert(0, 1, "test2".to_string()).unwrap();
    db.table1.remove(&0, &1).unwrap();

    db.table1.insert(1, 0, "test3".to_string()).unwrap();
    db.table1.insert(1, 1, "test4".to_string()).unwrap();

    db.table1.clear_key(&1).unwrap();

    drop(db);
    let db = LookupSchema::init(Config::in_folder(dir)).unwrap();

    assert!(db.table1.get().get(&1).is_none());
    assert!(db.table1.get().get(&0).unwrap().get(&1).is_none());

    drop(fs::remove_dir_all(dir));
}
