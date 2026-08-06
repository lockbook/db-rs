use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::{fs, io};

use crate::Id;
use crate::config::IpcProfile::Client;
use crate::config::{Config, IoConfig};
use crate::errors::DbResult;

const LOGGER_ID: usize = Id::MAX;

pub(crate) struct LogEntry<'a> {
    pub(crate) seq_no: Id,
    pub(crate) payload: &'a [u8],
}

pub struct TxEntry<'a> {
    pub(crate) shard: Id,
    pub(crate) table: Id,
    pub(crate) payload: &'a [u8],
}

impl LogEntry<'_> {
    pub fn head_entry<'a>(buf: &[u8]) -> (Option<LogEntry>, &[u8]) {
        // seq_no
        let offset = 0;
        let id_size = size_of::<Id>();
        let Some(seq_no) = buf.get(offset..id_size) else {
            return (None, &buf[buf.len()..]);
        };
        let seq_no = Id::from_be_bytes(seq_no.try_into().unwrap());
        let offset = offset + id_size;

        // payload
        let Some(payload_size) = buf.get(offset..id_size) else {
            return (None, &buf[buf.len()..]);
        };
        let payload_size = Id::from_be_bytes(payload_size.try_into().unwrap());
        let offset = offset + id_size;
        let Some(payload) = buf.get(offset..offset + payload_size) else {
            return (None, &buf[buf.len()..]);
        };
        let offset = offset + payload_size;

        // parsed entry & what remains
        let entry = LogEntry { seq_no, payload };
        (Some(entry), &buf[offset..])
    }
}

impl TxEntry<'_> {
    pub fn head_entry(&self, buf: &[u8]) -> (Option<TxEntry>, &[u8]) {
        todo!()
    }

    fn write_to_buffer(&self, buf: &mut Vec<u8>) {
        let shard = self.shard.to_be_bytes();
        let table = self.table.to_be_bytes();

        buf.reserve(shard.len() + table.len() + buf.len());
        buf.extend_from_slice(&shard);
        buf.extend_from_slice(&table);
        buf.extend_from_slice(self.payload);
    }
}

#[derive(Clone)]
pub struct Logger {
    pub(crate) seq: Arc<AtomicUsize>,
    pub(crate) table_seq: usize,
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
            seq: Default::default(),
            table_seq: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct Io {
    pending_tx: Vec<u8>,
    incomplete_write: bool,
    file: Option<fs::File>,
    lock: Option<fs::File>,
}

impl Logger {
    pub(crate) fn init(config: &Config) -> DbResult<Self> {
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
            pending_tx: Default::default(),
            incomplete_write: false,
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

    pub fn append(&mut self, data: Vec<u8>) {
        self.table_seq = self.seq.load(Ordering::SeqCst);
        let event = TxEntry {
            shard: self.shard_id,
            table: self.table_id,
            payload: &data,
        };

        let buffer = &mut self.io.lock().unwrap().pending_tx;
        event.write_to_buffer(buffer);
    }

    pub(crate) fn commit(&self) {
        todo!()
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

    pub(crate) fn next_event<'a>(&self, offset: usize, buffer: &[u8]) -> (Option<TxEntry>, usize) {
        todo!()
    }
}

pub struct LogReader {
    data: Vec<u8>,
    current_offset: usize,
    incomplete_log: bool,
}

impl LogReader {
    fn next_event(&mut self) -> Option<TxEntry> {
        if self.current_offset >= self.data.len() {
            return None;
        }

        todo!()
    }
}
