use db_rs::{
    Db, Schema, Shard,
    config::Config,
    store::{Store, option::SOption},
};

/// You have an app, it has a db
#[derive(Default)]
struct CustomerApp {
    db: Shard<CustomerView>,
}

/// Describe the db in terms of it's in memory views
/// these are persisted by an append only log
#[derive(Default)]
struct CustomerView {
    account: SOption<String>,
}

/// Describe your schema to the database
impl Schema for CustomerView {
    fn stores(&mut self) -> Vec<&mut dyn Store> {
        vec![&mut self.account]
    }
}

fn main() {
    // initialize your application state
    let app = CustomerApp::default();

    // this will actually read the log and populate your structures
    app.db.start_db(Config {});

    // writes update the view and hand back the bytes for the log
    let log_entry = app
        .db
        .view
        .write()
        .unwrap()
        .account
        .set(Some("parth".to_string()));
}
