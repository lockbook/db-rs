use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    log::Logger,
    store::{Store, from_bytes, to_bytes},
};

#[derive(Default)]
pub struct SOption<T> {
    log: Logger,
    data: Option<T>,
}

impl<T> SOption<T> {
    pub fn get(&self) -> &Option<T> {
        &self.data
    }
}

impl<T: Serialize> SOption<T> {
    /// Updates the in-memory view and returns the bytes destined for the log.
    pub fn set(&mut self, value: Option<T>) -> Option<T> {
        let bytes = to_bytes(&value).unwrap();
        self.log.append(bytes);
        let old = self.data.take();
        self.data = value;
        old
    }
}

impl<T: DeserializeOwned + 'static> Store for SOption<T> {
    fn handle_event(&mut self, data: &[u8]) {
        self.data = from_bytes(data).unwrap();
    }

    fn set_logger(&mut self, log: Logger) {
        self.log = log;
    }
}
