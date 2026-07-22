use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use std::{fs, io};

use crate::Id;
use crate::config::IpcProfile::Client;
use crate::config::{Config, IoConfig};

const LOGGER_ID: usize = Id::MAX;

pub(crate) struct Event {
    pub shard_id: Id,
    pub table_id: Id,
    pub data: Vec<u8>,
}

#[derive(Clone)]
pub struct Logger {
    pub(crate) shard_id: Id,
    pub(crate) table_id: Id,

    io: Arc<Mutex<Io>>,
}

// todo: aim to get rid of this
impl Default for Logger {
    fn default() -> Self {
        Self {
            shard_id: LOGGER_ID,
            table_id: LOGGER_ID,
            io: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct Io {
    file: Option<fs::File>,
    lock: Option<fs::File>,
}

impl Logger {
    pub(crate) fn init(config: &Config) -> io::Result<Self> {
        let base = Self::default();
        let Some(io_config) = &config.io else {
            return Ok(base);
        };

        fs::create_dir_all(&io_config.data_dir)?;

        let lock = if io_config.ipc_profile == Client {
            Some(Self::acquire_lock(io_config)?)
        } else {
            None
        };

        let mut opts = fs::OpenOptions::new();
        opts.read(true);
        if !io_config.read_only {
            opts.append(true).create(true);
        }

        let file = opts.open(io_config.log_path())?;

        *base.io.lock().unwrap() = Io {
            file: Some(file),
            lock,
        };

        Ok(base)
    }

    fn acquire_lock(io: &IoConfig) -> io::Result<fs::File> {
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(io.lock_path())?;

        // fs2 is unavailable on wasm; there's no multi-process story there anyway.
        #[cfg(not(target_family = "wasm"))]
        {
            use fs2::FileExt;
            // Non-blocking: fail fast with an error if another process holds it.
            // Readers share, writers are exclusive.
            if io.read_only {
                lock.try_lock_shared()?;
            } else {
                lock.try_lock_exclusive()?;
            }
        }

        Ok(lock)
    }

    pub fn append(&self, data: Vec<u8>) {
        let event = Event {
            shard_id: self.shard_id,
            table_id: self.table_id,
            data,
        };

        
    }

    pub(crate) fn read_from_file(&self) -> io::Result<Vec<u8>> {
        let binding = self.io.lock().unwrap();
        let Some(mut file) = binding.file.as_ref() else {
            return Ok(Vec::new());
        };

        file.seek(SeekFrom::Start(0))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn get_events(&self) -> Vec<Event> {
        todo!()
    }
}
