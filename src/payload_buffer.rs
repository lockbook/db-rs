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

#[cfg(test)]
mod tests {
    use super::PayloadBuffer;
    use crate::errors::Error;

    #[test]
    fn empty_buffer_has_no_payload() {
        let buffer = PayloadBuffer::default();
        let mut remaining = buffer.bytes.as_slice();

        assert!(matches!(
            PayloadBuffer::head_payload(&mut remaining),
            Ok(None)
        ));
    }

    #[test]
    fn empty_payload_is_distinct_from_empty_buffer() {
        let mut buffer = PayloadBuffer::default();
        buffer.push(b"");
        assert_eq!(buffer.bytes.len(), 8);

        let mut remaining = buffer.bytes.as_slice();
        assert_eq!(
            PayloadBuffer::head_payload(&mut remaining).unwrap(),
            Some(b"".as_slice())
        );
        assert!(matches!(
            PayloadBuffer::head_payload(&mut remaining),
            Ok(None)
        ));
    }

    #[test]
    fn single_payload_round_trip() {
        let mut buffer = PayloadBuffer::default();
        let payload = [0; 10];

        buffer.push(&payload);
        assert_eq!(buffer.bytes.len(), 18);

        let mut remaining = buffer.bytes.as_slice();
        let decoded = PayloadBuffer::head_payload(&mut remaining)
            .unwrap()
            .unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn multiple_payloads_round_trip() {
        let mut buffer = PayloadBuffer::default();
        let payloads: [&[u8]; 5] = [b"one", b"two", b"three", b"four", b"five"];

        for payload in payloads {
            buffer.push(payload);
        }

        let mut remaining = buffer.bytes.as_slice();
        let mut decoded = Vec::new();
        while let Some(payload) = PayloadBuffer::head_payload(&mut remaining).unwrap() {
            decoded.push(payload);
        }

        assert_eq!(decoded, payloads);
    }

    #[test]
    fn truncated_payload_is_incomplete() {
        let mut buffer = PayloadBuffer::default();
        buffer.push(&[0; 10]);
        buffer.bytes.pop();

        let mut remaining = buffer.bytes.as_slice();
        assert!(matches!(
            PayloadBuffer::head_payload(&mut remaining),
            Err(Error::IncompleteLog {
                remaining_bytes: 17
            })
        ));
    }
}
