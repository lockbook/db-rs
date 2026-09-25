use std::sync::mpsc::TryRecvError;

use db_rs::{
    View,
    config::Config,
    errors::Error,
    log::{Log, Notification},
    views::{
        composite_view::{Composite, Schema},
        hashmap::DbHashMap,
        hashmap_map::DbHashMapMap,
        hashmap_set::DbHashMapSet,
        option::DbOption,
        vec::DbVec,
    },
};

#[cfg(target_family = "wasm")]
use wasm_bindgen_test::wasm_bindgen_test;

#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn transactions_and_notifications() {
    let mut db = DbHashMap::<String, u64>::init(&Config::in_memory()).unwrap();
    let notifications = db.log_mut().notifications();

    assert_eq!(db.write_tx().unwrap().end_tx(&mut db).unwrap(), 0);
    assert_eq!(notifications.try_recv(), Err(TryRecvError::Empty));

    let tx = db.write_tx().unwrap();
    assert_eq!(tx.seq_no, 1);
    db.insert("key".into(), 42).unwrap();
    assert_eq!(tx.end_tx(&mut db).unwrap(), 1);
    assert_eq!(db.last_modified(), 1);
    assert_eq!(db.read_tx().unwrap().get("key"), Some(&42));
    assert_eq!(
        notifications.try_recv().unwrap(),
        Notification {
            local: true,
            seq_no: 1
        }
    );

    db.snapshot().unwrap();
    assert_eq!(db.last_modified(), 1);
    assert_eq!(notifications.try_recv(), Err(TryRecvError::Empty));

    let tx = db.write_tx().unwrap();
    db.insert("key".into(), 43).unwrap();
    assert_eq!(tx.end_tx(&mut db).unwrap(), 2);
    assert_eq!(db.read_tx().unwrap().get("key"), Some(&43));
    assert_eq!(
        notifications.try_recv().unwrap(),
        Notification {
            local: true,
            seq_no: 2
        }
    );
}

#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn ignores_log_location_and_does_not_persist() {
    let config = Config::in_memory().log_location("\0");
    let mut db = DbHashMap::<u64, u64>::init(&config).unwrap();
    let tx = db.write_tx().unwrap();
    db.insert(1, 42).unwrap();
    tx.end_tx(&mut db).unwrap();
    db.snapshot().unwrap();

    assert_eq!(Log::find_latest(&config).unwrap(), None);
    assert!(db.log_mut().get_bytes().unwrap().is_empty());

    let fresh = DbHashMap::<u64, u64>::init(&config).unwrap();
    assert!(fresh.read_tx().unwrap().is_empty());
    assert_eq!(fresh.last_modified(), 0);
    assert_eq!(db.get(&1), Some(&42));
}

#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn persistent_init_does_not_fall_back_to_memory() {
    let config = Config::default().log_location("\0");
    assert!(DbHashMap::<u64, u64>::init(&config).is_err());
}

#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn dropping_dirty_write_requires_reinitializing() {
    let mut db = DbHashMap::<u64, u64>::init(&Config::in_memory()).unwrap();
    let tx = db.write_tx().unwrap();
    db.insert(1, 42).unwrap();
    drop(tx);

    assert_eq!(db.last_modified(), 0);
    assert_eq!(db.get(&1), Some(&42));
    assert!(matches!(db.write_tx(), Err(Error::Poisoned)));
    assert!(matches!(db.write_tx(), Err(Error::Poisoned)));
    assert_eq!(db.last_modified(), 0);

    let mut fresh = DbHashMap::<u64, u64>::init(&Config::in_memory()).unwrap();
    assert!(fresh.is_empty());
    assert_eq!(fresh.write_tx().unwrap().end_tx(&mut fresh).unwrap(), 0);
}

#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn dropping_empty_write_is_harmless() {
    let mut db = DbHashMap::<u64, u64>::init(&Config::in_memory()).unwrap();
    drop(db.write_tx().unwrap());

    let tx = db.write_tx().unwrap();
    db.insert(1, 42).unwrap();
    assert_eq!(tx.end_tx(&mut db).unwrap(), 1);
    assert_eq!(db.read_tx().unwrap().get(&1), Some(&42));
}

#[derive(Default)]
struct TestSchema {
    map: DbHashMap<u64, u64>,
    option: DbOption<u64>,
    vec: DbVec<u64>,
    sets: DbHashMapSet<u64, u64>,
    maps: DbHashMapMap<u64, u64, u64>,
}

impl Schema for TestSchema {
    fn views_mut(&mut self) -> impl AsMut<[&mut dyn View]> {
        let views: [&mut dyn View; 5] = [
            &mut self.map,
            &mut self.option,
            &mut self.vec,
            &mut self.sets,
            &mut self.maps,
        ];
        views
    }
}

#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn dropped_composite_edits_poison_the_database() {
    let mut db = Composite::<TestSchema>::init(&Config::in_memory()).unwrap();
    let tx = db.write_tx().unwrap();
    db.schema.map.insert(1, 42).unwrap();
    db.schema.option.replace(43).unwrap();
    drop(tx);

    assert!(matches!(db.write_tx(), Err(Error::Poisoned)));
    assert!(matches!(db.write_tx(), Err(Error::Poisoned)));
    assert!(matches!(db.snapshot(), Err(Error::Poisoned)));
}

#[cfg_attr(target_family = "wasm", wasm_bindgen_test)]
#[cfg_attr(not(target_family = "wasm"), test)]
fn composite_snapshot_round_trip() {
    let config = Config::in_memory();
    let mut db = Composite::<TestSchema>::init(&config).unwrap();
    let tx = db.write_tx().unwrap();
    db.schema.map.insert(1, 2).unwrap();
    db.schema.option.replace(3).unwrap();
    db.schema.vec.push(4).unwrap();
    db.schema.sets.insert(5, 6).unwrap();
    db.schema.maps.insert(7, 8, 9).unwrap();
    tx.end_tx(&mut db).unwrap();
    let tx = db.write_tx().unwrap();
    db.schema.option.replace(10).unwrap();
    tx.end_tx(&mut db).unwrap();

    let bytes = db.generate_snapshot().unwrap();
    let mut restored = Composite::<TestSchema>::init(&config).unwrap();
    restored.handle_events(db.last_modified(), &bytes).unwrap();
    let read = restored.read_tx().unwrap();
    assert_eq!(read.schema.map.get(&1), Some(&2));
    assert_eq!(read.schema.option.as_ref(), Some(&10));
    assert_eq!(read.schema.vec.as_slice(), [4]);
    assert!(read.schema.sets.get(&5).unwrap().contains(&6));
    assert_eq!(read.schema.maps.get(&7).unwrap().get(&8), Some(&9));
    assert_eq!(read.schema.map.last_modified(), 1);
    assert_eq!(read.schema.option.last_modified(), 2);
}
