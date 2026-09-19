use serde::{Deserialize, Serialize};

use crate::errors::{Error, Result};

pub mod composite_view;
pub mod hashmap;
pub mod hashmap_map;
pub mod hashmap_set;
pub mod option;
pub mod vec;

pub(crate) fn bin_encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(bincode::serde::encode_to_vec(
        value,
        bincode::config::standard(),
    )?)
}

pub(crate) fn bin_decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T> {
    let (value, consumed) =
        bincode::serde::borrow_decode_from_slice(bytes, bincode::config::standard())?;
    if consumed != bytes.len() {
        return Err(Error::TrailingBytes {
            remaining_bytes: bytes.len() - consumed,
        });
    }
    Ok(value)
}
