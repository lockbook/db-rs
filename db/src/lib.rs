//! An ergonomic, embedded, single-threaded database for Rustaceans.
//!
//! ## Strengths
//!
//! -   Define a schema in Rust.
//! -   Use **your** types in the database as long as they implement `Serialize` and `Deserialize`. You don't have to fuss around
//!     with converting your data to database-specific types.
//! -   All your database interactions are typesafe. When you type `db.`, your tooling will suggest a list of your tables. When you
//!     select a table, you'll be greeted with that table-type's contract populated with your types. No need to wrap your db
//!     in a handwritten type safe contract.
//! -   Supports a variety of simple data-structures, including LookupTables, Lists, and many more. Implementing your own
//!     table types is trivial.
//! -   All table mutations are persisted to an append only log using the fast & compact bincode representation of your types.
//! -   You can `begin_transaction()`s to express atomic updates to multiple tables.
//!
//! ## Quickstart
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! db-rs = "0.2.1"
//! db-rs-derive = "0.2.1"
//! ```
//!
//! Define your schema:
//!
//! ```ignore
//! use db_rs_derive::Schema;
//! use db_rs::{Single, List, LookupTable};
//!
//! #[derive(Schema)]
//! struct SchemaV1 {
//!     owner: Single<Username>,
//!     admins: List<Username>,
//!     users: LookupTable<Username, Account>,
//! }
//! ```
//!
//! Initialize your DB:
//!
//! ```ignore
//! use db_rs::Db;
//! use db_rs::Config;
//!
//! let mut db = SchemaV1::init(Config::in_folder("/tmp/test/"))?;
//! db.owner.insert("Parth".to_string())?;
//!
//! println!("{}", db.owner.data().unwrap());
//! ```
//!
//! ## Table Types
//!
//! Each table has an in-memory representation and a corresponding log entry format. For instance
//! [List]'s in memory format is a [Vec], and you can look at it's corresponding [list::LogEntry]
//! to see how writes will be written to disk.
//!
//! Tables that start with `Lookup` have a `HashMap` as part of their in memory format.
//! [LookupTable] is the most general form, while [LookupList] and [LookupSet] are specializations
//! for people who want `HashMap<K, Vec<V>>` or `HashMap<K, HashSet<V>>`. Their reason for
//! existence is better log performance in the case of small modifications to the `Vec` or
//! `HashSet` in question (see [lookup_list::LogEntry] or [lookup_set::LogEntry]).
//!
//!
//! ## Log Compaction
//!
//! At any point you can call [Db::compact_log] on your database. This will atomically write a
//! compact representation of all your current tables. For example if there's a key in a
//! LookupTable that was written to many times, the compact representation will only contain the
//! last value. Each table type descibes it's own compact representation.
//!
//! If your database is in an `Arc<Mutex>>` you can additionally use the [BackgroundCompacter]
//! which will perform compactions periodically in a separate thread.
//!
//! ## TXs and Batch Writing
//!
//! You can [Db::begin_transaction] which will allow you to express batch operations that can be
//! discarded as a set if your program is interrupted. Presently there is no way to abort a
//! transaction. TXs are also a mechanism for batch writing, log entries are kept in memory until
//! the transaction completes and written once to disk.
//!
//! ## Active areas of thought and research
//!
//! -   Because the db implementation (like redis) is single threaded, it forces you to achieve application throughput via low
//!     latency rather than concurrency. Currently, this suits our needs. Simply being embedded gives us more than enough
//!     throughput compared to something like Postgres. For use in a server-style setting put the database in
//!     an `Arc<Mutex<>>`.
//! -   The database offers no tools at the moment to define integrity constraints beyond what the Rust type system implicitly
//!     enforces (non-null for instance). At the moment for us, this is simply an application side concern.
//!
//! ## Features
//!
//! `clone` - derive clone on all table types. Consistency between cloned database is not provided.
//! Useful in testing situations.
//!
//! ## Used by
//!
//! -   [Lockbook](https://github.com/lockbook/lockbook)
//!

pub use crate::compacter::BackgroundCompacter;
pub use crate::compacter::CancelSig;
pub use crate::config::Config;
pub use crate::db::Db;
pub use crate::errors::DbError;
pub use crate::errors::DbResult;
pub use crate::logger::Logger;
pub use crate::logger::TxHandle;

pub use crate::list::List;
pub use crate::lookup::LookupTable;
pub use crate::lookup_list::LookupList;
pub use crate::lookup_set::LookupSet;
pub use crate::single::Single;

pub mod compacter;
pub mod config;
pub mod db;
pub mod errors;
pub mod list;
pub mod logger;
pub mod lookup;
pub mod lookup_list;
pub mod lookup_set;
pub mod single;
pub mod table;

pub type TableId = u8;
pub type ByteCount = u32;
