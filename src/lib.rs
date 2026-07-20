use std::sync::{Arc, RwLock};

pub struct Config {}

trait Db {
    fn start_db(&self, config: Config);
}

pub struct Event {
    pub shard_id: usize,
    pub table_id: usize,
    pub data: Vec<u8>,
}

impl<V: Schema> Db for Shard<V> {
    fn start_db(&self, config: Config) {
        let events: Vec<Event> = vec![];
        let mut tables = self.view.write().unwrap().tables();

        for e in events {
            match tables.get_mut(e.table_id) {
                Some(table) => table.handle_event(e),
                None => todo!(),
            }
        }
    }
}

pub struct Shard<V: Schema> {
    view: Arc<RwLock<V>>,
}

pub trait Schema {
    fn tables(&mut self) -> Vec<Box<dyn Table>>;
}

pub trait Table {
    fn boxed(&self) -> Box<dyn Table>
    where
        Self: Clone + Sized + 'static,
    {
        Box::new(self.clone())
    }

    fn handle_event(&mut self, e: Event);
}

pub struct Logger {}

impl Logger {
    fn append(&self, bytes: &Vec<u8>) {}
}

#[derive(Default)]
pub struct DOption<T> {
    data: Arc<Option<T>>,
}

impl<T> Clone for DOption<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl<T: 'static> Table for DOption<T> {
    fn handle_event(&mut self, e: Event) {
        self.data = Arc::new(None);
    }
}

#[derive(Default)]
struct CustomerView {
    account: DOption<String>,
}

impl Schema for CustomerView {
    fn tables(&mut self) -> Vec<Box<dyn Table>> {
        vec![self.account.boxed()]
    }
}

struct CustomerApp {
    db: Shard<CustomerView>,
}

pub fn init() {
    let app = CustomerApp {
        db: Shard {
            view: Default::default()
        }
    };

    app.db.start_db(Config {  });

    // app.db.view.write().unwrap().account;
}
