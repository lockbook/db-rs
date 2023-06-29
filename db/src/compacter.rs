use crate::{Db, DbResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Default, Clone)]
pub struct CancelSig(Arc<AtomicBool>);

impl CancelSig {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_canceled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub trait BackgroundCompacter {
    /// Periodically compact the database log in a separate thread
    /// You can call this function if your db is wrapped in an `Arc<Mutex>`
    ///
    /// freq determines how often the background thread will aquire a mutex
    /// and call compact_log() on your db
    ///
    /// cancel is an AtomicBool which can be passed in and signal that compaction
    /// should cease (could take up-to freq to return)
    ///
    /// this fn returns the number of times compaction took place
    fn begin_compacter(&self, freq: Duration, cancel: CancelSig) -> JoinHandle<DbResult<usize>>;
}

impl<D> BackgroundCompacter for Arc<Mutex<D>>
where
    D: Db + Send + Sync + 'static,
{
    fn begin_compacter(&self, freq: Duration, cancel: CancelSig) -> JoinHandle<DbResult<usize>> {
        let db = self.clone();
        thread::spawn(move || {
            let mut count = 0;
            loop {
                thread::sleep(freq);

                if cancel.is_canceled() {
                    return Ok(count);
                }

                db.lock()?.compact_log()?;
                count += 1;
            }
        })
    }
}
