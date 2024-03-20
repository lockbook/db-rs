// // test that two concorrent processes don't destroy the log and take turns
// // test that they update each other's indexes
//
// use db_rs::Single;
// use db_rs_derive::Schema;
//
// #[derive(Schema)]
// pub struct SingleSchema {
//     table1: Single<u8>,
//     table2: Single<String>,
//     table3: Single<u32>,
//     table4: Single<Vec<u8>>,
// }
//
// #[test]
// fn test_events() {}
