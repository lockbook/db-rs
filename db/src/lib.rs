use crate::config::Config;
use crate::errors::DbResult;
use crate::logger::Logger;

pub mod config;
pub mod errors;
pub mod logger;
pub mod lookup;
pub mod single;
pub mod table;

pub trait Db: Sized {
    fn init(location: Config) -> DbResult<Self>;
    fn compact_log(&mut self) -> DbResult<()>;
    fn get_logger(&mut self) -> &mut Logger;
    fn begin_transaction(&mut self) {
        self.get_logger().begin_tx();
    }
}

pub type TableId = u8;
pub type ByteCount = u32;
