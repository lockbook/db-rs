pub type Id = usize;

pub trait Db {
    /// The in-memory view this db wraps. Named as an associated type so
    /// `write_tx`'s return can mention it without `Db` itself being generic.
    type View: Schema;

    fn start_db(&self, config: Config);
    fn write_tx(&self) -> TxGuard<'_, Self::View>;
}

impl<V: Schema> Db for Shard<V> {
    type View = V;

    fn start_db(&self, config: Config) {
        let log = Logger::init(&config).unwrap();
        let events = self.log.get_events();
        let mut tables = self.view.write().unwrap();
        let mut tables = tables.stores();

        for (idx, table) in tables.iter_mut().enumerate() {
            let mut log = log.clone();
            log.shard_id = self.id;
            log.table_id = idx;
            table.set_logger(log);
        }

        for e in events {
            match tables.get_mut(e.table_id) {
                Some(table) => table.handle_event(&e.data),
                None => todo!(),
            }
        }
    }

    fn write_tx(&self) -> TxGuard<'_, V> {
        let view = self.view.write().unwrap();

        TxGuard { view, log: &self.log }
    }
}

pub struct TxGuard<'a, V: Schema> {
    view: RwLockWriteGuard<'a, V>,
    log: &'a Logger,
}

impl<V: Schema> Deref for TxGuard<'_, V> {
    type Target = V;

    fn deref(&self) -> &V {
        &self.view
    }
}

impl<V: Schema> DerefMut for TxGuard<'_, V> {
    fn deref_mut(&mut self) -> &mut V {
        &mut self.view
    }
}

impl<V: Schema> Drop for TxGuard<'_, V> {
    fn drop(&mut self) {
        self.log.commit();
    }
}

#[derive(Default)]
pub struct Shard<V: Schema> {
    pub view: Arc<RwLock<V>>,

    id: Id,
    log: Logger,
}

pub trait Schema {
    fn stores(&mut self) -> Vec<&mut dyn Store>;
}

pub mod config;
pub mod log;
pub mod store;

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock, RwLockWriteGuard};

use crate::{config::Config, log::Logger, store::Store};
