use crate::config::Config;
use crate::{DbResult, TableId};
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
    file: File,
    config: Config,
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

        let inner = LoggerInner { file, config };
        let inner = Rc::new(RefCell::new(inner));

        Ok(Self { inner })
    }

    pub fn get_bytes(&self) -> DbResult<Vec<u8>> {
        let mut buffer: Vec<u8> = Vec::new();

        let mut inner = self.inner.borrow_mut();
        inner.file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    pub fn get_entries(&self, buffer: Vec<u8>) -> Vec<LogFormat> {
        todo!()
    }
    pub fn begin_tx(&self) {
        todo!()
    }
    pub fn end_tx(&self) {
        todo!()
    }
    pub fn write(&self, id: TableId, data: &[u8]) {
        todo!()
    }
}
