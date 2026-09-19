use db_rs::{View, config::Config, errors::Error, views::vec::DbVec};

#[test]
fn round_trip() {
    let config = Config::test();
    {
        let mut db = DbVec::<u64>::init(&config).unwrap();
        let mut tx = db.write_tx().unwrap();
        assert_eq!(tx.pop().unwrap(), None);
        tx.push(1).unwrap();
        tx.push(3).unwrap();
        tx.insert(1, 2).unwrap();
        tx.insert(3, 4).unwrap();
        assert_eq!(tx.remove(0).unwrap(), 1);
        assert_eq!(tx.pop().unwrap(), Some(4));
        tx.end_tx().unwrap();
    }

    let mut db = DbVec::<u64>::init(&config).unwrap();
    assert_eq!(db.as_slice(), &[2, 3]);
    assert_eq!(db.len(), 2);
    assert_eq!(db.get(1), Some(&3));
    assert_eq!(db.get(2), None);
    assert_eq!(db.iter().copied().collect::<Vec<_>>(), [2, 3]);
    db.write_tx().unwrap().clear().unwrap();
    drop(db);

    let db = DbVec::<u64>::init(&config).unwrap();
    assert!(db.is_empty());
}

#[test]
fn snapshots_preserve_order_during_reopen_and_catch_up() {
    let config = Config::test();
    let mut writer = DbVec::<u64>::init(&config).unwrap();
    let mut stale = DbVec::<u64>::init(&config).unwrap();

    {
        let mut tx = writer.write_tx().unwrap();
        tx.push(1).unwrap();
        tx.push(2).unwrap();
    }
    writer.snapshot().unwrap();
    writer.write_tx().unwrap().remove(0).unwrap();
    writer.snapshot().unwrap();
    writer.write_tx().unwrap().push(3).unwrap();

    stale.write_tx().unwrap().end_tx().unwrap();
    assert_eq!(stale.as_slice(), &[2, 3]);
    let fresh = DbVec::<u64>::init(&config).unwrap();
    assert_eq!(fresh.as_slice(), &[2, 3]);

    writer.write_tx().unwrap().clear().unwrap();
    writer.snapshot().unwrap();
    let fresh = DbVec::<u64>::init(&config).unwrap();
    assert!(fresh.is_empty());
}

#[test]
fn invalid_indices_leave_the_view_and_pending_events_unchanged() {
    let mut view = DbVec::new();
    view.push(42u64).unwrap();
    view.take_pending();

    assert!(matches!(
        view.insert(2, 99),
        Err(Error::IndexOutOfBounds { index: 2, len: 1 })
    ));
    assert!(matches!(
        view.remove(1),
        Err(Error::IndexOutOfBounds { index: 1, len: 1 })
    ));
    assert_eq!(view.as_slice(), &[42]);
    assert!(view.take_pending().is_empty());
}
