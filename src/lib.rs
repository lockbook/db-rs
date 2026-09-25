pub mod config;
pub mod errors;
pub mod guard;
pub mod log;
pub mod payload_buffer;
pub mod views;

use config::Config;
use errors::{Error, Result};
use guard::{Lock, ReadTx, WriteTx};
use log::{Log, LogEntry, head_entry};

pub trait View {
    fn log(&self) -> &Log;
    fn log_mut(&mut self) -> &mut Log;
    fn set_log(&mut self, log: Log);

    fn last_modified(&self) -> u64;

    fn handle_events(&mut self, seq_no: u64, events: &[u8]) -> Result<()>;
    fn take_pending(&mut self, seq_no: u64) -> Vec<u8>;
    fn generate_snapshot(&mut self) -> Result<Vec<u8>>;

    /// Restores a persistent view, or creates an empty in-memory view.
    fn init(config: &Config) -> Result<Self>
    where
        Self: Default,
    {
        let mut view = Self::default();
        let log = Log::init(config)?;
        let lock = log.write_lock()?;
        view.set_log(log);
        let lock = view.catch_up(lock)?;
        lock.unlock()?;
        Ok(view)
    }

    fn read_tx(&self) -> Result<ReadTx<'_, Self>>
    where
        Self: Sized,
    {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        let lock = self.log().read_lock()?;
        Ok(ReadTx { view: self, lock })
    }

    fn write_tx(&mut self) -> Result<WriteTx>
    where
        Self: Sized,
    {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        // Pending edits from an unfinished transaction cannot be carried into a new one.
        self.log_mut().poisoned = true;
        let seq_no = self.log().seq_no;
        if !self.take_pending(seq_no).is_empty() {
            return Err(Error::Poisoned);
        }
        self.log_mut().poisoned = false;

        let lock = self.log().write_lock()?;
        let lock = match self.catch_up(lock) {
            Ok(lock) => lock,
            Err(error) => {
                self.log_mut().poisoned = true;
                return Err(error);
            }
        };
        let seq_no = self
            .log()
            .seq_no
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        Ok(WriteTx { seq_no, lock })
    }

    fn catch_up(&mut self, lock: Lock) -> Result<Lock> {
        let initial_seq_no = self.log().seq_no;
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
                        self.handle_events(entry_seq_no, payload)?;
                        self.log_mut().seq_no = entry_seq_no;
                    }
                    LogEntry::Snapshot => {
                        snapshot = true;
                        break;
                    }
                }
            }

            if !snapshot {
                if self.log().seq_no != initial_seq_no {
                    self.log().notify(false);
                }
                return Ok(lock);
            }

            // The database-wide lock stays held while switching logs.
            self.log_mut().poisoned = true;
            let path = match self.log().find_next()? {
                Some(path) => path,
                None => {
                    let seq_no = self.log().seq_no;
                    // Recovery must not publish uncommitted edits from an abandoned transaction.
                    if !self.take_pending(seq_no).is_empty() {
                        return Err(Error::Poisoned);
                    }
                    let payload = self.generate_snapshot()?;
                    let path = self
                        .log()
                        .prepare_snapshot(seq_no, &payload)?
                        .ok_or(Error::MissingSnapshotLog)?;
                    self.log().publish_snapshot(&path)?;
                    path
                }
            };
            self.log_mut().switch_to(path)?;
            self.log_mut().poisoned = false;
        }
    }

    fn flush_pending(&mut self, seq_no: u64) -> Result<()> {
        if self.log().poisoned {
            return Err(Error::Poisoned);
        }
        // Once events are drained, any failure requires reopening the database.
        self.log_mut().poisoned = true;
        let events = self.take_pending(seq_no);
        let log = self.log_mut();
        if !events.is_empty() {
            log.append(LogEntry::Events {
                seq_no,
                payload: &events,
            })?;
            log.seq_no = seq_no;
            log.poisoned = false;
            log.notify(true);
        }
        self.log_mut().poisoned = false;
        Ok(())
    }

    fn snapshot(&mut self) -> Result<()>
    where
        Self: Sized,
    {
        let tx = self.write_tx()?;
        self.flush_pending(tx.seq_no)?;
        let payload = self.generate_snapshot()?;

        let log = self.log_mut();
        log.poisoned = true;
        let Some(path) = log.prepare_snapshot(log.seq_no, &payload)? else {
            log.poisoned = false;
            return tx.end_tx(self).map(|_| ());
        };
        // Stop writes to the old log before publishing the new snapshot.
        log.append(LogEntry::Snapshot)?;
        log.publish_snapshot(&path)?;
        log.switch_to(path)?;
        log.poisoned = false;

        tx.end_tx(self).map(|_| ())
    }
}
