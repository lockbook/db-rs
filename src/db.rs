use crate::{
    View,
    config::Config,
    errors::{Error, Result},
    log::{Log, LogEntry, head_entry},
};

pub struct Db<V: View> {
    view: V,
    log: Log,
    seq_no: u64,
    poisoned: bool,
}

impl<V: View> Db<V> {
    pub fn init(config: Config) -> Result<Self> {
        let mut log = Log::init(&config)?;
        let mut view = V::default();
        let bytes = log.get_bytes()?;
        let mut remaining = bytes.as_slice();
        let mut seq_no = 0;

        while let Some(entry) = head_entry(&mut remaining)? {
            view.handle_events(entry.payload)?;
            seq_no = entry.seq_no;
        }

        Ok(Self {
            view,
            log,
            seq_no,
            poisoned: false,
        })
    }

    pub fn begin_tx(&mut self) -> Result<&mut V> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        Ok(&mut self.view)
    }

    pub fn end_tx(&mut self) -> Result<()> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        // Once events are drained, any failure requires reopening the database.
        self.poisoned = true;
        let events = self.view.take_events();
        if !events.is_empty() {
            let seq_no = self.seq_no.checked_add(1).ok_or(Error::SequenceExhausted)?;
            self.log.append(LogEntry {
                seq_no,
                payload: &events,
            })?;
            self.seq_no = seq_no;
        }
        self.poisoned = false;
        Ok(())
    }
}
