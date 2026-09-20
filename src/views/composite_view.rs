use serde::{Deserialize, Serialize};

use crate::{
    View,
    errors::{Error, Result},
    log::Log,
    payload_buffer::PayloadBuffer,
};

use super::bin_decode;

/// Slice positions are table IDs and must remain stable.
pub trait Schema: Default {
    fn views_mut(&mut self) -> impl AsMut<[&mut dyn View]>;
}

#[derive(Serialize, Deserialize)]
struct Event<'a> {
    table_id: usize,
    last_modified: Option<u64>,
    payload: &'a [u8],
}

#[derive(Default)]
pub struct Composite<S: Schema> {
    pub schema: S,
    log: Option<Log>,
    last_modified: u64,
}

impl<S: Schema> View for Composite<S> {
    fn log(&self) -> &Log {
        self.log.as_ref().expect("view has no initialized log")
    }

    fn log_mut(&mut self) -> &mut Log {
        self.log.as_mut().expect("view has no initialized log")
    }

    fn set_log(&mut self, log: Log) {
        self.log = Some(log);
    }

    fn last_modified(&self) -> u64 {
        self.last_modified
    }

    fn handle_events(&mut self, seq_no: u64, mut events: &[u8]) -> Result<()> {
        let mut views = self.schema.views_mut();
        while let Some(bytes) = PayloadBuffer::head_payload(&mut events)? {
            let event: Event<'_> = bin_decode(bytes)?;
            let view = views
                .as_mut()
                .get_mut(event.table_id)
                .ok_or(Error::UnknownTable {
                    table_id: event.table_id,
                })?;
            view.handle_events(event.last_modified.unwrap_or(seq_no), event.payload)?;
        }

        self.last_modified = seq_no;
        Ok(())
    }

    fn take_pending(&mut self, seq_no: u64) -> Vec<u8> {
        let mut pending = PayloadBuffer::default();
        for (table_id, view) in self.schema.views_mut().as_mut().iter_mut().enumerate() {
            let payload = view.take_pending(seq_no);
            if payload.is_empty() {
                continue;
            }

            pending
                .push_encoded(&Event {
                    table_id,
                    last_modified: None,
                    payload: &payload,
                })
                .expect("encoding a table ID and byte slice cannot fail");
        }

        if !pending.bytes.is_empty() {
            self.last_modified = seq_no;
        }
        pending.bytes
    }

    fn generate_snapshot(&mut self) -> Result<Vec<u8>> {
        let mut snapshot = PayloadBuffer::default();
        for (table_id, view) in self.schema.views_mut().as_mut().iter_mut().enumerate() {
            let payload = view.generate_snapshot()?;
            snapshot.push_encoded(&Event {
                table_id,
                last_modified: Some(view.last_modified()),
                payload: &payload,
            })?;
        }

        Ok(snapshot.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        views::{bin_encode, hashmap::DbHashMap, option::DbOption},
    };

    #[derive(Default)]
    struct TestSchema {
        users: DbHashMap<String, String>,
        settings: DbOption<String>,
    }

    impl Schema for TestSchema {
        fn views_mut(&mut self) -> impl AsMut<[&mut dyn View]> {
            let views: [&mut dyn View; 2] = [&mut self.users, &mut self.settings];
            views
        }
    }

    #[test]
    fn routes_events_to_child_views() {
        let mut source = Composite::<TestSchema>::default();
        source
            .schema
            .users
            .insert("alice".into(), "Alice".into())
            .unwrap();
        source.schema.settings.replace("dark".into()).unwrap();

        let events = source.take_pending(1);
        assert_eq!(source.last_modified(), 1);
        assert_eq!(source.schema.users.last_modified(), 1);
        assert_eq!(source.schema.settings.last_modified(), 1);
        assert!(source.take_pending(2).is_empty());
        assert_eq!(source.last_modified(), 1);

        let mut db = Composite::<TestSchema>::default();
        db.handle_events(1, &events).unwrap();

        assert_eq!(
            db.schema.users.get("alice").map(String::as_str),
            Some("Alice")
        );
        assert_eq!(
            db.schema.settings.as_ref().map(String::as_str),
            Some("dark")
        );
        assert!(db.take_pending(1).is_empty());
    }

    #[test]
    fn take_pending_skips_unchanged_views() {
        let mut db = Composite::<TestSchema>::default();
        assert!(db.take_pending(1).is_empty());
        db.schema.settings.replace("dark".into()).unwrap();

        let events = db.take_pending(1);
        assert_eq!(db.last_modified(), 1);
        assert_eq!(db.schema.users.last_modified(), 0);
        assert_eq!(db.schema.settings.last_modified(), 1);
        let mut remaining = events.as_slice();
        let bytes = PayloadBuffer::head_payload(&mut remaining)
            .unwrap()
            .unwrap();
        let event: Event<'_> = bin_decode(bytes).unwrap();

        assert_eq!(event.table_id, 1);
        assert!(
            PayloadBuffer::head_payload(&mut remaining)
                .unwrap()
                .is_none()
        );
        assert!(db.take_pending(1).is_empty());
    }

    #[test]
    fn snapshot_round_trip() {
        let config = Config::test();
        {
            let mut db = Composite::<TestSchema>::init(&config).unwrap();
            {
                let tx = db.write_tx().unwrap();
                db.schema
                    .users
                    .insert("alice".into(), "Alice".into())
                    .unwrap();
                db.schema.settings.replace("dark".into()).unwrap();
                tx.end_tx(&mut db).unwrap();
            }
            db.snapshot().unwrap();
        }

        let db = Composite::<TestSchema>::init(&config).unwrap();
        assert_eq!(
            db.schema.users.get("alice").map(String::as_str),
            Some("Alice")
        );
        assert_eq!(
            db.schema.settings.as_ref().map(String::as_str),
            Some("dark")
        );
    }

    #[test]
    fn rejects_unknown_table_ids() {
        for table_id in [2, usize::MAX] {
            let mut events = PayloadBuffer::default();
            events.push(
                &bin_encode(&Event {
                    table_id,
                    last_modified: None,
                    payload: b"",
                })
                .unwrap(),
            );

            let mut db = Composite::<TestSchema>::default();
            assert!(matches!(
                db.handle_events(1, &events.bytes),
                Err(Error::UnknownTable { table_id: found }) if found == table_id
            ));
        }
    }
}
