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

    /// Empty or incomplete input returns None and an empty remainder.
    pub fn head_payload(buf: &[u8]) -> (Option<&[u8]>, &[u8]) {
        let size = size_of::<u64>();
        let Some(length) = buf.get(..size) else {
            return (None, &buf[buf.len()..]);
        };
        let Ok(length) = usize::try_from(u64::from_be_bytes(length.try_into().unwrap())) else {
            return (None, &buf[buf.len()..]);
        };
        let Some(end) = size.checked_add(length) else {
            return (None, &buf[buf.len()..]);
        };
        let Some(payload) = buf.get(size..end) else {
            return (None, &buf[buf.len()..]);
        };

        (Some(payload), &buf[end..])
    }
}
