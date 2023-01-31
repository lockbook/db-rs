use crate::errors::DbResult;
use crate::logger::Logger;
use crate::table::Table;
use crate::TableId;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

pub struct LookupTable<K, V>
where
    K: Hash + Eq + Serialize,
    V: Hash + Serialize,
{
    table_id: TableId,
    inner: HashMap<K, V>,
    pub(crate) logger: Logger,
}

#[derive(Serialize, Deserialize)]
pub enum LogEntry<K, V> {
    Insert(K, V),
    Remove(K),
    Clear,
}

impl<K, V> Table for LookupTable<K, V>
where
    K: Hash + Eq + Serialize + DeserializeOwned,
    V: Hash + Serialize + DeserializeOwned,
{
    fn init(table_id: TableId, logger: Logger) -> Self {
        Self { table_id, inner: HashMap::default(), logger }
    }

    fn handle_event(&mut self, bytes: &[u8]) -> DbResult<()> {
        match bincode::deserialize(bytes)? {
            LogEntry::Insert(k, v) => {
                self.insert(k, v)?;
            }
            LogEntry::Remove(k) => {
                self.inner.remove(&k);
            }
            LogEntry::Clear => {
                self.inner.clear();
            }
        };

        Ok(())
    }
}

impl<K, V> LookupTable<K, V>
where
    K: Hash + Eq + Serialize + DeserializeOwned,
    V: Hash + Serialize + DeserializeOwned,
{
    pub fn insert(&mut self, key: K, value: V) -> DbResult<Option<V>> {
        let log_entry = LogEntry::Insert(key, value);
        let data = bincode::serialize(&log_entry)?;

        let ret = if let LogEntry::Insert(key, value) = log_entry {
            self.inner.insert(key, value)
        } else {
            None
        };

        self.logger.write(self.table_id, data)?;
        Ok(ret)
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.inner.get(k)
    }

    pub fn clear(&mut self) -> DbResult<()> {
        self.inner.clear();
        let log_entry = LogEntry::<K, V>::Clear;
        let data = bincode::serialize(&log_entry)?;
        self.logger.write(self.table_id, data)?;

        Ok(())
    }
}
