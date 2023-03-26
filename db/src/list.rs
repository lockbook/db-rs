use crate::table::Table;
use crate::{DbResult, Logger, TableId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct List<T>
where
    T: Serialize + DeserializeOwned,
{
    table_id: TableId,
    inner: Vec<T>,
    pub logger: Logger,
}

#[derive(Serialize, Deserialize)]
pub enum LogEntry<T> {
    Push(T),
    Insert(usize, T),
    Remove(usize),
    Pop(),
    Clear,
}

impl<T> Table for List<T>
where
    T: Serialize + DeserializeOwned,
{
    fn init(table_id: TableId, logger: Logger) -> Self {
        let inner = vec![];
        Self { table_id, inner, logger }
    }

    fn handle_event(&mut self, bytes: &[u8]) -> DbResult<()> {
        match bincode::deserialize(bytes)? {
            LogEntry::Insert(idx, element) => {
                self.inner.insert(idx, element);
            }
            LogEntry::Remove(idx) => {
                self.inner.remove(idx);
            }
            LogEntry::Push(el) => {
                self.inner.push(el);
            }
            LogEntry::Pop() => {
                self.inner.pop();
            }
            LogEntry::Clear => {
                self.inner.clear();
            }
        };

        Ok(())
    }

    fn compact_repr(&self) -> DbResult<Vec<u8>> {
        let mut repr = vec![];

        for v in &self.inner {
            let data = bincode::serialize(&LogEntry::Push(v))?;
            let mut data = Logger::log_entry(self.table_id, data);
            repr.append(&mut data);
        }

        Ok(repr)
    }
}

impl<T> List<T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn push(&mut self, t: T) -> DbResult<()> {
        let log_entry = LogEntry::Push(&t);
        let data = bincode::serialize(&log_entry)?;
        self.inner.push(t);

        self.logger.write(self.table_id, data)?;
        Ok(())
    }

    pub fn pop(&mut self) -> DbResult<Option<T>> {
        let log_entry: LogEntry<()> = LogEntry::Pop();
        let data = bincode::serialize(&log_entry)?;
        let result = self.inner.pop();

        self.logger.write(self.table_id, data)?;
        Ok(result)
    }

    pub fn remove(&mut self, index: usize) -> DbResult<T> {
        let log_entry: LogEntry<usize> = LogEntry::Remove(index);
        let data = bincode::serialize(&log_entry)?;
        let result = self.inner.remove(index);

        self.logger.write(self.table_id, data)?;
        Ok(result)
    }

    pub fn data(&self) -> &[T] {
        &self.inner
    }
}
