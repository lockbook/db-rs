use std::io::{Read, Seek, SeekFrom};
use std::{fs, io};

use crate::config::IpcProfile::Client;
use crate::config::{Config, IoConfig};

pub struct Event {
    pub shard_id: u32,
    pub table_id: u32,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct Log {
    file: Option<fs::File>,
    lock: Option<fs::File>,
}

impl Log {
    pub(crate) fn init(config: &Config) -> io::Result<Self> {
        let Some(io) = &config.io else {
            return Ok(Self {
                file: None,
                lock: None,
            });
        };

        fs::create_dir_all(&io.data_dir)?;

        let lock = if io.ipc_profile == Client {
            Some(Self::acquire_lock(io)?)
        } else {
            None
        };

        let mut opts = fs::OpenOptions::new();
        opts.read(true);
        if !io.read_only {
            opts.append(true).create(true);
        }

        Ok(Self {
            file: Some(opts.open(io.log_path())?),
            lock,
        })
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

    // todo: this needs to take an event, table needs to know it's id, perhaps also the shard id,
    // tbd. Perhaps each logger knows this? Perhaps looger will be slightly different for each
    // person who can write to the log?
    pub fn append(&self, bytes: Vec<u8>) {}

    pub(crate) fn read_from_file(&self) -> io::Result<Vec<u8>> {
        let Some(mut file) = self.file.as_ref() else {
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
