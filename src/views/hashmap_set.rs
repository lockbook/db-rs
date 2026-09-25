use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet, hash_map},
    hash::Hash,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{View, errors::Result, log::Log, payload_buffer::PayloadBuffer};

use super::{bin_decode, bin_encode};

#[derive(Serialize, Deserialize)]
enum Diff<K, V> {
    Insert { key: K, value: V },
    Remove { key: K, value: V },
    CreateKey { key: K },
    ClearKey { key: K },
    Clear,
}

pub struct DbHashMapSet<K, V> {
    inner: HashMap<K, HashSet<V>>,
    pending_events: PayloadBuffer,
    log: Option<Log>,
    last_modified: u64,
}

impl<K, V> View for DbHashMapSet<K, V>
where
    K: Serialize + DeserializeOwned + Eq + Hash,
    V: Serialize + DeserializeOwned + Eq + Hash,
{
    fn log(&self) -> &Log {
        self.log.as_ref().expect("view has no initialized log")
    }

    fn log_mut(&mut self) -> &mut Log {
        self.log.as_mut().expect("view has no initialized log")
    }

    fn set_log(&mut self, log: Log) {
        self.log = Some(log);
    }

    fn last_modified(&self) -> u64 {
        self.last_modified
    }

    fn handle_events(&mut self, seq_no: u64, mut events: &[u8]) -> Result<()> {
        while let Some(event) = PayloadBuffer::head_payload(&mut events)? {
            match bin_decode::<Diff<K, V>>(event)? {
                Diff::Insert { key, value } => {
                    self.inner.entry(key).or_default().insert(value);
                }
                Diff::Remove { key, value } => {
                    if let Some(values) = self.inner.get_mut(&key) {
                        values.remove(&value);
                    }
                }
                Diff::CreateKey { key } => {
                    self.inner.insert(key, HashSet::new());
                }
                Diff::ClearKey { key } => {
                    self.inner.remove(&key);
                }
                Diff::Clear => self.inner.clear(),
            }
        }
        self.last_modified = seq_no;
        Ok(())
    }

    fn take_pending(&mut self, seq_no: u64) -> Vec<u8> {
        let pending = std::mem::take(&mut self.pending_events).bytes;
        if !pending.is_empty() {
            self.last_modified = seq_no;
        }
        pending
    }

    fn generate_snapshot(&mut self) -> Result<Vec<u8>> {
        let mut snapshot = PayloadBuffer::default();
        for (key, values) in &self.inner {
            if values.is_empty() {
                snapshot.push_encoded(&Diff::<_, ()>::CreateKey { key })?;
            }
            for value in values {
                snapshot.push_encoded(&Diff::Insert { key, value })?;
            }
        }
        Ok(snapshot.bytes)
    }
}

impl<K, V> Default for DbHashMapSet<K, V> {
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
            pending_events: PayloadBuffer::default(),
            log: None,
            last_modified: 0,
        }
    }
}

impl<K, V> DbHashMapSet<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> hash_map::Iter<'_, K, HashSet<V>> {
        self.inner.iter()
    }
}

impl<K: Eq + Hash, V> DbHashMapSet<K, V> {
    pub fn get<Q>(&self, key: &Q) -> Option<&HashSet<V>>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.inner.get(key)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.inner.contains_key(key)
    }
}

impl<K, V> DbHashMapSet<K, V>
where
    K: Serialize + Eq + Hash,
    V: Serialize + Eq + Hash,
{
    pub fn insert(&mut self, key: K, value: V) -> Result<bool> {
        let event = bin_encode(&Diff::Insert {
            key: &key,
            value: &value,
        })?;
        self.pending_events.push(&event);
        Ok(self.inner.entry(key).or_default().insert(value))
    }

    pub fn remove(&mut self, key: &K, value: &V) -> Result<bool> {
        let Some(values) = self.inner.get_mut(key) else {
            return Ok(false);
        };
        if !values.contains(value) {
            return Ok(false);
        }
        let event = bin_encode(&Diff::Remove { key, value })?;
        self.pending_events.push(&event);
        Ok(values.remove(value))
    }

    /// Creates an empty group, replacing any existing group at this key.
    pub fn create_key(&mut self, key: K) -> Result<Option<HashSet<V>>> {
        let event = bin_encode(&Diff::<_, ()>::CreateKey { key: &key })?;
        self.pending_events.push(&event);
        Ok(self.inner.insert(key, HashSet::new()))
    }

    pub fn clear_key(&mut self, key: &K) -> Result<Option<HashSet<V>>> {
        if !self.inner.contains_key(key) {
            return Ok(None);
        }
        let event = bin_encode(&Diff::<_, ()>::ClearKey { key })?;
        self.pending_events.push(&event);
        Ok(self.inner.remove(key))
    }

    pub fn clear(&mut self) -> Result<()> {
        self.pending_events.push_encoded(&Diff::<(), ()>::Clear)?;
        self.inner.clear();
        Ok(())
    }
}
