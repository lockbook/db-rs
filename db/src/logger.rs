use crate::config::Config;
use crate::errors::DbResult;
use crate::TableId;
use std::cell::RefCell;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Read;
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
        let path = config.db_location()?;

        if config.create_path {
            fs::create_dir_all(&path)?;
        }

        let can_write = !config.read_only;

        let file = OpenOptions::new()
            .read(true)
            .create(config.create_db)
            .append(can_write)
            .open(&path)?;

        let incomplete_write = false;
        let tx_data = None;
        let current_txs = 0;

        let inner = LoggerInner { file, config, incomplete_write, tx_data, current_txs };
        let inner = Rc::new(RefCell::new(inner));

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
            let table_id = buffer[index];
            index += 1;

            let size = u32::from_be_bytes(
                buffer[index..index + 4]
                    .try_into()
                    .expect("slice with incorrect length"),
            ) as usize;
            index += 4;

            if buffer.len() < index + size {
                self.inner.borrow_mut().incomplete_write = true;
                return Ok(entries);
            }

            if table_id == 0 {
                if buffer.len() < index + 4 {
                    self.inner.borrow_mut().incomplete_write = true;
                    return Ok(entries);
                } else {
                    continue;
                }
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

    pub fn end_tx(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.current_txs == 0 {
            eprintln!("called end_tx while no transaction active!");
            return;
        }

        inner.current_txs -= 0;
        if inner.current_txs == 0 {
            let data = inner.tx_data.take();
            drop(inner);
            if let Some(data) = data {
                self.write(0, &data);
            }
        }
    }

    pub fn write(&self, id: TableId, data: &[u8]) {
        todo!()
    }
}

pub struct TxHandle {
    inner: Logger,
}

impl Drop for TxHandle {
    fn drop(&mut self) {
        self.inner.end_tx();
    }
}
