use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    log::{Event, Log},
    store::{Store, from_bytes, to_bytes},
};

#[derive(Default)]
pub struct SOption<T> {
    log: Option<Log>,
    data: Option<T>,
}

impl<T> SOption<T> {
    pub fn get(&self) -> &Option<T> {
        &self.data
    }
}

impl<T: Serialize> SOption<T> {
    /// Updates the in-memory view and returns the bytes destined for the log.
    pub fn set(&mut self, value: Option<T>) -> Vec<u8> {
        let bytes = to_bytes(&value).unwrap();
        self.data = value;
        bytes
    }
}

impl<T: DeserializeOwned + 'static> Store for SOption<T> {
    fn handle_event(&mut self, e: Event) {
        self.data = from_bytes(&e.data).unwrap();
    }

    fn set_logger(&mut self, log: Log) {
        self.log = Some(log);
    }
}
