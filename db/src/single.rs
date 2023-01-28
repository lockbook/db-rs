use crate::logger::Logger;
use crate::serializer::Codec;
use crate::table::Table;
use crate::TableId;
use std::marker::PhantomData;

pub struct Single<V, C>
where
    C: Codec<Option<V>>,
{
    table_id: TableId,
    inner: Option<V>,
    pub logger: Logger,
    c: PhantomData<C>,
}

impl<V, C> Table for Single<V, C>
where
    C: Codec<Option<V>>,
{
    fn init(table_id: TableId, logger: Logger) -> Self {
        Self {
            table_id,
            inner: None,
            logger,
            c: Default::default(),
        }
    }

    fn handle_event(&mut self, bytes: &[u8]) {
        match C::deserialize(bytes) {
            Some(v) => {
                self.insert(v);
            }
            None => {
                self.clear();
            }
        };
    }
}

impl<V, C> Single<V, C>
where
    C: Codec<Option<V>>,
{
    pub fn insert(&mut self, value: V) -> Option<V> {
        let log_entry = Some(value);
        let data = C::serialize(&log_entry);

        let ret = if let Some(value) = log_entry {
            self.inner.replace(value)
        } else {
            None
        };

        self.logger.write(self.table_id, &data);

        ret
    }

    pub fn get(&self) -> Option<&V> {
        self.inner.as_ref()
    }

    pub fn clear(&mut self) -> Option<V> {
        let log_entry = None;
        let data = C::serialize(&log_entry);
        let ret = self.inner.take();
        self.logger.write(self.table_id, &data);
        ret
    }
}
