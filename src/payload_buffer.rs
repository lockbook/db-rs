#[derive(Default)]
pub struct PayloadBuffer {
    pub bytes: Vec<u8>,
}

impl PayloadBuffer {
    pub fn push(&mut self, payload: &[u8]) {
        self.bytes
            .extend_from_slice(&(payload.len() as u64).to_be_bytes());
        self.bytes.extend_from_slice(payload);
    }

    /// Advances on success; leaves input unchanged on incomplete framing.
    pub fn head_payload<'a>(remaining: &mut &'a [u8]) -> Result<Option<&'a [u8]>> {
        let buf = *remaining;
        if buf.is_empty() {
            return Ok(None);
        }
        let incomplete = || Error::IncompleteLog {
            remaining_bytes: buf.len(),
        };
        let size = size_of::<u64>();
        let length = buf.get(..size).ok_or_else(incomplete)?;
        let length = usize::try_from(u64::from_be_bytes(length.try_into().unwrap()))
            .map_err(|_| incomplete())?;
        let end = size.checked_add(length).ok_or_else(incomplete)?;
        let payload = buf.get(size..end).ok_or_else(incomplete)?;

        *remaining = &buf[end..];
        Ok(Some(payload))
    }
}
use crate::errors::{Error, Result};
