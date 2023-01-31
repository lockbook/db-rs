use crate::{Config, DbResult, Logger, TxHandle};

pub trait Db: Sized {
    fn init(location: Config) -> DbResult<Self>;
    fn compact_log(&mut self) -> DbResult<()>;
    fn get_logger(&self) -> &Logger;
    fn begin_transaction(&mut self) -> TxHandle {
        self.get_logger().begin_tx()
    }
}
