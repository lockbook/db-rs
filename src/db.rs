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
        let mut view = V::default();
        let mut seq_no = 0;

        'candidate: loop {
            let mut log = Log::init(&config)?;
            let bytes = log.get_bytes()?;
            let mut remaining = bytes.as_slice();

            while let Some(entry) = head_entry(&mut remaining)? {
                match entry {
                    LogEntry::Events {
                        seq_no: entry_seq_no,
                        payload,
                    } => {
                        if entry_seq_no == seq_no {
                            continue;
                        }
                        if entry_seq_no < seq_no {
                            return Err(Error::OutOfOrderSequence {
                                current: seq_no,
                                found: entry_seq_no,
                            });
                        }
                        view.handle_events(payload)?;
                        seq_no = entry_seq_no;
                    }
                    LogEntry::Snapshot => continue 'candidate,
                }
            }

            return Ok(Self {
                view,
                log,
                seq_no,
                poisoned: false,
            });
        }
    }

    pub fn read_tx(&self) -> Result<&V> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        Ok(&self.view)
    }

    pub fn write_tx(&mut self) -> Result<&mut V> {
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
            self.log.append(LogEntry::Events {
                seq_no,
                payload: &events,
            })?;
            self.seq_no = seq_no;
        }
        self.poisoned = false;
        Ok(())
    }

    pub fn snapshot(&mut self) -> Result<()> {
        self.end_tx()?;
        let payload = self.view.snapshot()?;

        self.poisoned = true;
        let Some(new_log) = self.log.create_snapshot(self.seq_no, &payload)? else {
            self.poisoned = false;
            return Ok(());
        };
        self.log.append(LogEntry::Snapshot)?;
        self.log = new_log;
        self.poisoned = false;

        Ok(())
    }
}
