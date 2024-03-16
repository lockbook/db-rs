// test that two concorrent processes don't destroy the log and take turns
// test that they update each other's indexes

use std::{
    fs,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use db_rs::{BackgroundCompacter, CancelSig, Config, Db, Single};
use db_rs_derive::Schema;
use notify::Watcher;

#[derive(Schema)]
pub struct SingleSchema {
    table1: Single<u8>,
    table2: Single<String>,
    table3: Single<u32>,
    table4: Single<Vec<u8>>,
}

#[test]
fn test_events() {
    let dir = "/tmp/s/";

    drop(fs::remove_dir_all(dir));
    let mut config = Config::in_folder(dir);
    config.cooperative = true;
    let db = SingleSchema::init(config).unwrap();
    let config = db.config().unwrap();
    let db = Arc::new(Mutex::new(db));

    let mut watcher = notify::recommended_watcher(|res| {
        println!("{:?}", res);
    })
    .unwrap();

    watcher
        .watch(&config.db_location().unwrap(), notify::RecursiveMode::NonRecursive)
        .unwrap();

    thread::sleep(Duration::from_secs(3));

    db.begin_compacter(Duration::from_secs(1), CancelSig::default());

    thread::sleep(Duration::from_secs(3));
}
