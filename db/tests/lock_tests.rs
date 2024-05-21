use db_rs::{Config, Db, List};
use db_rs_derive::Schema;
use std::{
    fs, thread,
    time::{Duration, Instant},
};

#[derive(Schema)]
struct Schema {
    list1: List<String>,
    list2: List<String>,
    list3: List<String>,
}

#[test]
fn locks() {
    let dir = "/tmp/lock/";
    drop(fs::remove_dir_all(dir));
    let db = Schema::init(Config::in_folder(dir)).unwrap();
    assert!(Schema::init(Config::in_folder(dir)).is_err());
    drop(db);
    drop(fs::remove_dir_all(dir));
}

#[test]
fn no_locks() {
    let dir = "/tmp/lock2/";
    drop(fs::remove_dir_all(dir));
    let mut config = Config::in_folder(dir);
    config.fs_locks = false;

    let db = Schema::init(config.clone()).unwrap();
    assert!(Schema::init(config).is_ok());

    drop(db);
    drop(fs::remove_dir_all(dir));
}

#[test]
fn locks_blocks() {
    let dir = "/tmp/lock3/";
    drop(fs::remove_dir_all(dir));
    let mut config = Config::in_folder(dir);
    config.fs_locks_block = true;

    let db = Schema::init(config.clone()).unwrap();

    let block_time = thread::spawn(|| {
        let start = Instant::now();
        assert!(Schema::init(config).is_ok());
        start.elapsed().as_millis()
    });

    thread::sleep(Duration::from_millis(250));
    drop(db);
    let block_time = block_time.join().unwrap();
    assert!(block_time > 250);

    drop(fs::remove_dir_all(dir));
}
