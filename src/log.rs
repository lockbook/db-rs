use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    mem::size_of,
};

use crate::{config::Config, payload_buffer::PayloadBuffer, types::SeqNo};

pub(crate) struct LogEntry<'a> {
    pub(crate) seq_no: SeqNo,
    pub(crate) payload: &'a [u8],
}

pub(crate) fn head_entry(buf: &[u8]) -> (Option<LogEntry<'_>>, &[u8]) {
    let offset = 0;
    let id_size = size_of::<SeqNo>();

    let Some(seq_no) = buf.get(offset..offset + id_size) else {
        return (None, &buf[buf.len()..]);
    };
    let seq_no = SeqNo::from_be_bytes(seq_no.try_into().unwrap());
    let offset = offset + id_size;

    let (Some(payload), rest) = PayloadBuffer::head_payload(&buf[offset..]) else {
        return (None, &buf[buf.len()..]);
    };

    let entry = LogEntry { seq_no, payload };
    (Some(entry), rest)
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
