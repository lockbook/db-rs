pub mod config;
pub mod errors;
pub mod log;
pub mod payload_buffer;
pub mod views;

use config::Config;
use errors::{Error, Result};
use log::{Log, LogEntry, head_entry};

pub trait View: Default {
    fn log(&self) -> &Log;
    fn log_mut(&mut self) -> &mut Log;
    fn set_log(&mut self, log: Log);

    fn handle_events(&mut self, events: &[u8]) -> Result<()>;
    fn take_events(&mut self) -> Vec<u8>;
    fn snapshot_bytes(&self) -> Result<Vec<u8>>;

    /// Restores a view with its active log.
    fn init(config: &Config) -> Result<Self> {
        let mut view = Self::default();
        let mut seq_no = 0;

        'candidate: loop {
            let mut log = Log::init(config)?;
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

            log.seq_no = seq_no;
            view.set_log(log);
            return Ok(view);
        }
    }

    fn read_tx(&self) -> Result<&Self> {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        Ok(self)
    }

    fn write_tx(&mut self) -> Result<&mut Self> {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        Ok(self)
    }

    fn end_tx(&mut self) -> Result<()> {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        // Once events are drained, any failure requires reopening the database.
        self.log_mut().poisoned = true;
        let events = self.take_events();
        let log = self.log_mut();
        if !events.is_empty() {
            let seq_no = log.seq_no.checked_add(1).ok_or(Error::SequenceExhausted)?;
            log.append(LogEntry::Events {
                seq_no,
                payload: &events,
            })?;
            log.seq_no = seq_no;
        }
        log.poisoned = false;
        Ok(())
    }

    fn snapshot(&mut self) -> Result<()> {
        self.end_tx()?;
        let payload = self.snapshot_bytes()?;

        let log = self.log_mut();
        log.poisoned = true;
        let Some(new_log) = log.create_snapshot(log.seq_no, &payload)? else {
            log.poisoned = false;
            return Ok(());
        };
        log.append(LogEntry::Snapshot)?;
        self.set_log(new_log);

        Ok(())
    }
}
