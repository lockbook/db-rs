use crate::table::Table;
use crate::{DbResult, Logger, TableId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// A special case of [crate::lookup::LookupTable] where the value of the [HashMap] is a `Vec<V>`.
#[derive(Debug)]
#[cfg_attr(feature = "clone", derive(Clone))]
pub struct LookupList<K, V>
where
    K: Hash + Eq + Serialize,
    V: Serialize + Eq,
{
    table_id: TableId,
    inner: HashMap<K, Vec<V>>,
    pub logger: Logger,
}

#[derive(Serialize, Deserialize)]
pub enum LogEntry<K, V> {
    Push(K, V),
    Remove(K, usize),
    CreateKey(K),
    ClearKey(K),
    Clear,
}

impl<K, V> Table for LookupList<K, V>
where
    K: Hash + Eq + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + Eq + Hash,
{
    fn init(table_id: TableId, logger: Logger) -> Self {
        Self { table_id, inner: HashMap::default(), logger }
    }

    fn handle_event(&mut self, bytes: &[u8]) -> DbResult<()> {
        match bincode::deserialize::<LogEntry<K, V>>(bytes)? {
            LogEntry::Push(k, v) => {
                self.push_inner(k, v);
            }
            LogEntry::Remove(k, idx) => {
                if let Some(vec) = self.inner.get_mut(&k) {
                    vec.remove(idx);
                }
            }
            LogEntry::CreateKey(k) => {
                self.inner.insert(k, Vec::new());
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
                let data = bincode::serialize(&LogEntry::Push(k, v))?;
                let mut data = Logger::log_entry(self.table_id, data);
                repr.append(&mut data);
            }
        }

        Ok(repr)
    }
}

impl<K, V> LookupList<K, V>
where
    K: Hash + Eq + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned + Eq + Hash,
{
    pub(crate) fn push_inner(&mut self, k: K, v: V) {
        if let Some(vec) = self.inner.get_mut(&k) {
            vec.push(v);
        } else {
            self.inner.insert(k, vec![v]);
        }
    }
    pub fn push(&mut self, k: K, v: V) -> DbResult<()> {
        let log_entry = LogEntry::Push(&k, &v);
        let data = bincode::serialize(&log_entry)?;
        self.push_inner(k, v);
        self.logger.write(self.table_id, data)?;
        Ok(())
    }

    pub fn create_key(&mut self, key: K) -> DbResult<Option<Vec<V>>> {
        let log_entry = LogEntry::<&K, &V>::CreateKey(&key);
        let data = bincode::serialize(&log_entry)?;

        let ret = self.inner.insert(key, Vec::new());

        self.logger.write(self.table_id, data)?;
        Ok(ret)
    }

    pub fn remove(&mut self, key: &K, idx: usize) -> DbResult<bool> {
        if let Some(vec) = self.inner.get_mut(key) {
            let log_entry = LogEntry::Remove::<&K, &V>(key, idx);
            let data = bincode::serialize(&log_entry)?;
            self.logger.write(self.table_id, data)?;
            vec.remove(idx);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get(&self) -> &HashMap<K, Vec<V>> {
        &self.inner
    }

    pub fn clear(&mut self) -> DbResult<()> {
        self.inner.clear();
        let log_entry = LogEntry::<K, V>::Clear;
        let data = bincode::serialize(&log_entry)?;
        self.logger.write(self.table_id, data)?;

        Ok(())
    }

    pub fn clear_key(&mut self, key: &K) -> DbResult<Option<Vec<V>>> {
        let log_entry = LogEntry::<&K, &V>::ClearKey(key);
        let data = bincode::serialize(&log_entry)?;
        let ret = self.inner.remove(key);
        self.logger.write(self.table_id, data)?;

        Ok(ret)
    }
}
