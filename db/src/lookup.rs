use crate::logger::Logger;
use crate::serializer::Codec;
use crate::table::Table;
use crate::TableId;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

pub struct LookupTable<K, V, C>
where
    K: Hash + Eq,
    V: Hash,
    C: Codec<LogEntry<K, V>>,
{
    table_id: TableId,
    inner: HashMap<K, V>,
    pub(crate) logger: Logger,
    c: PhantomData<C>,
}

pub enum LogEntry<K, V> {
    Insert(K, V),
    Remove(K),
    Clear,
}

impl<K, V, C> Table for LookupTable<K, V, C>
where
    K: Hash + Eq,
    V: Hash,
    C: Codec<LogEntry<K, V>>,
{
    fn init(table_id: TableId, logger: Logger) -> Self {
        Self {
            table_id,
            inner: HashMap::default(),
            logger,
            c: Default::default(),
        }
    }

    fn handle_event(&mut self, bytes: &[u8]) {
        match C::deserialize(bytes) {
            LogEntry::Insert(k, v) => {
                self.insert(k, v);
            }
            LogEntry::Remove(k) => todo!(),
            LogEntry::Clear => self.clear(),
        };
    }
}

impl<K, V, C> LookupTable<K, V, C>
where
    K: Hash + Eq,
    V: Hash,
    C: Codec<LogEntry<K, V>>,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let log_entry = LogEntry::Insert(key, value);
        let data = C::serialize(&log_entry);

        let ret = if let LogEntry::Insert(key, value) = log_entry {
            self.inner.insert(key, value)
        } else {
            None
        };

        self.logger.write(self.table_id, &data);
        ret
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.inner.get(k)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        let log_entry = LogEntry::Clear;
        let data = C::serialize(&log_entry);
        self.logger.write(self.table_id, &data);
    }
}
