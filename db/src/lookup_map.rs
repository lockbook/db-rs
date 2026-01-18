use crate::table::Table;
use crate::{DbResult, Logger, TableId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;

/// A special case of [crate::lookup::LookupTable] where the value of the [HashMap] is another `HashMap<K2, V>`.
#[derive(Debug)]
#[cfg_attr(feature = "clone", derive(Clone))]
pub struct LookupMap<K1, K2, V>
where
    K1: Hash + Eq + Serialize,
    K2: Hash + Eq + Serialize,
    V: Serialize + DeserializeOwned,
{
    table_id: TableId,
    inner: HashMap<K1, HashMap<K2, V>>,
    pub logger: Logger,
}

#[derive(Serialize, Deserialize)]
pub enum LogEntry<K1, K2, V> {
    Insert(K1, K2, V),
    Remove(K1, K2),
    CreateKey(K1),
    ClearKey(K1),
    Clear,
}

impl<K1, K2, V> Table for LookupMap<K1, K2, V>
where
    K1: Hash + Eq + Serialize + DeserializeOwned,
    K2: Hash + Eq + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    fn init(table_id: TableId, logger: Logger) -> Self {
        Self { table_id, inner: HashMap::default(), logger }
    }

    fn handle_event(&mut self, bytes: &[u8]) -> DbResult<()> {
        match bincode::deserialize::<LogEntry<K1, K2, V>>(bytes)? {
            LogEntry::Insert(k1, k2, v) => {
                self.insert_inner(k1, k2, v);
            }
            LogEntry::Remove(k1, k2) => {
                if let Some(map) = self.inner.get_mut(&k1) {
                    map.remove(&k2);
                };
            }
            LogEntry::CreateKey(k1) => {
                self.inner.insert(k1, HashMap::new());
            }
            LogEntry::ClearKey(k1) => {
                self.inner.remove(&k1);
            }
            LogEntry::Clear => {
                self.inner.clear();
            }
        };

        Ok(())
    }

    fn compact_repr(&self) -> DbResult<Vec<u8>> {
        let mut repr = vec![];
        for (k1, values) in &self.inner {
            if values.is_empty() {
                let data = bincode::serialize(&LogEntry::<&K1, &K2, &V>::CreateKey(k1))?;
                let mut data = Logger::log_entry(self.table_id, data);
                repr.append(&mut data);
                continue;
            }
            for (k2, v) in values {
                let data = bincode::serialize(&LogEntry::Insert(k1, k2, v))?;
                let mut data = Logger::log_entry(self.table_id, data);
                repr.append(&mut data);
            }
        }

        Ok(repr)
    }
}

impl<K1, K2, V> LookupMap<K1, K2, V>
where
    K1: Hash + Eq + Serialize + DeserializeOwned,
    K2: Hash + Eq + Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    pub(crate) fn insert_inner(&mut self, k1: K1, k2: K2, v: V) -> Option<V> {
        if let Some(map) = self.inner.get_mut(&k1) {
            map.insert(k2, v)
        } else {
            let mut map = HashMap::new();
            map.insert(k2, v);
            self.inner.insert(k1, map);
            None
        }
    }

    pub fn insert(&mut self, k1: K1, k2: K2, v: V) -> DbResult<Option<V>> {
        let log_entry = LogEntry::Insert(&k1, &k2, &v);
        let data = bincode::serialize(&log_entry)?;
        let ret = self.insert_inner(k1, k2, v);

        self.logger.write(self.table_id, data)?;
        Ok(ret)
    }

    pub fn create_key(&mut self, key: K1) -> DbResult<Option<HashMap<K2, V>>> {
        let log_entry = LogEntry::<&K1, K2, &V>::CreateKey(&key);
        let data = bincode::serialize(&log_entry)?;

        let ret = self.inner.insert(key, HashMap::new());

        self.logger.write(self.table_id, data)?;
        Ok(ret)
    }

    pub fn remove(&mut self, k1: &K1, k2: &K2) -> DbResult<Option<V>> {
        if let Some(map) = self.inner.get_mut(k1) {
            let log_entry = LogEntry::Remove::<&K1, &K2, V>(k1, k2);
            let data = bincode::serialize(&log_entry)?;
            self.logger.write(self.table_id, data)?;
            Ok(map.remove(k2))
        } else {
            Ok(None)
        }
    }

    pub fn clear_key(&mut self, k1: &K1) -> DbResult<Option<HashMap<K2, V>>> {
        let log_entry = LogEntry::ClearKey::<&K1, K2, V>(k1);
        let data = bincode::serialize(&log_entry)?;

        let ret = self.inner.remove(&k1);

        self.logger.write(self.table_id, data)?;

        Ok(ret)
    }

    pub fn get(&self) -> &HashMap<K1, HashMap<K2, V>> {
        &self.inner
    }

    pub fn clear(&mut self) -> DbResult<()> {
        self.inner.clear();
        let log_entry = LogEntry::<K1, K2, V>::Clear;
        let data = bincode::serialize(&log_entry)?;
        self.logger.write(self.table_id, data)?;

        Ok(())
    }
}
