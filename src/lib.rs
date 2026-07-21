pub trait Db {
    fn start_db(&self, config: Config);
}

impl<V: Schema> Db for Shard<V> {
    fn start_db(&self, config: Config) {
        let events = self.log.get_events();
        let mut tables = self.view.write().unwrap();
        let mut tables = tables.stores();

        for e in events {
            match tables.get_mut(e.table_id) {
                Some(table) => table.handle_event(e),
                None => todo!(),
            }
        }
    }
}

#[derive(Default)]
pub struct Shard<V: Schema> {
    log: Log,
    pub view: Arc<RwLock<V>>,
}

pub trait Schema {
    fn stores(&mut self) -> Vec<&mut dyn Store>;
}

pub mod config;
pub mod log;
pub mod store;

use std::sync::{Arc, RwLock};

use crate::{
    config::Config,
    log::{Event, Log},
    store::Store,
};
