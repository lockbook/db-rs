use crate::logger::Logger;
use crate::serializer::Codec;
use std::marker::PhantomData;

pub struct Single<V, C, L>
where
    C: Codec<Option<V>>,
    L: Logger,
{
    inner: Option<V>,
    logger: L,
    c: PhantomData<C>,
}

impl<V, C, L> Single<V, C, L>
where
    C: Codec<Option<V>>,
    L: Logger,
{
    pub fn insert(&mut self, value: V) -> Option<V> {
        let log_entry = Some(value);
        let data = C::serialize(&log_entry);

        let ret = if let Some(value) = log_entry {
            self.inner.replace(value)
        } else {
            None
        };

        self.logger.write(&data);

        ret
    }

    pub fn get(&self) -> Option<&V> {
        self.inner.as_ref()
    }

    pub fn clear(&mut self) -> Option<V> {
        let log_entry = None;
        let data = C::serialize(&log_entry);
        let ret = self.inner.take();
        self.logger.write(&data);
        ret
    }
}
