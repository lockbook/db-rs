use std::{
    env,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

use db_rs::{View, config::Config, views::hashmap::DbHashMap};

#[test]
fn reader_blocks_another_process_writer() {
    let config = Config::test();
    let mut db = DbHashMap::<String, u64>::init(&config).unwrap();
    let (mut worker, mut socket) = spawn_worker(&config);

    let read = db.read_tx().unwrap();
    start_and_expect_blocked(&mut socket);
    drop(read);

    finish_worker(&mut worker, &mut socket);
    db.write_tx().unwrap().end_tx(&mut db).unwrap();
    assert_eq!(db.read_tx().unwrap().get("child"), Some(&2));
}

#[test]
fn stale_reader_blocks_another_process_writer_after_snapshot() {
    let config = Config::test();
    let reader = DbHashMap::<String, u64>::init(&config).unwrap();
    let mut writer = DbHashMap::<String, u64>::init(&config).unwrap();
    let tx = writer.write_tx().unwrap();
    writer.insert("parent".into(), 1).unwrap();
    tx.end_tx(&mut writer).unwrap();
    writer.snapshot().unwrap();
    let (mut worker, mut socket) = spawn_worker(&config);

    let read = reader.read_tx().unwrap();
    start_and_expect_blocked(&mut socket);
    drop(read);

    finish_worker(&mut worker, &mut socket);
    writer.write_tx().unwrap().end_tx(&mut writer).unwrap();
    assert_eq!(writer.get("child"), Some(&2));
}

#[test]
fn writers_serialize_across_processes() {
    let config = Config::test();
    let mut db = DbHashMap::<String, u64>::init(&config).unwrap();
    let (mut worker, mut socket) = spawn_worker(&config);

    let write = db.write_tx().unwrap();
    db.insert("parent".into(), 1).unwrap();
    start_and_expect_blocked(&mut socket);
    write.end_tx(&mut db).unwrap();

    finish_worker(&mut worker, &mut socket);
    db.write_tx().unwrap().end_tx(&mut db).unwrap();
    let read = db.read_tx().unwrap();
    assert_eq!(read.get("parent"), Some(&1));
    assert_eq!(read.get("child"), Some(&2));
}

#[test]
fn writers_on_different_logs_serialize_across_processes() {
    let config = Config::test();
    let mut db = DbHashMap::<String, u64>::init(&config).unwrap();
    let (mut worker, mut socket) = spawn_worker(&config);
    let tx = db.write_tx().unwrap();
    db.insert("parent".into(), 1).unwrap();
    tx.end_tx(&mut db).unwrap();
    db.snapshot().unwrap();

    let write = db.write_tx().unwrap();
    start_and_expect_blocked(&mut socket);
    write.end_tx(&mut db).unwrap();

    finish_worker(&mut worker, &mut socket);
    db.write_tx().unwrap().end_tx(&mut db).unwrap();
    let read = db.read_tx().unwrap();
    assert_eq!(read.get("parent"), Some(&1));
    assert_eq!(read.get("child"), Some(&2));
}

#[test]
fn ipc_worker() {
    let Some(location) = env::var_os("DB_RS_IPC_LOCATION") else {
        return;
    };
    let address = env::var("DB_RS_IPC_ADDRESS").unwrap();
    let config = Config::default().log_location(location);
    let mut db = DbHashMap::<String, u64>::init(&config).unwrap();
    let mut socket = TcpStream::connect(address).unwrap();

    send(&mut socket, b'R');
    assert_eq!(receive(&mut socket), b'G');
    send(&mut socket, b'A');
    let write = db.write_tx().unwrap();
    send(&mut socket, b'L');
    db.insert("child".into(), 2).unwrap();
    write.end_tx(&mut db).unwrap();
    send(&mut socket, b'D');
}

fn spawn_worker(config: &Config) -> (Worker, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let child = Command::new(env::current_exe().unwrap())
        .args(["--exact", "ipc_worker", "--nocapture"])
        .env("DB_RS_IPC_LOCATION", &config.log_location)
        .env(
            "DB_RS_IPC_ADDRESS",
            listener.local_addr().unwrap().to_string(),
        )
        .spawn()
        .unwrap();
    let mut worker = Worker(Some(child));
    let deadline = Instant::now() + Duration::from_secs(5);

    let mut socket = loop {
        match listener.accept() {
            Ok((socket, _)) => break socket,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                assert!(worker.0.as_mut().unwrap().try_wait().unwrap().is_none());
                assert!(Instant::now() < deadline, "worker did not connect");
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("failed to accept worker: {error}"),
        }
    };
    socket.set_nonblocking(false).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    assert_eq!(receive(&mut socket), b'R');
    (worker, socket)
}

fn start_and_expect_blocked(socket: &mut TcpStream) {
    send(socket, b'G');
    assert_eq!(receive(socket), b'A');

    socket
        .set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let mut byte = [0];
    let result = socket.read_exact(&mut byte);
    assert!(
        matches!(result, Err(ref error) if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut)),
        "writer acquired the lock before the other transaction ended: {result:?}"
    );
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
}

fn finish_worker(worker: &mut Worker, socket: &mut TcpStream) {
    assert_eq!(receive(socket), b'L');
    assert_eq!(receive(socket), b'D');
    assert!(worker.0.take().unwrap().wait().unwrap().success());
}

fn send(socket: &mut TcpStream, byte: u8) {
    socket.write_all(&[byte]).unwrap();
}

fn receive(socket: &mut TcpStream) -> u8 {
    let mut byte = [0];
    socket.read_exact(&mut byte).unwrap();
    byte[0]
}

struct Worker(Option<Child>);

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
