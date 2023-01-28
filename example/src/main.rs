use db_rs::config::Config;
use db_rs::lookup::LookupTable;
use db_rs::serializer::Bincode;
use db_rs::single::Single;
use db_rs_derive::Db;

#[derive(Db)]
struct SampleSchemaV1 {
    pub names: LookupTable<String, String, Bincode>,
    pub is_good: Single<bool, Bincode>,
}

fn main() {
    let a = SampleSchemaV1::init(Config::default());
}
//
// impl Db for SampleSchemaV1 {
//     fn init(location: Config) -> Self {
//         let log = Logger::init(location);
//         let log_entries = log.get_entries();
//
//         let mut table_0 = LookupTable::<String, String, Bincode>::init(0, log.clone());
//         let mut table_1 = Single::<bool, Bincode>::init(1, log.clone());
//
//         for entry in log_entries {
//             match entry.table_id {
//                 0 => table_0.handle_event(entry.bytes),
//                 1 => table_1.handle_event(entry.bytes),
//                 _ => todo!(),
//             }
//         }
//         Self {
//             names: table_0,
//             is_good: table_1,
//         }
//     }
//
//     fn get_logger(&mut self) -> &mut Logger {
//         &mut self.names.logger
//     }
// }
