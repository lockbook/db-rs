use std::{
    fs,
    io::ErrorKind,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use db_rs::{config::Config, db::Db, views::hashmap::SHashMap};

#[test]
fn snapshot() {
    let config = Config::default().log_location(test_directory());
    let mut db: Db<SHashMap<String, u64>> = Db::init(config).unwrap();

    let view = db.begin_tx().unwrap();
    view.insert("one".into(), 1).unwrap();
    view.insert("two".into(), 2).unwrap();
    view.insert("three".into(), 3).unwrap();
    db.end_tx().unwrap();
}

fn test_directory() -> PathBuf {
    let mut id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    loop {
        let path = std::env::temp_dir().join(format!("db-rs-{id}"));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => id += 1,
            Err(error) => panic!("failed to create test directory: {error}"),
        }
    }
}
