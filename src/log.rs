use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    errors::{Error, Result},
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

pub struct Log {
    file: File,
    path: PathBuf,
    directory: PathBuf,
    pub(crate) seq_no: u64,
    pub(crate) poisoned: bool,
}

impl Log {
    pub fn init(config: &Config) -> io::Result<Self> {
        let directory = if config.log_location.is_empty() {
            PathBuf::from(".")
        } else {
            config.log_location.clone()
        };
        let candidate = Self::find_candidate(config)?;
        let create = candidate.is_none();
        let path = candidate.unwrap_or_else(|| directory.join("db.0.log"));
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(create)
            .open(&path)?;

        Ok(Self {
            file,
            path,
            directory,
            seq_no: 0,
            poisoned: false,
        })
    }

    pub fn find_candidate(config: &Config) -> io::Result<Option<PathBuf>> {
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
            let Some(sequence) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("db."))
                .and_then(|name| name.strip_suffix(".log"))
                .and_then(|number| number.parse::<u64>().ok())
            else {
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

    pub(crate) fn append(&mut self, entry: LogEntry<'_>) -> Result<()> {
        let body = bincode::serde::encode_to_vec(entry, bincode::config::standard())?;
        let mut buffer = PayloadBuffer::default();
        buffer.push(&body);
        self.file.write_all(&buffer.bytes)?;
        self.file.sync_all()?;
        Ok(())
    }

    pub(crate) fn create_snapshot(&self, seq_no: u64, payload: &[u8]) -> Result<Option<Self>> {
        let path = self.directory.join(format!("db.{seq_no}.log"));
        if path == self.path {
            return Ok(None);
        }

        let temporary_path = self.directory.join(format!("db.{seq_no}.tmp"));
        {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temporary_path)?;
            let mut snapshot = Self {
                file,
                path: temporary_path.clone(),
                directory: self.directory.clone(),
                seq_no,
                poisoned: false,
            };
            snapshot.append(LogEntry::Events { seq_no, payload })?;
        }
        fs::rename(temporary_path, &path)?;

        let file = OpenOptions::new().read(true).append(true).open(&path)?;
        Ok(Some(Self {
            file,
            path,
            directory: self.directory.clone(),
            seq_no,
            poisoned: false,
        }))
    }

    pub fn get_bytes(&mut self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub(crate) fn read_lock(&self) -> io::Result<File> {
        let file = OpenOptions::new().read(true).open(&self.path)?;
        file.lock_shared()?;
        Ok(file)
    }

    pub(crate) fn write_lock(&self) -> io::Result<File> {
        let file = OpenOptions::new().read(true).write(true).open(&self.path)?;
        file.lock()?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

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
    fn appended_events_can_be_read() {
        let config = Config::test();
        let events: [(u64, &[u8]); 3] = [(1, b"one"), (2, b"two"), (3, b"three")];

        let mut log = Log::init(&config).unwrap();
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
    }

    #[test]
    fn finds_no_candidate_without_numbered_logs() {
        let config = Config::test();
        for name in ["db.log", "db.1.tmp", "notes"] {
            File::create(config.log_location.join(name)).unwrap();
        }

        assert_eq!(Log::find_candidate(&config).unwrap(), None);
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
            Log::find_candidate(&config).unwrap(),
            Some(config.log_location.join("db.10.log"))
        );
    }
}
