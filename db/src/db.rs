use std::sync::{Arc, Mutex};

use crate::{table::Table, Config, DbResult, Logger, TableId, TxHandle};

pub trait Db: Sized {
    fn init(mut config: Config) -> DbResult<Arc<Mutex<Self>>> {
        let schema_name = Self::schema_name();
        config.schema_name = Some(schema_name.to_string());

        let mut db = Self::init_tables(config)?;
        let log_data = db.get_logger().get_bytes()?;
        let log_entries = db.get_logger().get_entries(&log_data)?;
        for entry in log_entries {
            db.handle_event(entry.table_id, entry.bytes)?;
        }

        Ok(Arc::new(Mutex::new(db)))
    }

    fn compact_log(&mut self) -> DbResult<()>;

    fn get_logger(&self) -> &Logger;

    fn config(&self) -> DbResult<Config> {
        self.get_logger().config()
    }

    fn incomplete_write(&self) -> DbResult<bool> {
        self.get_logger().incomplete_write()
    }

    fn begin_transaction(&mut self) -> DbResult<TxHandle> {
        self.get_logger().begin_tx()
    }

    #[doc(hidden)]
    fn init_tables(config: Config) -> DbResult<Self>;

    #[doc(hidden)]
    fn handle_event(&mut self, table_id: TableId, data: &[u8]) -> DbResult<()>;

    fn schema_name() -> &'static str;
}
