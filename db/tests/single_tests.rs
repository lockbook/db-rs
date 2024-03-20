use db_rs::{Config, Db, Single};
use db_rs_derive::Schema;
use std::fs::remove_dir_all;
use std::path::PathBuf;

#[derive(Schema)]
pub struct SingleSchema {
    table1: Single<u8>,
    table2: Single<String>,
    table3: Single<u32>,
    table4: Single<Vec<u8>>,
    table5: Single<PathBuf>,
}

#[test]
fn test_simple() {
    let dir = "/tmp/c";

    drop(remove_dir_all(dir));

    let mut db = SingleSchema::init(Config::in_folder(dir)).unwrap();
    let mut db = db.lock().unwrap();
    db.table1.insert(5).unwrap();
    assert_eq!(db.table1.get(), Some(&5));

    let mut db = SingleSchema::init(Config::in_folder(dir)).unwrap();
    let mut db = db.lock().unwrap();
    assert_eq!(db.table1.get(), Some(&5));
    assert_eq!(db.table1.clear().unwrap(), Some(5));
    assert_eq!(db.table1.get(), None);

    let db = SingleSchema::init(Config::in_folder(dir)).unwrap();
    let mut db = db.lock().unwrap();
    assert_eq!(db.table1.get(), None);

    drop(remove_dir_all(dir));
}

#[test]
fn test_complex() {
    let dir = "/tmp/d";

    drop(remove_dir_all(dir));

    let mut db = SingleSchema::init(Config::in_folder(dir)).unwrap();
    let mut db = db.lock().unwrap();
    db.table1.insert(5).unwrap();
    db.table2.insert("test".to_string()).unwrap();
    db.table3.insert(u32::MAX).unwrap();
    db.table4.insert("--offline test --color=always --test single_tests test_simple --no-fail-fast --manifest-path /Users/parth/Documents/db-rs/db/Cargo.toml -- --format=json --exact -Z unstable-options --show-output".bytes().collect()).unwrap();
    db.table5
        .insert(PathBuf::from("/test/test/test/test/"))
        .unwrap();

    let db = SingleSchema::init(Config::in_folder(dir)).unwrap();
    let mut db = db.lock().unwrap();
    assert_eq!(db.table1.get().unwrap(), &5);
    assert_eq!(db.table2.get().unwrap(), &("test".to_string()));
    assert_eq!(db.table3.get().unwrap(), &u32::MAX);
    assert_eq!(db.table4.get().unwrap(), &"--offline test --color=always --test single_tests test_simple --no-fail-fast --manifest-path /Users/parth/Documents/db-rs/db/Cargo.toml -- --format=json --exact -Z unstable-options --show-output".bytes().collect::<Vec<u8>>());
    assert_eq!(db.table5.get().unwrap(), &PathBuf::from("/test/test/test/test/"));

    drop(remove_dir_all(dir));
}
