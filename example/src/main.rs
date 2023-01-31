use db_rs::config::Config;
use db_rs::lookup::LookupTable;
use db_rs::single::Single;
use db_rs_derive::Db;

#[derive(Db)]
struct SampleSchemaV1 {
    pub names: LookupTable<String, String>,
    pub is_good: Single<bool>,
}

fn main() {
    let mut a = SampleSchemaV1::init(Config::in_folder("/tmp/test/")).unwrap();
    a.names.clear().unwrap();
}
