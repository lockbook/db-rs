use crate::logger::Logger;
use crate::serializer::Codec;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

pub struct LookupTable<K, V, C, L>
where
    K: Hash + Eq,
    V: Hash,
    C: Codec<LogEntry<K, V>>,
    L: Logger,
{
    inner: HashMap<K, V>,
    logger: L,
    c: PhantomData<C>,
}

pub enum LogEntry<K, V> {
    Insert(K, V),
    Remove(K),
    Clear,
}

impl<K, V, C, L> LookupTable<K, V, C, L>
where
    K: Hash + Eq,
    V: Hash,
    C: Codec<LogEntry<K, V>>,
    L: Logger,
{
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let log_entry = LogEntry::Insert(key, value);
        let data = C::serialize(&log_entry);

        let ret = if let LogEntry::Insert(key, value) = log_entry {
            self.inner.insert(key, value)
        } else {
            None
        };

        self.logger.write(&data);
        ret
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.inner.get(k)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        let log_entry = LogEntry::Clear;
        let data = C::serialize(&log_entry);
        self.logger.write(&data);
    }
}
