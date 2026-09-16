use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
};

use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    errors::{Error, Result},
    payload_buffer::PayloadBuffer,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct LogEntry<'a> {
    pub(crate) seq_no: u64,
    pub(crate) payload: &'a [u8],
}

pub(crate) fn head_entry<'a>(remaining: &mut &'a [u8]) -> Result<Option<LogEntry<'a>>> {
    let mut rest = *remaining;
    let Some(body) = PayloadBuffer::head_payload(&mut rest)? else {
        return Ok(None);
    };
    let (entry, consumed) =
        bincode::serde::borrow_decode_from_slice(body, bincode::config::standard())?;
    if consumed != body.len() {
        return Err(Error::TrailingBytes {
            remaining_bytes: body.len() - consumed,
        });
    }
    *remaining = rest;
    Ok(Some(entry))
}

pub struct Log {
    file: File,
}

impl Log {
    pub fn init(config: &Config) -> io::Result<Self> {
        let path = config.log_location.join("db.log");
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;

        Ok(Self { file })
    }

    pub fn get_bytes(&mut self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}
