use db_rs::{Config, Db, LookupTable};
use db_rs_derive::Schema;
use std::fs::remove_dir_all;

#[derive(Schema)]
struct TxTest {
    table: LookupTable<u8, String>,
}

#[test]
fn test() {
    let dir = "/tmp/g";
    drop(remove_dir_all(dir));

    let mut db = TxTest::init(Config::in_folder(dir)).unwrap();
    let tx = db.begin_transaction();
    db.table.insert(43, "test".to_string()).unwrap();

    {
        let db = TxTest::init(Config::in_folder(dir)).unwrap();
        assert_eq!(db.table.get(&43), None);
    }
    drop(tx);
    {
        let db = TxTest::init(Config::in_folder(dir)).unwrap();
        assert_eq!(db.table.get(&43), Some(&"test".to_string()));
    }

    drop(remove_dir_all(dir));
}
