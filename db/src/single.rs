use crate::errors::DbResult;
use crate::logger::Logger;
use crate::table::Table;
use crate::TableId;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct Single<V>
where
    V: Serialize + DeserializeOwned,
{
    table_id: TableId,
    inner: Option<V>,
    pub logger: Logger,
}

impl<V> Table for Single<V>
where
    V: Serialize + DeserializeOwned,
{
    fn init(table_id: TableId, logger: Logger) -> Self {
        Self {
            table_id,
            inner: None,
            logger,
        }
    }

    fn handle_event(&mut self, bytes: &[u8]) -> DbResult<()> {
        match bincode::deserialize(bytes)? {
            Some(v) => {
                self.insert(v)?;
            }
            None => {
                self.clear()?;
            }
        };

        Ok(())
    }
}

impl<V> Single<V>
where
    V: Serialize + DeserializeOwned,
{
    pub fn insert(&mut self, value: V) -> DbResult<Option<V>> {
        let log_entry = Some(value);
        let data = bincode::serialize(&log_entry)?;

        let ret = if let Some(value) = log_entry {
            self.inner.replace(value)
        } else {
            None
        };

        self.logger.write(self.table_id, &data);

        Ok(ret)
    }

    pub fn get(&self) -> Option<&V> {
        self.inner.as_ref()
    }

    pub fn clear(&mut self) -> DbResult<Option<V>> {
        let log_entry = Option::<V>::None;
        let data = bincode::serialize(&log_entry)?;
        let ret = self.inner.take();
        self.logger.write(self.table_id, &data);
        Ok(ret)
    }
}
