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
    let a = SampleSchemaV1::init(Config::in_folder("/tmp/test/"));
}
