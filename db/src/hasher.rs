use std::hash::{BuildHasher, Hasher};

/// An identity [`Hasher`] for [`uuid::Uuid`] keys.
///
/// UUIDs already contain enough randomness to be used directly as hash values,
/// so hashing them again is pure overhead. This hasher copies the first 8 bytes
/// of the UUID into a `u64` and returns it as the hash.
#[derive(Default, Clone, Copy)]
pub struct UuidIdentityHasher {
    hash: u64,
}

impl Hasher for UuidIdentityHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        if bytes.len() >= 16 {
            self.hash = u64::from_ne_bytes(bytes[..8].try_into().unwrap());
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct UuidIdentityHasherBuilder;

impl BuildHasher for UuidIdentityHasherBuilder {
    type Hasher = UuidIdentityHasher;

    fn build_hasher(&self) -> Self::Hasher {
        UuidIdentityHasher::default()
    }
}
