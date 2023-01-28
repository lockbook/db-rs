pub trait Codec<T> {
    fn serialize(t: &T) -> Vec<u8>;
    fn deserialize(b: &[u8]) -> T;
}

pub struct Bincode {}

impl<T> Codec<T> for Bincode {
    fn serialize(t: &T) -> Vec<u8> {
        todo!()
    }

    fn deserialize(b: &[u8]) -> T {
        todo!()
    }
}
