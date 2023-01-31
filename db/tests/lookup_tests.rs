use db_rs::Db;
use db_rs::{Config, LookupTable};
use db_rs_derive::Schema;
use std::fs;

#[derive(Schema)]
pub struct LookupSchema {
    table1: LookupTable<u8, String>,
}

#[test]
fn test() {
    drop(fs::remove_dir_all("/tmp/c/"));
    let mut db = LookupSchema::init(Config::in_folder("/tmp/c/")).unwrap();
    db.table1.insert(5, "test".to_string()).unwrap();
    drop(db);

    let db = LookupSchema::init(Config::in_folder("/tmp/c/")).unwrap();
    assert_eq!(db.table1.get(&5).unwrap(), "test");
    drop(fs::remove_dir_all("/tmp/c/").unwrap());
}
