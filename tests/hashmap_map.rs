use std::collections::HashMap;

use db_rs::{View, config::Config, views::hashmap_map::DbHashMapMap};

#[test]
fn round_trip() {
    let config = Config::test();
    {
        let mut db = DbHashMapMap::<u64, u64, String>::init(&config).unwrap();
        let mut tx = db.write_tx().unwrap();
        assert_eq!(tx.insert(1, 10, "old".into()).unwrap(), None);
        assert_eq!(
            tx.insert(1, 10, "new".into()).unwrap().as_deref(),
            Some("old")
        );
        tx.insert(1, 20, "removed".into()).unwrap();
        assert_eq!(tx.remove(&1, &20).unwrap().as_deref(), Some("removed"));
        assert_eq!(tx.remove(&1, &20).unwrap(), None);
        assert_eq!(tx.remove(&99, &20).unwrap(), None);
        tx.insert(2, 30, "reset".into()).unwrap();
        assert_eq!(
            tx.create_key(2).unwrap(),
            Some(HashMap::from([(30, "reset".into())]))
        );
        tx.insert(3, 40, "cleared".into()).unwrap();
        assert_eq!(
            tx.clear_key(&3).unwrap(),
            Some(HashMap::from([(40, "cleared".into())]))
        );
        assert_eq!(tx.clear_key(&3).unwrap(), None);
        tx.end_tx().unwrap();
    }

    let mut db = DbHashMapMap::<u64, u64, String>::init(&config).unwrap();
    assert_eq!(db.get(&1), Some(&HashMap::from([(10, "new".into())])));
    assert_eq!(db.get(&2), Some(&HashMap::new()));
    assert!(!db.contains_key(&3));
    assert_eq!(db.len(), 2);
    assert_eq!(db.iter().count(), 2);
    db.write_tx().unwrap().clear().unwrap();
    drop(db);

    let db = DbHashMapMap::<u64, u64, String>::init(&config).unwrap();
    assert!(db.is_empty());
}

#[test]
fn snapshot_preserves_entries_and_empty_groups() {
    let config = Config::test();
    {
        let mut db = DbHashMapMap::<u64, u64, String>::init(&config).unwrap();
        {
            let mut tx = db.write_tx().unwrap();
            tx.insert(1, 10, "ten".into()).unwrap();
            tx.insert(1, 20, "twenty".into()).unwrap();
            assert_eq!(tx.create_key(2).unwrap(), None);
            tx.insert(3, 30, "removed".into()).unwrap();
            tx.remove(&3, &30).unwrap();
        }
        db.snapshot().unwrap();
    }

    let db = DbHashMapMap::<u64, u64, String>::init(&config).unwrap();
    assert_eq!(
        db.get(&1),
        Some(&HashMap::from([(10, "ten".into()), (20, "twenty".into())]))
    );
    assert_eq!(db.get(&2), Some(&HashMap::new()));
    assert_eq!(db.get(&3), Some(&HashMap::new()));
    assert_eq!(db.len(), 3);
}
