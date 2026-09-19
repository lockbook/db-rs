pub mod config;
pub mod errors;
pub mod guard;
pub mod log;
pub mod payload_buffer;
pub mod views;

use std::fs::File;

use config::Config;
use errors::{Error, Result};
use guard::{ReadTx, WriteTx};
use log::{Log, LogEntry, head_entry};

pub trait View: Default {
    fn log(&self) -> &Log;
    fn log_mut(&mut self) -> &mut Log;
    fn set_log(&mut self, log: Log);

    fn handle_events(&mut self, events: &[u8]) -> Result<()>;
    fn take_pending(&mut self) -> Vec<u8>;
    fn snapshot_bytes(&self) -> Result<Vec<u8>>;

    /// Restores a view with its active log.
    fn init(config: &Config) -> Result<Self> {
        let mut view = Self::default();
        let log = Log::init(config)?;
        let lock = log.write_lock()?;
        view.set_log(log);
        let lock = view.catch_up(lock)?;
        lock.unlock()?;
        Ok(view)
    }

    fn read_tx(&self) -> Result<ReadTx<'_, Self>> {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        let mut lock = self.log().read_lock()?;
        let stale = self.log().is_stale(&mut lock)?;
        Ok(ReadTx {
            view: self,
            lock,
            stale,
        })
    }

    fn write_tx(&mut self) -> Result<WriteTx<'_, Self>> {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        let lock = self.log().write_lock()?;
        let lock = match self.catch_up(lock) {
            Ok(lock) => lock,
            Err(error) => {
                self.log_mut().poisoned = true;
                return Err(error);
            }
        };
        Ok(WriteTx {
            view: self,
            lock,
            finalized: false,
        })
    }

    fn catch_up(&mut self, mut lock: File) -> Result<File> {
        loop {
            let bytes = self.log_mut().get_bytes()?;
            let mut remaining = bytes.as_slice();
            let mut snapshot = false;

            while let Some(entry) = head_entry(&mut remaining)? {
                match entry {
                    LogEntry::Events {
                        seq_no: entry_seq_no,
                        payload,
                    } => {
                        let seq_no = self.log().seq_no;
                        if entry_seq_no == seq_no {
                            continue;
                        }
                        if entry_seq_no < seq_no {
                            return Err(Error::OutOfOrderSequence {
                                current: seq_no,
                                found: entry_seq_no,
                            });
                        }
                        self.handle_events(payload)?;
                        self.log_mut().seq_no = entry_seq_no;
                    }
                    LogEntry::Snapshot => {
                        snapshot = true;
                        break;
                    }
                }
            }

            if !snapshot {
                return Ok(lock);
            }

            let Some(new_log) = self.log().find_next()? else {
                return Err(Error::MissingSnapshotLog);
            };
            let new_lock = new_log.write_lock()?;
            lock.unlock()?;
            self.set_log(new_log);
            lock = new_lock;
        }
    }

    fn flush_pending(&mut self) -> Result<()> {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        // Once events are drained, any failure requires reopening the database.
        self.log_mut().poisoned = true;
        let events = self.take_pending();
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
        let mut tx = self.write_tx()?;
        tx.flush_pending()?;
        let payload = tx.snapshot_bytes()?;

        let log = tx.log_mut();
        log.poisoned = true;
        let Some(new_log) = log.create_snapshot(log.seq_no, &payload)? else {
            log.poisoned = false;
            return tx.end_tx();
        };
        log.append(LogEntry::Snapshot)?;
        tx.set_log(new_log);

        tx.end_tx()
    }
}
