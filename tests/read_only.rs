use std::{fs, io::ErrorKind, path::PathBuf, sync::mpsc, thread, time::Duration};

use db_rs::{View, config::Config, errors::Error, log::Log, views::hashmap::DbHashMap};

type Map = DbHashMap<String, u64>;

#[test]
fn read_only_sees_committed_writes() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();
    write(&mut db, "one", 1);
    db.snapshot().unwrap();
    write(&mut db, "two", 2);

    let reader = Map::init(&read_only(&config)).unwrap();
    assert_eq!(reader.get("one"), Some(&1));
    assert_eq!(reader.get("two"), Some(&2));
}

#[test]
fn read_only_stops_before_an_incomplete_write() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();
    write(&mut db, "one", 1);
    let committed = latest_log_len(&config);
    write(&mut db, "two", 2);
    drop(db);
    let log = fs::read(latest_log(&config)).unwrap();

    for cut in committed + 1..log.len() {
        let config = Config::test();
        fs::write(config.log_location.join("db.0.log"), &log[..cut]).unwrap();

        let mut reader = Map::init(&read_only(&config)).unwrap();
        assert_eq!(reader.get("one"), Some(&1));
        assert_eq!(reader.get("two"), None);
        assert_eq!(reader.log_mut().take_incomplete_write(), None);
        assert_eq!(latest_log_len(&config), cut);
    }
}

#[test]
fn read_only_refuses_to_write() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();
    write(&mut db, "one", 1);
    let len = latest_log_len(&config);

    let mut reader = Map::init(&read_only(&config)).unwrap();
    assert!(matches!(reader.write_tx(), Err(Error::ReadOnly)));
    assert!(matches!(reader.snapshot(), Err(Error::ReadOnly)));
    assert_eq!(reader.get("one"), Some(&1));
    assert_eq!(latest_log_len(&config), len);
}

#[test]
fn read_only_creates_nothing() {
    let config = Config::test();

    let result = Map::init(&read_only(&config));
    assert!(matches!(result, Err(Error::Io(ref error)) if error.kind() == ErrorKind::NotFound));
    assert_eq!(fs::read_dir(&config.log_location).unwrap().count(), 0);
}

#[test]
fn read_only_does_not_wait_for_a_writer() {
    let config = Config::test();
    let mut db = Map::init(&config).unwrap();
    write(&mut db, "one", 1);
    let tx = db.write_tx().unwrap();

    let reader_config = read_only(&config);
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let read = Map::init(&reader_config)
            .map(|reader| reader.get("one").copied())
            .map_err(|error| error.to_string());
        let _ = sender.send(read);
    });
    let read = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("read-only open waited for the writer");
    assert_eq!(read, Ok(Some(1)));

    tx.end_tx(&mut db).unwrap();
}

fn read_only(config: &Config) -> Config {
    Config::default()
        .log_location(config.log_location.clone())
        .read_only()
}

fn write(db: &mut Map, key: &str, value: u64) {
    let tx = db.write_tx().unwrap();
    db.insert(key.into(), value).unwrap();
    tx.end_tx(db).unwrap();
}

fn latest_log(config: &Config) -> PathBuf {
    Log::find_latest(config).unwrap().unwrap()
}

fn latest_log_len(config: &Config) -> usize {
    fs::metadata(latest_log(config)).unwrap().len() as usize
}
