use serde::{Serialize, de::DeserializeOwned};

use crate::errors::{Error, Result};

pub mod hashmap;

pub(crate) fn bin_encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(bincode::serde::encode_to_vec(
        value,
        bincode::config::standard(),
    )?)
}

pub(crate) fn bin_decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let (value, consumed) = bincode::serde::decode_from_slice(bytes, bincode::config::standard())?;
    if consumed != bytes.len() {
        return Err(Error::TrailingBytes {
            remaining_bytes: bytes.len() - consumed,
        });
    }
    Ok(value)
}
