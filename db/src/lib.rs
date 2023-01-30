use crate::config::Config;
use crate::errors::DbError;
use crate::logger::Logger;

pub mod config;
pub mod errors;
pub mod logger;
pub mod lookup;
pub mod serializer;
pub mod single;
pub mod table;

pub type DbResult<T> = Result<T, DbError>;

pub trait Db: Sized {
    fn init(location: Config) -> DbResult<Self>;
    fn get_logger(&mut self) -> &mut Logger;
    fn begin_transaction(&mut self) {
        self.get_logger().begin_tx();
    }
}

pub type TableId = u8;
pub type ByteCount = u32;
