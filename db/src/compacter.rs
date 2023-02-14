use crate::{Db, DbResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

type CancelSig = Arc<AtomicBool>;

pub trait BackgroundCompacter {
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

                if cancel.load(Ordering::Relaxed) {
                    return Ok(count);
                }

                db.lock()?.compact_log()?;
                count += 1;
            }
        })
    }
}
