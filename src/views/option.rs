use serde::{Serialize, de::DeserializeOwned};

use crate::{View, errors::Result, log::Log};

use super::{bin_decode, bin_encode};

pub struct DbOption<T> {
    inner: Option<T>,
    pending_events: Vec<u8>,
    log: Option<Log>,
    last_modified: u64,
}

impl<T: Serialize + DeserializeOwned> View for DbOption<T> {
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

    fn handle_events(&mut self, seq_no: u64, events: &[u8]) -> Result<()> {
        self.inner = bin_decode(events)?;
        self.last_modified = seq_no;
        Ok(())
    }

    fn take_pending(&mut self, seq_no: u64) -> Vec<u8> {
        let pending = std::mem::take(&mut self.pending_events);
        if !pending.is_empty() {
            self.last_modified = seq_no;
        }
        pending
    }

    fn generate_snapshot(&mut self) -> Result<Vec<u8>> {
        bin_encode(&self.inner)
    }
}

impl<T> Default for DbOption<T> {
    fn default() -> Self {
        Self {
            inner: None,
            pending_events: Vec::new(),
            log: None,
            last_modified: 0,
        }
    }
}

impl<T> DbOption<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_ref(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    pub fn is_some(&self) -> bool {
        self.inner.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.inner.is_none()
    }
}

impl<T: Serialize> DbOption<T> {
    pub fn replace(&mut self, value: T) -> Result<Option<T>> {
        self.pending_events = bin_encode(&Some(&value))?;
        Ok(self.inner.replace(value))
    }

    pub fn take(&mut self) -> Result<Option<T>> {
        if self.inner.is_none() {
            return Ok(None);
        }

        self.pending_events = bin_encode(&Option::<T>::None)?;
        Ok(self.inner.take())
    }
}
