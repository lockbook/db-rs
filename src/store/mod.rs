pub trait Store {
    // fn boxed(&mut self) -> Box<&mut dyn Store>
    // where
    //     Self: Sized + 'static,
    // {
    //     Box::new(self)
    // }

    fn set_logger(&mut self, log: Log);

    fn handle_event(&mut self, e: Event);
}

const BINCODE_CONFIG: Configuration = bincode::config::standard();

pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, EncodeError> {
    bincode::serde::encode_to_vec(value, BINCODE_CONFIG)
}

pub fn from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DecodeError> {
    bincode::serde::decode_from_slice(bytes, BINCODE_CONFIG).map(|(value, _)| value)
}

pub mod option;

use bincode::{
    config::Configuration,
    error::{DecodeError, EncodeError},
};
use serde::{Serialize, de::DeserializeOwned};

use crate::log::{Event, Log};
