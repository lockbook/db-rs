use db_rs::{Config, Db, List};
use db_rs_derive::Schema;
use std::fs;

#[derive(Schema)]
struct Schema {
    list1: List<String>,
    list2: List<String>,
    list3: List<String>,
}

#[test]
fn list_test() {
    let dir = "/tmp/j/";
    drop(fs::remove_dir_all(dir));
    let mut db = Schema::init(Config::in_folder(dir)).unwrap();
    db.list1.push("a".to_string()).unwrap();
    assert_eq!(db.list1.data(), ["a"]);

    let db = Schema::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.list1.data(), ["a"]);

    drop(fs::remove_dir_all(dir));
}

#[test]
fn list_test2() {
    let dir = "/tmp/k/";
    drop(fs::remove_dir_all(dir));
    let mut db = Schema::init(Config::in_folder(dir)).unwrap();
    db.list1.push("a".to_string()).unwrap();

    db.list2.push("b".to_string()).unwrap();
    db.list2.push("c".to_string()).unwrap();
    db.list2.push("d".to_string()).unwrap();

    db.list3.push("e".to_string()).unwrap();
    db.list3.push("f".to_string()).unwrap();
    db.list3.push("g".to_string()).unwrap();
    db.list3.push("h".to_string()).unwrap();
    db.list3.push("i".to_string()).unwrap();
    db.list3.push("j".to_string()).unwrap();

    assert_eq!(db.list1.data(), ["a"]);
    assert_eq!(db.list2.data(), ["b", "c", "d"]);
    assert_eq!(db.list3.data(), ["e", "f", "g", "h", "i", "j"]);

    let db = Schema::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.list1.data(), ["a"]);
    assert_eq!(db.list2.data(), ["b", "c", "d"]);
    assert_eq!(db.list3.data(), ["e", "f", "g", "h", "i", "j"]);
    drop(fs::remove_dir_all(dir));
}
#[test]
fn list_test3() {
    let dir = "/tmp/kk/";
    drop(fs::remove_dir_all(dir));
    let mut db = Schema::init(Config::in_folder(dir)).unwrap();
    db.list1.push("a".to_string()).unwrap();

    db.list2.push("b".to_string()).unwrap();
    db.list2.push("c".to_string()).unwrap();
    db.list2.push("d".to_string()).unwrap();
    db.list2.remove(1).unwrap();

    db.list3.push("e".to_string()).unwrap();
    db.list3.push("f".to_string()).unwrap();
    db.list3.push("g".to_string()).unwrap();
    db.list3.push("h".to_string()).unwrap();
    db.list3.push("i".to_string()).unwrap();
    db.list3.push("j".to_string()).unwrap();

    db.list3.pop().unwrap();
    db.list3.pop().unwrap();

    assert_eq!(db.list1.data(), ["a"]);
    assert_eq!(db.list2.data(), ["b", "d"]);
    assert_eq!(db.list3.data(), ["e", "f", "g", "h"]);

    let db = Schema::init(Config::in_folder(dir)).unwrap();
    assert_eq!(db.list1.data(), ["a"]);
    assert_eq!(db.list2.data(), ["b", "d"]);
    assert_eq!(db.list3.data(), ["e", "f", "g", "h"]);
    drop(fs::remove_dir_all(dir));
}
