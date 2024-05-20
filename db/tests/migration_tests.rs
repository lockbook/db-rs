use std::{fs, path::PathBuf};

use db_rs::{Config, Db, LookupTable};
use db_rs_derive::Schema;

#[derive(Schema)]
struct MigrationTest {
    table: LookupTable<u8, String>,
}

#[test]
fn migration_test() {
    let orig = PathBuf::from("tests/test_data/v1_log_format/MigrationTest.original");
    let v1 = PathBuf::from("tests/test_data/v1_log_format/MigrationTest");
    let v2 = PathBuf::from("tests/test_data/v1_log_format/MigrationTest.db");

    // ensure reasonable starting point
    assert!(orig.exists());
    let _ = fs::remove_file(&v1);
    let _ = fs::remove_file(&v2);

    // copy original to create test file
    fs::copy(orig, &v1).unwrap();

    // run init ensure migration succeeds
    let db = MigrationTest::init(Config::in_folder("tests/test_data/v1_log_format")).unwrap();

    // make sure the files we expect are there
    assert!(v2.exists());
    assert!(!v1.exists());

    // make sure the data we expect is there
    assert_eq!(db.table.get().len(), 4);
    assert_eq!(db.table.get().get(&0).unwrap(), "zero");
    assert_eq!(db.table.get().get(&1).unwrap(), "one");
    assert_eq!(db.table.get().get(&2).unwrap(), "two");
    assert_eq!(db.table.get().get(&3).unwrap(), "three");

    // drop the db re-run init
    drop(db);
    let db = MigrationTest::init(Config::in_folder("tests/test_data/v1_log_format")).unwrap();

    // make sure the files we expect are there
    assert!(v2.exists());
    assert!(!v1.exists());

    // make sure the data we expect is there
    assert_eq!(db.table.get().len(), 4);
    assert_eq!(db.table.get().get(&0).unwrap(), "zero");
    assert_eq!(db.table.get().get(&1).unwrap(), "one");
    assert_eq!(db.table.get().get(&2).unwrap(), "two");
    assert_eq!(db.table.get().get(&3).unwrap(), "three");
}

/// this code will be run against v1 (<0.3), the resulting database will be stored in this
/// directory as a snapshot. The test will make a copy for a pretend migration, the copies will be
/// cleaned up.
#[test]
#[ignore]
fn generate_data() {
    let dir = "tests/test_data/v1_log_format";
    let mut db = MigrationTest::init(Config::in_folder(dir)).unwrap();
    db.table.insert(0, "zero".into()).unwrap();
    db.table.insert(1, "one".into()).unwrap();
    db.table.insert(2, "two".into()).unwrap();
    db.table.insert(3, "three".into()).unwrap();
}
