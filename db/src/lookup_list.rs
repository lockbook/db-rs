use crate::table::Table;
use crate::{DbResult, Logger, TableId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

pub struct LookupSet<K, V>
where
    K: Hash + Eq + Serialize,
    V: Serialize + Eq,
{
    table_id: TableId,
    inner: HashMap<K, HashSet<V>>,
    pub logger: Logger,
}

#[derive(Serialize, Deserialize)]
pub enum LogEntry<K, V> {
    Insert(K, V),
    Remove(K, V),
    ClearKey(K),
    Clear,
}

impl<K, V> Table for LookupSet<K, V>
where
    K: Hash + Eq + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + Eq + Hash,
{
    fn init(table_id: TableId, logger: Logger) -> Self {
        Self { table_id, inner: HashMap::default(), logger }
    }

    fn handle_event(&mut self, bytes: &[u8]) -> DbResult<()> {
        match bincode::deserialize::<LogEntry<K, V>>(bytes)? {
            LogEntry::Insert(k, v) => {
                if let Some(list) = self.inner.get_mut(&k) {
                    list.insert(v);
                } else {
                    let mut set = HashSet::new();
                    set.insert(v);
                    self.inner.insert(k, set);
                }
            }
            LogEntry::Remove(k, v) => {
                if let Some(x) = self.inner.get_mut(&k) {
                    x.remove(&v);
                }
            }
            LogEntry::ClearKey(k) => {
                self.inner.remove(&k);
            }
            LogEntry::Clear => {
                self.inner.clear();
            }
        };

        Ok(())
    }

    fn compact_repr(&self) -> DbResult<Vec<u8>> {
        let mut repr = vec![];
        for (k, values) in &self.inner {
            for v in values {
                let data = bincode::serialize(&LogEntry::Insert(k, v))?;
                let mut data = Logger::log_entry(self.table_id, data);
                repr.append(&mut data);
            }
        }

        Ok(repr)
    }
}

impl<K, V> LookupSet<K, V>
where
    K: Hash + Eq + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + Eq + Hash,
{
    pub fn insert(&mut self, key: K, value: V) -> DbResult<bool> {
        let log_entry = LogEntry::Insert(&key, &value);
        let data = bincode::serialize(&log_entry)?;

        let ret = self
            .inner
            .get_mut(&key)
            .unwrap_or(&mut HashSet::default())
            .insert(value);

        self.logger.write(self.table_id, data)?;
        Ok(ret)
    }

    pub fn remove(&mut self, key: &K, value: &V) -> DbResult<bool> {
        let log_entry = LogEntry::Remove::<&K, &V>(key, value);
        let data = bincode::serialize(&log_entry)?;
        let ret = self.inner.get_mut(key).unwrap().remove(value);
        self.logger.write(self.table_id, data)?;
        Ok(ret)
    }

    pub fn data(&self) -> &HashMap<K, HashSet<V>> {
        &self.inner
    }

    pub fn clear(&mut self) -> DbResult<()> {
        self.inner.clear();
        let log_entry = LogEntry::<K, V>::Clear;
        let data = bincode::serialize(&log_entry)?;
        self.logger.write(self.table_id, data)?;

        Ok(())
    }
    pub fn clear_key(&mut self, key: &K) -> DbResult<()> {
        self.inner.get_mut(key).unwrap().clear();
        let log_entry = LogEntry::<K, V>::Clear;
        let data = bincode::serialize(&log_entry)?;
        self.logger.write(self.table_id, data)?;

        Ok(())
    }
}
