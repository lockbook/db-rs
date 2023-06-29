use crate::table::Table;
use crate::{DbResult, Logger, TableId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// A special case of [crate::lookup::LookupTable] where the value of the [HashMap] is a `HashSet<V>`.
#[derive(Debug)]
#[cfg_attr(feature = "clone", derive(Clone))]
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
    CreateKey(K),
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
                self.insert_inner(k, v);
            }
            LogEntry::Remove(k, v) => {
                if let Some(x) = self.inner.get_mut(&k) {
                    x.remove(&v);
                }
            }
            LogEntry::CreateKey(k) => {
                self.inner.insert(k, HashSet::new());
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
            if values.is_empty() {
                let data = bincode::serialize(&LogEntry::<&K, &V>::CreateKey(k))?;
                let mut data = Logger::log_entry(self.table_id, data);
                repr.append(&mut data);
                continue;
            }
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
    pub(crate) fn insert_inner(&mut self, k: K, v: V) -> bool {
        if let Some(set) = self.inner.get_mut(&k) {
            set.insert(v)
        } else {
            let mut set = HashSet::new();
            set.insert(v);
            self.inner.insert(k, set);
            false
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> DbResult<bool> {
        let log_entry = LogEntry::Insert(&key, &value);
        let data = bincode::serialize(&log_entry)?;
        let ret = self.insert_inner(key, value);
        self.logger.write(self.table_id, data)?;
        Ok(ret)
    }

    pub fn create_key(&mut self, key: K) -> DbResult<Option<HashSet<V>>> {
        let log_entry = LogEntry::<&K, &V>::CreateKey(&key);
        let data = bincode::serialize(&log_entry)?;

        let ret = self.inner.insert(key, HashSet::new());

        self.logger.write(self.table_id, data)?;
        Ok(ret)
    }

    pub fn remove(&mut self, key: &K, value: &V) -> DbResult<bool> {
        if let Some(set) = self.inner.get_mut(key) {
            let log_entry = LogEntry::Remove::<&K, &V>(key, value);
            let data = bincode::serialize(&log_entry)?;
            self.logger.write(self.table_id, data)?;
            Ok(set.remove(value))
        } else {
            Ok(false)
        }
    }

    pub fn get(&self) -> &HashMap<K, HashSet<V>> {
        &self.inner
    }

    pub fn clear(&mut self) -> DbResult<()> {
        self.inner.clear();
        let log_entry = LogEntry::<K, V>::Clear;
        let data = bincode::serialize(&log_entry)?;
        self.logger.write(self.table_id, data)?;

        Ok(())
    }

    pub fn clear_key(&mut self, key: &K) -> DbResult<Option<HashSet<V>>> {
        let log_entry = LogEntry::<&K, &V>::ClearKey(key);
        let data = bincode::serialize(&log_entry)?;
        let ret = self.inner.remove(key);
        self.logger.write(self.table_id, data)?;

        Ok(ret)
    }
}
