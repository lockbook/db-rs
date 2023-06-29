// cargo test -F clone
#[cfg(feature = "clone")]
mod clone_feature {
    use std::fs;

    use db_rs::*;
    use db_rs_derive::Schema;

    #[derive(Schema, Clone)]
    struct CloneFT {
        table1: LookupTable<u8, String>,
        table2: Single<String>,
        table3: List<String>,
        table4: LookupSet<u8, String>,
        table5: LookupList<u8, String>,
    }

    #[test]
    fn test() {
        let dir = "/tmp/o/";
        drop(fs::remove_dir_all(dir));
        let mut db = CloneFT::init(Config::in_folder(dir)).unwrap();
        db.table1.insert(5, "test".to_string()).unwrap();
        let db2 = db.clone();
        assert_eq!(db2.table1.get().get(&5).unwrap(), "test");
    }
}
