# db-rs

An ergonomic, embedded, single-threaded database for Rustaceans.

## Strengths

-   Define a schema in Rust.
-   Use **your** types in the database as long as they implement `Serialize` and `Deserialize`. You don't have to fuss around
    with converting your data to database-specific types.
-   All your database interactions are typesafe. When you type `db.`, your tooling will suggest a list of your tables. When you
    select a table, you'll be greeted with that table-type's contract populated with your types. No need to wrap your db
    in a handwritten type safe contract.
-   Supports a variety of simple data-structures, including LookupTables, Lists, and many more. Implementing your own
    table types is trivial.
-   All table mutations are persisted to an append only log using the fast & compact bincode representation of your types.
-   You can `begin_transaction()`s to express atomic updates to multiple tables.

## Quickstart

Add the following to your `Cargo.toml`:

```toml
db-rs = "0.1.13"
db-rs-derive = "0.1.13"
```

Define your schema:

```rust
use db_rs_derive::Schema;
use db_rs::{Single, LookupTable};

#[derive(Schema)]
struct SchemaV1 {
    owner: Single<Username>,
    admins: List<Username>,
    users: LookupTable<Username, Account>,
}
```

Initialize your DB:

```rust
use db_rs::Db;
use db_rs::Config;

fn main() {
    let mut db = SchemaV1::init(Config::in_folder("/tmp/test/"))?;
    db.owner.insert("Parth".to_string())?;
}
```

## Active areas of thought and research

-   Because the db implementation (like redis) is single threaded, it forces you to achieve application throughput via low
    latency rather than concurrency. Currently, this suits our needs. Simply being embedded gives us more than enough
    throughput compared to something like Postgres. For use in a server-style setting put the database in
    an `Arc<Mutex<>>`.
-   The database offers no tools at the moment to define integrity constraints beyond what the Rust type system implicitly
    enforces (non-null for instance). At the moment for us, this is simply an application side concern.

## Used by

-   [Lockbook](https://github.com/lockbook/lockbook)
