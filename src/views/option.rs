use serde::{Serialize, de::DeserializeOwned};

use crate::{View, errors::Result, log::Log};

use super::{bin_decode, bin_encode};

pub struct DbOption<T> {
    inner: Option<T>,
    pending_events: Vec<u8>,
    log: Option<Log>,
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

    fn handle_events(&mut self, events: &[u8]) -> Result<()> {
        self.inner = bin_decode(events)?;
        Ok(())
    }

    fn take_events(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.pending_events)
    }

    fn snapshot_bytes(&self) -> Result<Vec<u8>> {
        bin_encode(&self.inner)
    }
}

impl<T> Default for DbOption<T> {
    fn default() -> Self {
        Self {
            inner: None,
            pending_events: Vec::new(),
            log: None,
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
