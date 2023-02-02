use crate::config::Config;
use crate::errors::DbResult;
use crate::{ByteCount, TableId};
use std::cell::RefCell;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::rc::Rc;

pub struct LogFormat<'a> {
    pub table_id: TableId,
    pub bytes: &'a [u8],
}

#[derive(Clone)]
pub struct Logger {
    inner: Rc<RefCell<LoggerInner>>,
}

struct LoggerInner {
    config: Config,
    file: File,
    incomplete_write: bool,
    current_txs: usize,
    tx_data: Option<Vec<u8>>,
}

impl Logger {
    pub fn init(config: Config) -> DbResult<Self> {
        if config.create_path {
            fs::create_dir_all(&config.path)?;
        }

        let file = Self::open_file(&config, &config.db_location()?)?;

        let incomplete_write = false;
        let tx_data = None;
        let current_txs = 0;

        let inner = Rc::new(
            RefCell::new(
                LoggerInner { file, config, incomplete_write, tx_data, current_txs }
            )
        );

        Ok(Self { inner })
    }

    pub fn get_bytes(&self) -> DbResult<Vec<u8>> {
        let mut buffer: Vec<u8> = Vec::new();

        let mut inner = self.inner.borrow_mut();
        inner.file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    pub fn get_entries<'a>(&self, buffer: &'a [u8]) -> DbResult<Vec<LogFormat<'a>>> {
        let mut index = 0;
        let mut entries = vec![];

        while index < buffer.len() {
            if buffer.len() < index + 4 + 1 {
                self.inner.borrow_mut().incomplete_write = true;
                return Ok(entries);
            }

            let table_id = buffer[index];
            index += 1;

            let size = ByteCount::from_be_bytes(
                buffer[index..index + 4] // todo bounds check
                    .try_into()
                    .expect("slice with incorrect length"),
            ) as usize;
            index += 4;

            if buffer.len() < index + size {
                self.inner.borrow_mut().incomplete_write = true;
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

    pub fn begin_tx(&self) -> TxHandle {
        let h = TxHandle { inner: self.clone() };
        let mut inner = self.inner.borrow_mut();
        if inner.tx_data.is_none() {
            inner.tx_data = Some(vec![]);
        }
        inner.current_txs += 1;
        h
    }

    pub fn end_tx(&self) -> DbResult<()> {
        let mut inner = self.inner.borrow_mut();
        if inner.current_txs == 0 {
            return Ok(());
        }

        inner.current_txs -= 1;
        if inner.current_txs == 0 {
            let data = inner.tx_data.take();
            drop(inner);
            if let Some(data) = data {
                self.write_to_file(0, data)?;
            }
        }

        Ok(())
    }

    pub fn write(&self, id: TableId, data: Vec<u8>) -> DbResult<()> {
        let mut inner = self.inner.borrow_mut();
        if let Some(tx_data) = &mut inner.tx_data {
            tx_data.append(&mut Self::log_entry(id, data));
            return Ok(());
        }
        drop(inner);

        self.write_to_file(id, data)
    }

    pub fn write_to_file(&self, id: TableId, data: Vec<u8>) -> DbResult<()> {
        let mut inner = self.inner.borrow_mut();
        if !inner.config.no_io {
            inner.file.write_all(&Self::log_entry(id, data))?;
        }
        Ok(())
    }

    pub fn log_entry(id: TableId, mut data: Vec<u8>) -> Vec<u8> {
        // could be more efficient by unsafe prepending to data
        let mut data_to_write = Vec::with_capacity(data.len() + 5);
        data_to_write.push(id);
        data_to_write.extend((data.len() as ByteCount).to_be_bytes());
        data_to_write.append(&mut data);
        data_to_write
    }

    pub fn compact_log(&self, data: Vec<u8>) -> DbResult<()> {
        let mut inner = self.inner.borrow_mut();
        if inner.config.no_io {
            return Ok(());
        }

        let temp_path = inner.config.compaction_location()?;
        let final_path = inner.config.db_location()?;

        let mut file = Self::open_file(&inner.config, &temp_path)?;
        let data = Self::log_entry(0, data);
        file.write_all(&data)?;

        fs::rename(temp_path, final_path)?;
        inner.file = file;

        Ok(())
    }

    fn open_file(config: &Config, db_location: &Path) -> DbResult<File> {
        Ok(OpenOptions::new()
            .read(true)
            .create(config.create_db)
            .append(!config.read_only)
            .open(db_location)?)
    }

    pub(crate) fn config(&self) -> Config {
        self.inner.borrow().config.clone()
    }

    pub(crate) fn incomplete_write(&self) -> bool {
        self.inner.borrow().incomplete_write
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
