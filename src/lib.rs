use crate::config::Config;
use crate::logger::Logger;
use crate::lookup::LookupTable;
use crate::serializer::Bincode;
use crate::single::Single;
use crate::table::Table;

mod config;
mod logger;
mod lookup;
mod serializer;
mod single;
mod table;

pub trait Db {
    fn init(location: Config) -> Self;
    fn get_logger(&mut self) -> &mut Logger;
    fn begin_transaction(&mut self) {
        self.get_logger().begin_tx();
    }
}

// table types
pub type TableId = u8;
pub type ByteCount = u32;

pub struct SampleSchemaV1 {
    pub names: LookupTable<String, String, Bincode>,
    pub is_good: Single<bool, Bincode>,
}

impl Db for SampleSchemaV1 {
    fn init(location: Config) -> Self {
        let log = Logger::init(location);
        let log_entries = log.get_entries();

        let mut table_0 = LookupTable::<String, String, Bincode>::init(0, log.clone());
        let mut table_1 = Single::<bool, Bincode>::init(1, log.clone());

        for entry in log_entries {
            match entry.table_id {
                0 => table_0.handle_event(entry.bytes),
                1 => table_1.handle_event(entry.bytes),
                _ => todo!(),
            }
        }
        Self {
            names: table_0,
            is_good: table_1,
        }
    }

    fn get_logger(&mut self) -> &mut Logger {
        &mut self.names.logger
    }
}
