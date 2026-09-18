use db_rs::{View, config::Config, views::option::DbOption};

#[test]
fn round_trip() {
    let config = Config::test();
    {
        let mut db = DbOption::<String>::init(&config).unwrap();
        assert!(db.read_tx().unwrap().is_none());
        {
            let mut view = db.write_tx().unwrap();
            assert_eq!(view.replace("first".into()).unwrap(), None);
            assert_eq!(
                view.replace("second".into()).unwrap().as_deref(),
                Some("first")
            );
        }
        db.flush_pending().unwrap();
    }

    let mut db = DbOption::<String>::init(&config).unwrap();
    assert_eq!(
        db.read_tx().unwrap().as_ref().map(String::as_str),
        Some("second")
    );
    assert_eq!(
        db.write_tx().unwrap().take().unwrap().as_deref(),
        Some("second")
    );
    db.flush_pending().unwrap();
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
            db.write_tx().unwrap().replace(42).unwrap();
            db.flush_pending().unwrap();
            if clear {
                db.write_tx().unwrap().take().unwrap();
                db.flush_pending().unwrap();
            }
            db.snapshot().unwrap();
        }

        let db = DbOption::<u64>::init(&config).unwrap();
        let expected = if clear { None } else { Some(&42) };
        assert_eq!(db.read_tx().unwrap().as_ref(), expected);
    }
}
