use std::collections::HashSet;

use db_rs::{View, config::Config, views::hashmap_set::DbHashMapSet};

#[test]
fn round_trip() {
    let config = Config::test();
    {
        let mut db = DbHashMapSet::<u64, u64>::init(&config).unwrap();
        let mut tx = db.write_tx().unwrap();
        assert!(tx.insert(1, 10).unwrap());
        assert!(!tx.insert(1, 10).unwrap());
        tx.insert(1, 20).unwrap();
        assert!(tx.remove(&1, &10).unwrap());
        assert!(!tx.remove(&1, &10).unwrap());
        assert!(!tx.remove(&99, &10).unwrap());
        tx.insert(2, 30).unwrap();
        assert_eq!(tx.create_key(2).unwrap(), Some(HashSet::from([30])));
        tx.insert(3, 40).unwrap();
        assert_eq!(tx.clear_key(&3).unwrap(), Some(HashSet::from([40])));
        assert_eq!(tx.clear_key(&3).unwrap(), None);
        tx.end_tx().unwrap();
    }

    let mut db = DbHashMapSet::<u64, u64>::init(&config).unwrap();
    assert_eq!(db.get(&1), Some(&HashSet::from([20])));
    assert_eq!(db.get(&2), Some(&HashSet::new()));
    assert!(!db.contains_key(&3));
    assert_eq!(db.len(), 2);
    assert_eq!(db.iter().count(), 2);
    db.write_tx().unwrap().clear().unwrap();
    drop(db);

    let db = DbHashMapSet::<u64, u64>::init(&config).unwrap();
    assert!(db.is_empty());
}

#[test]
fn snapshot_preserves_members_and_empty_groups() {
    let config = Config::test();
    {
        let mut db = DbHashMapSet::<u64, u64>::init(&config).unwrap();
        {
            let mut tx = db.write_tx().unwrap();
            tx.insert(1, 10).unwrap();
            tx.insert(1, 20).unwrap();
            assert_eq!(tx.create_key(2).unwrap(), None);
            tx.insert(3, 30).unwrap();
            tx.remove(&3, &30).unwrap();
        }
        db.snapshot().unwrap();
    }

    let db = DbHashMapSet::<u64, u64>::init(&config).unwrap();
    assert_eq!(db.get(&1), Some(&HashSet::from([10, 20])));
    assert_eq!(db.get(&2), Some(&HashSet::new()));
    assert_eq!(db.get(&3), Some(&HashSet::new()));
    assert_eq!(db.len(), 3);
}
