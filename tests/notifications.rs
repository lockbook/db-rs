use std::sync::mpsc::TryRecvError;

use db_rs::{View, config::Config, log::Notification, views::hashmap::DbHashMap};

#[test]
fn notifies_local_writes_and_catch_up_across_snapshots() {
    let config = Config::test();
    let mut writer = DbHashMap::<String, u64>::init(&config).unwrap();
    let mut reader = DbHashMap::<String, u64>::init(&config).unwrap();
    let writes = writer.log_mut().notifications();
    let replays = reader.log_mut().notifications();

    for seq_no in 1..=2 {
        let tx = writer.write_tx().unwrap();
        writer.insert("key".into(), seq_no).unwrap();
        tx.end_tx(&mut writer).unwrap();
        writer.snapshot().unwrap();
    }

    for seq_no in 1..=2 {
        assert_eq!(
            writes.try_recv().unwrap(),
            Notification {
                local: true,
                seq_no
            }
        );
    }
    assert_eq!(writes.try_recv(), Err(TryRecvError::Empty));
    assert_eq!(replays.try_recv(), Err(TryRecvError::Empty));

    reader.write_tx().unwrap().end_tx(&mut reader).unwrap();
    assert_eq!(
        replays.try_recv().unwrap(),
        Notification {
            local: false,
            seq_no: 2
        }
    );
    assert_eq!(reader.get("key"), Some(&2));

    reader.write_tx().unwrap().end_tx(&mut reader).unwrap();
    assert_eq!(replays.try_recv(), Err(TryRecvError::Empty));

    let tx = reader.write_tx().unwrap();
    reader.insert("key".into(), 3).unwrap();
    tx.end_tx(&mut reader).unwrap();
    assert_eq!(
        replays.try_recv().unwrap(),
        Notification {
            local: true,
            seq_no: 3
        }
    );
}

#[test]
fn disconnected_receiver_does_not_prevent_writes() {
    let config = Config::test();
    let mut db = DbHashMap::<String, u64>::init(&config).unwrap();
    drop(db.log_mut().notifications());
    let tx = db.write_tx().unwrap();
    db.insert("key".into(), 1).unwrap();
    tx.end_tx(&mut db).unwrap();
    assert_eq!(db.last_modified(), 1);
}
