use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    View,
    errors::{Error, Result},
    log::Log,
    payload_buffer::PayloadBuffer,
};

use super::{bin_decode, bin_encode};

#[derive(Serialize, Deserialize)]
enum Diff<T> {
    Push { value: T },
    Insert { index: usize, value: T },
    Remove { index: usize },
    Clear,
}

pub struct DbVec<T> {
    inner: Vec<T>,
    pending_events: PayloadBuffer,
    log: Option<Log>,
    last_modified: u64,
}

impl<T: Serialize + DeserializeOwned> View for DbVec<T> {
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
            match bin_decode::<Diff<T>>(event)? {
                Diff::Push { value } => self.inner.push(value),
                Diff::Insert { index, value } => {
                    if index > self.inner.len() {
                        return Err(Error::IndexOutOfBounds {
                            index,
                            len: self.inner.len(),
                        });
                    }
                    self.inner.insert(index, value);
                }
                Diff::Remove { index } => {
                    if index >= self.inner.len() {
                        return Err(Error::IndexOutOfBounds {
                            index,
                            len: self.inner.len(),
                        });
                    }
                    self.inner.remove(index);
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
        for value in &self.inner {
            snapshot.push_encoded(&Diff::Push { value })?;
        }
        Ok(snapshot.bytes)
    }
}

impl<T> Default for DbVec<T> {
    fn default() -> Self {
        Self {
            inner: Vec::new(),
            pending_events: PayloadBuffer::default(),
            log: None,
            last_modified: 0,
        }
    }
}

impl<T> DbVec<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }

    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }
}

impl<T: Serialize> DbVec<T> {
    pub fn push(&mut self, value: T) -> Result<()> {
        let event = bin_encode(&Diff::Push { value: &value })?;
        self.pending_events.push(&event);
        self.inner.push(value);
        Ok(())
    }

    pub fn insert(&mut self, index: usize, value: T) -> Result<()> {
        if index > self.inner.len() {
            return Err(Error::IndexOutOfBounds {
                index,
                len: self.inner.len(),
            });
        }
        let event = bin_encode(&Diff::Insert {
            index,
            value: &value,
        })?;
        self.pending_events.push(&event);
        self.inner.insert(index, value);
        Ok(())
    }

    pub fn remove(&mut self, index: usize) -> Result<T> {
        if index >= self.inner.len() {
            return Err(Error::IndexOutOfBounds {
                index,
                len: self.inner.len(),
            });
        }
        self.pending_events
            .push_encoded(&Diff::<()>::Remove { index })?;
        Ok(self.inner.remove(index))
    }

    pub fn pop(&mut self) -> Result<Option<T>> {
        let Some(index) = self.inner.len().checked_sub(1) else {
            return Ok(None);
        };
        self.remove(index).map(Some)
    }

    pub fn clear(&mut self) -> Result<()> {
        self.pending_events.push_encoded(&Diff::<()>::Clear)?;
        self.inner.clear();
        Ok(())
    }
}
