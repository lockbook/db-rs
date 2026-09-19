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
    payload: &'a [u8],
}

#[derive(Default)]
pub struct Composite<S: Schema> {
    pub schema: S,
    log: Option<Log>,
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

    fn handle_events(&mut self, mut events: &[u8]) -> Result<()> {
        let mut views = self.schema.views_mut();
        while let Some(bytes) = PayloadBuffer::head_payload(&mut events)? {
            let event: Event<'_> = bin_decode(bytes)?;
            let view = views
                .as_mut()
                .get_mut(event.table_id)
                .ok_or(Error::UnknownTable {
                    table_id: event.table_id,
                })?;
            view.handle_events(event.payload)?;
        }

        Ok(())
    }

    fn take_pending(&mut self) -> Vec<u8> {
        let mut pending = PayloadBuffer::default();
        for (table_id, view) in self.schema.views_mut().as_mut().iter_mut().enumerate() {
            let payload = view.take_pending();
            if payload.is_empty() {
                continue;
            }

            pending
                .push_encoded(&Event {
                    table_id,
                    payload: &payload,
                })
                .expect("encoding a table ID and byte slice cannot fail");
        }

        pending.bytes
    }

    fn generate_snapshot(&mut self) -> Result<Vec<u8>> {
        let mut snapshot = PayloadBuffer::default();
        for (table_id, view) in self.schema.views_mut().as_mut().iter_mut().enumerate() {
            let payload = view.generate_snapshot()?;
            snapshot.push_encoded(&Event {
                table_id,
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

        let events = source.take_pending();
        assert!(source.take_pending().is_empty());

        let mut db = Composite::<TestSchema>::default();
        db.handle_events(&events).unwrap();

        assert_eq!(
            db.schema.users.get("alice").map(String::as_str),
            Some("Alice")
        );
        assert_eq!(
            db.schema.settings.as_ref().map(String::as_str),
            Some("dark")
        );
        assert!(db.take_pending().is_empty());
    }

    #[test]
    fn take_pending_skips_unchanged_views() {
        let mut db = Composite::<TestSchema>::default();
        assert!(db.take_pending().is_empty());
        db.schema.settings.replace("dark".into()).unwrap();

        let events = db.take_pending();
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
        assert!(db.take_pending().is_empty());
    }

    #[test]
    fn snapshot_round_trip() {
        let config = Config::test();
        {
            let mut db = Composite::<TestSchema>::init(&config).unwrap();
            {
                let mut tx = db.write_tx().unwrap();
                tx.schema
                    .users
                    .insert("alice".into(), "Alice".into())
                    .unwrap();
                tx.schema.settings.replace("dark".into()).unwrap();
                tx.end_tx().unwrap();
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
                    payload: b"",
                })
                .unwrap(),
            );

            let mut db = Composite::<TestSchema>::default();
            assert!(matches!(
                db.handle_events(&events.bytes),
                Err(Error::UnknownTable { table_id: found }) if found == table_id
            ));
        }
    }
}
