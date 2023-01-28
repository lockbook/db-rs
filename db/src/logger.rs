use crate::config::Config;
use crate::TableId;
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;

pub struct LogFormat<'a> {
    pub table_id: TableId,
    pub bytes: &'a [u8],
}

#[derive(Clone)]
pub struct Logger {
    file: Rc<RefCell<File>>,
}

impl Logger {
    pub fn init(c: Config) -> Self {
        todo!()
    }
    pub fn get_entries(&self) -> Vec<LogFormat> {
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
