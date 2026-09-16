use crate::{
    View,
    config::Config,
    errors::{Error, Result},
    log::{Log, head_entry},
};

pub struct Db<V: View> {
    view: V,
    log: Log,
}

impl<V: View> Db<V> {
    pub fn init(config: Config) -> Result<Self> {
        let mut log = Log::init(&config)?;
        let mut view = V::default();
        let bytes = log.get_bytes()?;
        let mut remaining = bytes.as_slice();

        while !remaining.is_empty() {
            let (Some(entry), rest) = head_entry(remaining) else {
                return Err(Error::IncompleteLog {
                    remaining_bytes: remaining.len(),
                });
            };

            view.handle_events(entry.payload)?;
            remaining = rest;
        }

        Ok(Self {
            view,
            log,
        })
    }

    pub fn begin_tx(&mut self) -> &mut V {
        &mut self.view
    }

    pub fn end_tx(&mut self) {}
}
