use std::{
    borrow::Borrow,
    collections::{HashMap, hash_map},
    hash::{BuildHasher, Hash, RandomState},
};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{View, errors::Result, payload_buffer::PayloadBuffer};

use super::{bin_decode, bin_encode};

#[derive(Serialize, Deserialize)]
enum Diff<K, V> {
    Insert { key: K, value: V },
    Remove { key: K },
    Clear,
}

pub struct SHashMap<K, V, S = RandomState> {
    inner: HashMap<K, V, S>,
    pending_events: PayloadBuffer,
}

impl<K, V, S> View for SHashMap<K, V, S>
where
    K: DeserializeOwned + Serialize + Eq + Hash,
    V: DeserializeOwned + Serialize,
    S: BuildHasher + Default,
{
    fn handle_events(&mut self, mut events: &[u8]) -> Result<()> {
        while let Some(event) = PayloadBuffer::head_payload(&mut events)? {
            let diff: Diff<K, V> = bin_decode(event)?;

            match diff {
                Diff::Insert { key, value } => {
                    self.inner.insert(key, value);
                }
                Diff::Remove { key } => {
                    self.inner.remove(&key);
                }
                Diff::Clear => {
                    self.inner.clear();
                }
            }
        }

        Ok(())
    }

    fn take_events(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending_events).bytes
    }

    fn snapshot(&self) -> Result<Vec<u8>> {
        let mut snapshot = PayloadBuffer::default();

        for (key, value) in &self.inner {
            let event = bin_encode(&Diff::Insert { key, value })?;
            snapshot.push(&event);
        }

        Ok(snapshot.bytes)
    }
}

impl<K, V> SHashMap<K, V> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            pending_events: PayloadBuffer::default(),
        }
    }
}

impl<K, V, S> Default for SHashMap<K, V, S>
where
    S: Default,
{
    fn default() -> Self {
        Self {
            inner: HashMap::with_hasher(S::default()),
            pending_events: PayloadBuffer::default(),
        }
    }
}

impl<K, V, S> SHashMap<K, V, S> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn iter(&self) -> hash_map::Iter<'_, K, V> {
        self.inner.iter()
    }
}

impl<K, V, S> SHashMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
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

impl<K, V, S> SHashMap<K, V, S>
where
    K: Eq + Hash + Serialize,
    V: Serialize,
    S: BuildHasher,
{
    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>> {
        let event = bin_encode(&Diff::Insert {
            key: &key,
            value: &value,
        })?;

        self.pending_events.push(&event);
        Ok(self.inner.insert(key, value))
    }

    pub fn remove(&mut self, key: &K) -> Result<Option<V>> {
        if !self.inner.contains_key(key) {
            return Ok(None);
        }

        let event = bin_encode(&Diff::<&K, ()>::Remove { key })?;

        self.pending_events.push(&event);
        Ok(self.inner.remove(key))
    }

    pub fn clear(&mut self) -> Result<()> {
        let event = bin_encode(&Diff::<(), ()>::Clear)?;

        self.pending_events.push(&event);
        self.inner.clear();
        Ok(())
    }
}
