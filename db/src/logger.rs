use crate::config::Config;
use crate::errors::DbResult;
use crate::{ByteCount, DbError, TableId};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[cfg(not(target_family = "wasm"))]
use fs2::FileExt;

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
    log_metadata: Option<LogMetadata>,
    incomplete_write: bool,
    current_txs: usize,
    tx_data: Option<Vec<u8>>,
}

impl Logger {
    pub fn init(config: Config) -> DbResult<Self> {
        if config.create_path {
            // todo: is this happening for no_io?
            fs::create_dir_all(&config.path)?;
        }

        let mut file = if config.no_io {
            None
        } else {
            Self::handle_migration(&config)?;
            Some(Self::open_file(&config, &config.db_location_v2()?)?)
        };

        let incomplete_write = false;
        let tx_data = None;
        let current_txs = 0;

        let log_metadata = Self::read_or_stamp_metadata(&config, &mut file)?;

        let inner = Arc::new(Mutex::new(LoggerInner {
            file,
            config,
            incomplete_write,
            tx_data,
            current_txs,
            log_metadata,
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
        let final_path = inner.config.db_location_v2()?;

        let mut file = Self::open_file(&inner.config, &temp_path)?;

        // write compaction count for future IPC reasons
        let mut log_meta = inner
            .log_metadata
            .ok_or(DbError::Unexpected("log meta missing -- no_io == false"))?;
        log_meta.compaction_count += 1;
        let metadata_bytes = log_meta.to_bytes();
        file.write_all(&metadata_bytes)?;

        // write compacted data to a temporary file
        let compacted_data = Self::log_entry(0, data);
        file.write_all(&compacted_data)?;

        // atomically make this the new log
        fs::rename(temp_path, final_path)?;
        inner.file = Some(file);
        inner.log_metadata = Some(log_meta);

        Ok(())
    }

    fn handle_migration(config: &Config) -> DbResult<()> {
        let v1 = config.db_location_v1()?;
        let v2 = config.db_location_v2()?;
        let v2_temp = PathBuf::from(format!("{}.migration", v2.to_string_lossy()));

        if !v1.exists() {
            return Ok(());
        }

        if v2_temp.exists() {
            fs::remove_file(&v2_temp)?;
        }

        if v2.exists() {
            return Ok(());
        }

        let v1_bytes = fs::read(&v1)?;
        let mut v2_bytes = LogMetadata::default().to_bytes().to_vec();
        v2_bytes.extend(v1_bytes);
        fs::write(&v2_temp, v2_bytes)?;
        fs::rename(v2_temp, v2)?;
        fs::remove_file(v1)?;

        Ok(())
    }

    fn open_file(config: &Config, db_location: &Path) -> DbResult<File> {
        let file = OpenOptions::new()
            .read(true)
            .create(config.create_db || config.read_only)
            .append(!config.read_only)
            .open(db_location)?;

        if config.fs_locks {
            if config.fs_locks_block {
                #[cfg(not(target_family = "wasm"))]
                file.lock_exclusive()?;
            } else {
                #[cfg(not(target_family = "wasm"))]
                file.try_lock_exclusive()?;
            }
        }

        Ok(file)
    }

    pub(crate) fn config(&self) -> DbResult<Config> {
        Ok(self.inner.lock()?.config.clone())
    }

    pub(crate) fn incomplete_write(&self) -> DbResult<bool> {
        Ok(self.inner.lock()?.incomplete_write)
    }

    fn read_or_stamp_metadata(
        config: &Config, file: &mut Option<File>,
    ) -> DbResult<Option<LogMetadata>> {
        match file {
            Some(file) => {
                let mut buffer = [0_u8; 2];
                let bytes_read = file.read(&mut buffer)?;
                let mut needs_stamp = false;
                match bytes_read {
                    0 => {
                        needs_stamp = true;
                        buffer = LogMetadata::default().to_bytes();
                    }
                    2 => {}
                    _ => {
                        return Err(DbError::Unexpected(
                            "Unexpected amount of bytes read from log stamp",
                        ))
                    }
                };

                if !config.read_only && needs_stamp {
                    file.write_all(&buffer)?;
                }
                let meta = LogMetadata::from_bytes(buffer);
                if meta.log_version != 1 {
                    return Err(DbError::Unexpected("unexpected log format version found"));
                }

                Ok(Some(meta))
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LogMetadata {
    /// knowing the log version that we're reading allows us to evolve the format and make breaking
    /// changes. At the very least, allows us to return an error in the event of a version mismatch
    /// (leaving the migration up to the client)
    log_version: u8,

    /// compaction count is going to be a key data point to read when there are multiple processes
    /// reading and operating on the same log
    compaction_count: u8,
}

impl Default for LogMetadata {
    fn default() -> Self {
        Self { log_version: 1, compaction_count: 0 }
    }
}

impl LogMetadata {
    fn to_bytes(self) -> [u8; 2] {
        [self.log_version, self.compaction_count]
    }

    fn from_bytes(bytes: [u8; 2]) -> Self {
        Self { log_version: bytes[0], compaction_count: bytes[1] }
    }
}

#[cfg(not(target_family = "wasm"))]
impl Drop for LoggerInner {
    fn drop(&mut self) {
        if let Some(file) = &self.file {
            if self.config.fs_locks {
                if let Err(e) = file.unlock() {
                    eprintln!("failed to unlock log lock: {:?}", e);
                }
            }
        }
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
