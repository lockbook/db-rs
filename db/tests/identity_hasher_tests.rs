use db_rs::hasher::{UuidIdentityHasher, UuidIdentityHasherBuilder};
use db_rs::utils::random_test_dir;
use db_rs::Db;
use db_rs::{Config, LookupList, LookupTable};
use db_rs_derive::Schema;
use std::fs;
use std::hash::{BuildHasher, Hash, Hasher};
use uuid::Uuid;

#[derive(Schema)]
pub struct UuidLookupSchema {
    table: LookupTable<Uuid, String, UuidIdentityHasherBuilder>,
    list: LookupList<Uuid, String, UuidIdentityHasherBuilder>,
}

#[test]
fn identity_hasher_matches_uuid_bytes() {
    let id = Uuid::new_v4();
    let mut hasher = UuidIdentityHasher::default();
    id.hash(&mut hasher);

    let expected = u64::from_ne_bytes(id.as_bytes()[..8].try_into().unwrap());
    assert_eq!(hasher.finish(), expected);
}

#[test]
fn build_hasher_hash_one_matches() {
    let id = Uuid::new_v4();
    let expected = u64::from_ne_bytes(id.as_bytes()[..8].try_into().unwrap());
    assert_eq!(UuidIdentityHasherBuilder.hash_one(id), expected);
}

#[test]
fn lookup_table_with_identity_hasher() {
    let dir = &random_test_dir();
    drop(fs::remove_dir_all(dir));

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let mut db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table.insert(id1, "first".to_string()).unwrap();
    db.table.insert(id2, "second".to_string()).unwrap();
    db.table.insert(id1, "first-updated".to_string()).unwrap();
    drop(db);

    let db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.table.get().get(&id1).unwrap(), "first-updated");
    assert_eq!(db.table.get().get(&id2).unwrap(), "second");
    assert_eq!(db.table.get().len(), 2);

    drop(fs::remove_dir_all(dir));
}

#[test]
fn lookup_table_remove_and_clear_with_identity_hasher() {
    let dir = &random_test_dir();
    drop(fs::remove_dir_all(dir));

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    let mut db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table.insert(id1, "one".to_string()).unwrap();
    db.table.insert(id2, "two".to_string()).unwrap();
    db.table.insert(id3, "three".to_string()).unwrap();
    db.table.remove(&id2).unwrap();
    drop(db);

    let db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.table.get().get(&id1).unwrap(), "one");
    assert!(db.table.get().get(&id2).is_none());
    assert_eq!(db.table.get().get(&id3).unwrap(), "three");

    drop(db);
    let mut db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    db.table.clear().unwrap();
    drop(db);

    let db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    assert!(db.table.get().is_empty());

    drop(fs::remove_dir_all(dir));
}

#[test]
fn lookup_list_with_identity_hasher() {
    let dir = &random_test_dir();
    drop(fs::remove_dir_all(dir));

    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let mut db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    db.list.push(id1, "a".to_string()).unwrap();
    db.list.push(id1, "b".to_string()).unwrap();
    db.list.push(id2, "c".to_string()).unwrap();
    drop(db);

    let db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    assert_eq!(
        db.list.get().get(&id1).unwrap(),
        &vec!["a".to_string(), "b".to_string()]
    );
    assert_eq!(db.list.get().get(&id2).unwrap(), &vec!["c".to_string()]);

    drop(fs::remove_dir_all(dir));
}

#[test]
fn lookup_table_compacts_with_identity_hasher() {
    let dir = &random_test_dir();
    drop(fs::remove_dir_all(dir));

    let ids: Vec<Uuid> = (0..50).map(|_| Uuid::new_v4()).collect();

    let mut db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    for (i, id) in ids.iter().enumerate() {
        db.table.insert(*id, format!("value-{i}")).unwrap();
    }
    db.compact_log().unwrap();
    drop(db);

    let db = UuidLookupSchema::init(Config::in_folder(dir)).unwrap();
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(db.table.get().get(id).unwrap(), &format!("value-{i}"));
    }

    drop(fs::remove_dir_all(dir));
}
