use crate::errors::DbResult;
use crate::logger::Logger;
use crate::TableId;

pub trait Table {
    fn init(table_id: TableId, logger: Logger) -> Self;
    fn handle_event(&mut self, bytes: &[u8]) -> DbResult<()>;
    fn compact_repr(&self) -> DbResult<Vec<u8>>;
}
