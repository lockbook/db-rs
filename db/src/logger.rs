use crate::config::Config;
use crate::errors::DbResult;
use crate::{ByteCount, TableId};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct LogFormat<'a> {
    pub table_id: TableId,
    pub bytes: &'a [u8],
}

#[derive(Clone, Debug)]
pub struct Logger {
    inner: Arc<Mutex<LoggerInner>>,
}

#[derive(Debug)]
struct LoggerInner {
    config: Config,
    file: Option<File>,
    incomplete_write: bool,
    current_txs: usize,
    tx_data: Option<Vec<u8>>,
}

impl Logger {
    pub fn init(config: Config) -> DbResult<Self> {
        if config.create_path {
            fs::create_dir_all(&config.path)?;
        }

        let file = if config.no_io {
            None
        } else {
            Some(Self::open_file(&config, &config.db_location()?)?)
        };

        let incomplete_write = false;
        let tx_data = None;
        let current_txs = 0;

        let inner = Arc::new(Mutex::new(LoggerInner {
            file,
            config,
            incomplete_write,
            tx_data,
            current_txs,
        }));

        Ok(Self { inner })
    }

    pub fn get_bytes(&self) -> DbResult<Vec<u8>> {
        let mut buffer: Vec<u8> = Vec::new();

        let mut inner = self.inner.lock()?;
        if let Some(file) = inner.file.as_mut() {
            file.read_to_end(&mut buffer)?;
        }

        Ok(buffer)
    }

    pub fn get_entries<'a>(&self, buffer: &'a [u8]) -> DbResult<Vec<LogFormat<'a>>> {
        let mut index = 0;
        let mut entries = vec![];

        while index < buffer.len() {
            if buffer.len() < index + 4 + 1 {
                self.inner.lock()?.incomplete_write = true;
                return Ok(entries);
            }

            let table_id = buffer[index];
            index += 1;

            let size = ByteCount::from_be_bytes(
                buffer[index..index + 4]
                    .try_into()
                    .expect("slice with incorrect length"),
            ) as usize;
            index += 4;

            if buffer.len() < index + size {
                self.inner.lock()?.incomplete_write = true;
                return Ok(entries);
            }

            if table_id == 0 {
                continue;
            }

            let bytes = &buffer[index..index + size];
            entries.push(LogFormat { table_id, bytes });
            index += size;
        }

        Ok(entries)
    }

    pub fn begin_tx(&self) -> DbResult<TxHandle> {
        let h = TxHandle { inner: self.clone() };
        let mut inner = self.inner.lock()?;
        if inner.tx_data.is_none() {
            inner.tx_data = Some(vec![]);
        }
        inner.current_txs += 1;
        Ok(h)
    }

    pub fn end_tx(&self) -> DbResult<()> {
        let mut inner = self.inner.lock()?;
        if inner.current_txs == 0 {
            return Ok(());
        }

        inner.current_txs -= 1;
        if inner.current_txs == 0 {
            let data = inner.tx_data.take();
            drop(inner);
            if let Some(data) = data {
                self.write_to_file(Self::log_entry(0, data))?;
            }
        }

        Ok(())
    }

    pub fn write(&self, id: TableId, mut data: Vec<u8>) -> DbResult<()> {
        let mut inner = self.inner.lock()?;
        if inner.config.no_io {
            return Ok(());
        }

        if let Some(tx_data) = &mut inner.tx_data {
            tx_data.extend(Self::header(id, &data));
            tx_data.append(&mut data);
            return Ok(());
        }

        drop(inner);

        self.write_to_file(Self::log_entry(id, data))
    }

    fn write_to_file(&self, data: Vec<u8>) -> DbResult<()> {
        let mut inner = self.inner.lock()?;
        if let Some(file) = inner.file.as_mut() {
            file.write_all(&data)?;
        }
        Ok(())
    }

    pub fn header(id: TableId, data: &[u8]) -> [u8; 5] {
        let size_info = (data.len() as ByteCount).to_be_bytes();
        [id, size_info[0], size_info[1], size_info[2], size_info[3]]
    }

    pub fn log_entry(id: TableId, mut data: Vec<u8>) -> Vec<u8> {
        let header = Self::header(id, &data);
        data.reserve(header.len());
        data.splice(0..0, header);
        data
    }

    pub fn compact_log(&self, data: Vec<u8>) -> DbResult<()> {
        let mut inner = self.inner.lock()?;
        if inner.config.no_io {
            return Ok(());
        }

        let temp_path = inner.config.compaction_location()?;
        let final_path = inner.config.db_location()?;

        let mut file = Self::open_file(&inner.config, &temp_path)?;
        let data = Self::log_entry(0, data);
        file.write_all(&data)?;

        fs::rename(temp_path, final_path)?;
        inner.file = Some(file);

        Ok(())
    }

    fn open_file(config: &Config, db_location: &Path) -> DbResult<File> {
        Ok(OpenOptions::new()
            .read(true)
            .create(config.create_db || config.read_only)
            .append(!config.read_only)
            .open(db_location)?)
    }

    pub(crate) fn config(&self) -> DbResult<Config> {
        Ok(self.inner.lock()?.config.clone())
    }

    pub(crate) fn incomplete_write(&self) -> DbResult<bool> {
        Ok(self.inner.lock()?.incomplete_write)
    }
}

#[must_use = "DB stays in Tx mode while this value is in scope. Manually call drop_safely() to handle io errors that may arise when tx terminates."]
pub struct TxHandle {
    inner: Logger,
}

impl TxHandle {
    pub fn drop_safely(&self) -> DbResult<()> {
        self.inner.end_tx()
    }
}

impl Drop for TxHandle {
    fn drop(&mut self) {
        self.drop_safely()
            .expect("auto tx-end panicked. Call drop_safely() for non-panicking variant.");
    }
}
