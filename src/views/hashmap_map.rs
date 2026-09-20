use std::{
    borrow::Borrow,
    collections::{HashMap, hash_map},
    hash::Hash,
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{View, errors::Result, log::Log, payload_buffer::PayloadBuffer};

use super::{bin_decode, bin_encode};

#[derive(Serialize, Deserialize)]
enum Diff<K1, K2, V> {
    Insert { key: K1, inner_key: K2, value: V },
    Remove { key: K1, inner_key: K2 },
    CreateKey { key: K1 },
    ClearKey { key: K1 },
    Clear,
}

pub struct DbHashMapMap<K1, K2, V> {
    inner: HashMap<K1, HashMap<K2, V>>,
    pending_events: PayloadBuffer,
    log: Option<Log>,
    last_modified: u64,
}

impl<K1, K2, V> View for DbHashMapMap<K1, K2, V>
where
    K1: Serialize + DeserializeOwned + Eq + Hash,
    K2: Serialize + DeserializeOwned + Eq + Hash,
    V: Serialize + DeserializeOwned,
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
            match bin_decode::<Diff<K1, K2, V>>(event)? {
                Diff::Insert {
                    key,
                    inner_key,
                    value,
                } => {
                    self.inner.entry(key).or_default().insert(inner_key, value);
                }
                Diff::Remove { key, inner_key } => {
                    if let Some(values) = self.inner.get_mut(&key) {
                        values.remove(&inner_key);
                    }
                }
                Diff::CreateKey { key } => {
                    self.inner.insert(key, HashMap::new());
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
                snapshot.push_encoded(&Diff::<_, (), ()>::CreateKey { key })?;
            }
            for (inner_key, value) in values {
                snapshot.push_encoded(&Diff::Insert {
                    key,
                    inner_key,
                    value,
                })?;
            }
        }
        Ok(snapshot.bytes)
    }
}

impl<K1, K2, V> Default for DbHashMapMap<K1, K2, V> {
    fn default() -> Self {
        Self {
            inner: HashMap::new(),
            pending_events: PayloadBuffer::default(),
            log: None,
            last_modified: 0,
        }
    }
}

impl<K1, K2, V> DbHashMapMap<K1, K2, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> hash_map::Iter<'_, K1, HashMap<K2, V>> {
        self.inner.iter()
    }
}

impl<K1: Eq + Hash, K2, V> DbHashMapMap<K1, K2, V> {
    pub fn get<Q>(&self, key: &Q) -> Option<&HashMap<K2, V>>
    where
        K1: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.inner.get(key)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K1: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.inner.contains_key(key)
    }
}

impl<K1, K2, V> DbHashMapMap<K1, K2, V>
where
    K1: Serialize + Eq + Hash,
    K2: Serialize + Eq + Hash,
    V: Serialize,
{
    pub fn insert(&mut self, key: K1, inner_key: K2, value: V) -> Result<Option<V>> {
        let event = bin_encode(&Diff::Insert {
            key: &key,
            inner_key: &inner_key,
            value: &value,
        })?;
        self.pending_events.push(&event);
        Ok(self.inner.entry(key).or_default().insert(inner_key, value))
    }

    pub fn remove(&mut self, key: &K1, inner_key: &K2) -> Result<Option<V>> {
        let Some(values) = self.inner.get_mut(key) else {
            return Ok(None);
        };
        if !values.contains_key(inner_key) {
            return Ok(None);
        }
        let event = bin_encode(&Diff::<_, _, ()>::Remove { key, inner_key })?;
        self.pending_events.push(&event);
        Ok(values.remove(inner_key))
    }

    /// Creates an empty group, replacing any existing group at this key.
    pub fn create_key(&mut self, key: K1) -> Result<Option<HashMap<K2, V>>> {
        let event = bin_encode(&Diff::<_, (), ()>::CreateKey { key: &key })?;
        self.pending_events.push(&event);
        Ok(self.inner.insert(key, HashMap::new()))
    }

    pub fn clear_key(&mut self, key: &K1) -> Result<Option<HashMap<K2, V>>> {
        if !self.inner.contains_key(key) {
            return Ok(None);
        }
        let event = bin_encode(&Diff::<_, (), ()>::ClearKey { key })?;
        self.pending_events.push(&event);
        Ok(self.inner.remove(key))
    }

    pub fn clear(&mut self) -> Result<()> {
        self.pending_events
            .push_encoded(&Diff::<(), (), ()>::Clear)?;
        self.inner.clear();
        Ok(())
    }
}
