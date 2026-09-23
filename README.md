# db-rs

## In-memory and WebAssembly

Use `Config::in_memory()` on `wasm32-unknown-unknown`, or for an in-memory
database on native platforms:

```rust
use db_rs::{View, config::Config, views::hashmap::DbHashMap};

let mut db = DbHashMap::<String, u64>::init(&Config::in_memory()).unwrap();
let tx = db.write_tx().unwrap();
db.insert("key".into(), 42).unwrap();
tx.end_tx(&mut db).unwrap();
assert_eq!(db.read_tx().unwrap().get("key"), Some(&42));
```

Transactions, sequence numbers, composite views, and local notifications work
without filesystem access. Each initialization creates an independent database;
data is lost when the view is dropped. There is no persistence or IPC in this
mode. `snapshot()` does not create a file; `generate_snapshot()` still produces
serialized view data.

`Config::default()` remains file-backed. It does not silently fall back to
in-memory storage on unsupported platforms. Browser storage such as IndexedDB
is not implemented.

### Running the WASM tests

With Node.js installed:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.108 --locked
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
    cargo test --target wasm32-unknown-unknown --test in_memory
```

The same tests run natively with `cargo test --test in_memory`.
