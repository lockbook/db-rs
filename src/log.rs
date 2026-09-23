use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
};

use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    errors::{Error, Result},
    guard::Lock,
    payload_buffer::PayloadBuffer,
};

#[derive(Serialize, Deserialize)]
pub(crate) enum LogEntry<'a> {
    Events { seq_no: u64, payload: &'a [u8] },
    Snapshot,
}

pub(crate) fn head_entry<'a>(remaining: &mut &'a [u8]) -> Result<Option<LogEntry<'a>>> {
    let mut rest = *remaining;
    let Some(body) = PayloadBuffer::head_payload(&mut rest)? else {
        return Ok(None);
    };
    let (entry, consumed) =
        bincode::serde::borrow_decode_from_slice(body, bincode::config::standard())?;
    if consumed != body.len() {
        return Err(Error::TrailingBytes {
            remaining_bytes: body.len() - consumed,
        });
    }
    *remaining = rest;
    Ok(Some(entry))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Notification {
    pub local: bool,
    pub seq_no: u64,
}

pub struct Log {
    file: Option<File>,
    path: PathBuf,
    directory: PathBuf,
    pub(crate) seq_no: u64,
    pub(crate) poisoned: bool,
    notifications: Option<Sender<Notification>>,
}

impl Log {
    pub fn init(config: &Config) -> io::Result<Self> {
        let directory = if config.log_location.is_empty() {
            PathBuf::from(".")
        } else {
            config.log_location.clone()
        };
        let mut log = Self {
            file: None,
            path: PathBuf::new(),
            directory,
            seq_no: 0,
            poisoned: false,
            notifications: None,
        };
        if !config.in_memory {
            let candidate = Self::find_latest(config)?;
            let create = candidate.is_none();
            let path = candidate.unwrap_or_else(|| log.directory.join("db.0.log"));
            let file = OpenOptions::new()
                .read(true)
                .append(true)
                .create(create)
                .open(&path)?;
            log.file = Some(file);
            log.path = path;
        }
        Ok(log)
    }

    pub fn notifications(&mut self) -> Receiver<Notification> {
        let (sender, receiver) = mpsc::channel();
        self.notifications = Some(sender);
        receiver
    }

    pub(crate) fn notify(&self, local: bool) {
        if let Some(sender) = &self.notifications {
            let _ = sender.send(Notification {
                local,
                seq_no: self.seq_no,
            });
        }
    }

    pub fn find_latest(config: &Config) -> io::Result<Option<PathBuf>> {
        if config.in_memory {
            return Ok(None);
        }
        let directory = if config.log_location.is_empty() {
            Path::new(".")
        } else {
            &config.log_location
        };
        let mut latest: Option<(u64, fs::DirEntry)> = None;

        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let Some(sequence) = Self::log_sequence(&entry.file_name()) else {
                continue;
            };
            if latest
                .as_ref()
                .is_none_or(|(current, _)| sequence > *current)
            {
                latest = Some((sequence, entry));
            }
        }

        Ok(latest.map(|(_, entry)| entry.path()))
    }

    fn log_sequence(name: &std::ffi::OsStr) -> Option<u64> {
        name.to_str()?
            .strip_prefix("db.")?
            .strip_suffix(".log")?
            .parse()
            .ok()
    }

    pub(crate) fn find_next(&self) -> io::Result<Option<PathBuf>> {
        let current = Self::log_sequence(self.path.file_name().expect("log path has a file name"))
            .expect("active log has a numbered file name");
        let mut next: Option<(u64, fs::DirEntry)> = None;

        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let Some(sequence) = Self::log_sequence(&entry.file_name()) else {
                continue;
            };
            if sequence <= current {
                continue;
            }
            if next
                .as_ref()
                .is_none_or(|(candidate, _)| sequence < *candidate)
            {
                next = Some((sequence, entry));
            }
        }

        Ok(next.map(|(_, entry)| entry.path()))
    }

    pub(crate) fn switch_to(&mut self, path: PathBuf) -> io::Result<()> {
        let file = OpenOptions::new().read(true).append(true).open(&path)?;
        self.file = Some(file);
        self.path = path;
        Ok(())
    }

    pub(crate) fn append(&mut self, entry: LogEntry<'_>) -> Result<()> {
        if let Some(file) = &mut self.file {
            Self::append_to(file, entry)?;
        }
        Ok(())
    }

    fn append_to(file: &mut File, entry: LogEntry<'_>) -> Result<()> {
        let mut buffer = PayloadBuffer::default();
        buffer.push_encoded(&entry)?;
        file.write_all(&buffer.bytes)?;
        file.sync_all()?;
        Ok(())
    }

    pub(crate) fn prepare_snapshot(&self, seq_no: u64, payload: &[u8]) -> Result<Option<PathBuf>> {
        if self.file.is_none() {
            return Ok(None);
        }
        let path = self.directory.join(format!("db.{seq_no}.log"));
        if path == self.path {
            return Ok(None);
        }

        let temporary_path = self.directory.join(format!("db.{seq_no}.tmp"));
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temporary_path)?;
            Self::append_to(&mut file, LogEntry::Events { seq_no, payload })?;
        }
        Ok(Some(path))
    }

    pub(crate) fn publish_snapshot(&self, path: &Path) -> io::Result<()> {
        fs::rename(path.with_extension("tmp"), path)
    }

    pub fn get_bytes(&mut self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        if let Some(file) = &mut self.file {
            file.read_to_end(&mut bytes)?;
        }
        Ok(bytes)
    }

    pub(crate) fn read_lock(&self) -> io::Result<Lock> {
        if self.file.is_none() {
            return Ok(Lock(None));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.directory.join("db.lock"))?;
        file.lock_shared()?;
        Ok(Lock(Some(file)))
    }

    pub(crate) fn write_lock(&self) -> io::Result<Lock> {
        if self.file.is_none() {
            return Ok(Lock(None));
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.directory.join("db.lock"))?;
        file.lock()?;
        Ok(Lock(Some(file)))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File, OpenOptions, TryLockError};

    use super::{Log, LogEntry, head_entry};
    use crate::config::Config;

    #[test]
    fn init_creates_initial_log() {
        let config = Config::test();

        let log = Log::init(&config).unwrap();

        assert!(config.log_location.join("db.0.log").is_file());
        drop(log);
    }

    #[test]
    fn locks_use_a_separate_database_file() {
        let config = Config::test();
        let log = Log::init(&config).unwrap();
        let lock = log.write_lock().unwrap();

        let database_lock = File::open(config.log_location.join("db.lock")).unwrap();
        assert!(matches!(
            database_lock.try_lock_shared(),
            Err(TryLockError::WouldBlock)
        ));

        let data = OpenOptions::new()
            .read(true)
            .write(true)
            .open(config.log_location.join("db.0.log"))
            .unwrap();
        data.try_lock().unwrap();
        data.unlock().unwrap();
        lock.unlock().unwrap();
        database_lock.try_lock_shared().unwrap();
        assert!(config.log_location.join("db.lock").is_file());
    }

    #[test]
    fn appended_events_can_be_read() {
        let config = Config::test();
        let events: [(u64, &[u8]); 3] = [(1, b"one"), (2, b"two"), (3, b"three")];

        let mut log = Log::init(&config).unwrap();
        let lock = log.write_lock().unwrap();
        for (seq_no, payload) in events {
            log.append(LogEntry::Events { seq_no, payload }).unwrap();
        }
        drop(log);

        let mut log = Log::init(&config).unwrap();
        let bytes = log.get_bytes().unwrap();
        let mut remaining = bytes.as_slice();
        for (expected_seq_no, expected_payload) in events {
            let Some(LogEntry::Events { seq_no, payload }) = head_entry(&mut remaining).unwrap()
            else {
                panic!("expected events entry");
            };
            assert_eq!(seq_no, expected_seq_no);
            assert_eq!(payload, expected_payload);
        }
        assert!(head_entry(&mut remaining).unwrap().is_none());
        lock.unlock().unwrap();
    }

    #[test]
    fn snapshot_recovery_rejects_pending_edits() {
        use crate::{View, errors::Error, views::hashmap::DbHashMap};

        type Map = DbHashMap<u64, u64>;

        let config = Config::test();
        let mut committed = Map::init(&config).unwrap();
        let mut dirty = Map::init(&config).unwrap();

        let tx = dirty.write_tx().unwrap();
        dirty.insert(2, 20).unwrap();
        drop(tx);

        let tx = committed.write_tx().unwrap();
        committed.insert(1, 10).unwrap();
        tx.end_tx(&mut committed).unwrap();

        // Leave a snapshot marker with neither a published nor a temporary snapshot.
        let tx = committed.write_tx().unwrap();
        committed.log_mut().append(LogEntry::Snapshot).unwrap();
        tx.end_tx(&mut committed).unwrap();
        drop(committed);

        assert!(matches!(dirty.write_tx(), Err(Error::Poisoned)));
        assert!(matches!(dirty.write_tx(), Err(Error::Poisoned)));
        assert!(!config.log_location.join("db.1.log").exists());
        assert!(!config.log_location.join("db.1.tmp").exists());

        // A fresh view can recover using only the committed log history.
        let recovered = Map::init(&config).unwrap();
        assert_eq!(recovered.get(&1), Some(&10));
        assert_eq!(recovered.get(&2), None);
        assert_eq!(recovered.log().seq_no, 1);
        assert!(config.log_location.join("db.1.log").is_file());

        let reopened = Map::init(&config).unwrap();
        assert_eq!(reopened.get(&1), Some(&10));
        assert_eq!(reopened.get(&2), None);
    }

    #[test]
    fn finds_no_candidate_without_numbered_logs() {
        let config = Config::test();
        for name in ["db.log", "db.1.tmp", "notes"] {
            File::create(config.log_location.join(name)).unwrap();
        }

        assert_eq!(Log::find_latest(&config).unwrap(), None);
    }

    #[test]
    fn finds_latest_log_candidate() {
        let config = Config::test();
        for name in [
            "db.0.log",
            "db.9.log",
            "db.10.log",
            "db.log",
            "db.11.tmp",
            "notes",
        ] {
            File::create(config.log_location.join(name)).unwrap();
        }
        fs::create_dir(config.log_location.join("db.99.log")).unwrap();

        assert_eq!(
            Log::find_latest(&config).unwrap(),
            Some(config.log_location.join("db.10.log"))
        );
    }

    #[test]
    fn finds_next_log_instead_of_latest() {
        let config = Config::test();
        File::create(config.log_location.join("db.0.log")).unwrap();
        let log = Log::init(&config).unwrap();
        for name in ["db.9.log", "db.10.log"] {
            File::create(config.log_location.join(name)).unwrap();
        }

        assert_eq!(
            log.find_next().unwrap().unwrap(),
            config.log_location.join("db.9.log")
        );
    }
}
