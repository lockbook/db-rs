use db_rs::{View, config::Config, views::option::DbOption};

#[test]
fn round_trip() {
    let config = Config::test();
    {
        let mut db = DbOption::<String>::init(&config).unwrap();
        assert!(db.read_tx().unwrap().is_none());
        {
            let tx = db.write_tx().unwrap();
            assert_eq!(db.replace("first".into()).unwrap(), None);
            assert_eq!(
                db.replace("second".into()).unwrap().as_deref(),
                Some("first")
            );
            tx.end_tx(&mut db).unwrap();
        }
    }

    let mut db = DbOption::<String>::init(&config).unwrap();
    assert_eq!(
        db.read_tx().unwrap().as_ref().map(String::as_str),
        Some("second")
    );
    let tx = db.write_tx().unwrap();
    assert_eq!(db.take().unwrap().as_deref(), Some("second"));
    tx.end_tx(&mut db).unwrap();
    drop(db);

    let db = DbOption::<String>::init(&config).unwrap();
    assert!(db.read_tx().unwrap().is_none());
}

#[test]
fn snapshots_preserve_some_and_none() {
    for clear in [false, true] {
        let config = Config::test();
        {
            let mut db = DbOption::<u64>::init(&config).unwrap();
            let tx = db.write_tx().unwrap();
            db.replace(42).unwrap();
            tx.end_tx(&mut db).unwrap();
            if clear {
                let tx = db.write_tx().unwrap();
                db.take().unwrap();
                tx.end_tx(&mut db).unwrap();
            }
            db.snapshot().unwrap();
        }

        let db = DbOption::<u64>::init(&config).unwrap();
        let expected = if clear { None } else { Some(&42) };
        assert_eq!(db.read_tx().unwrap().as_ref(), expected);
    }
}
