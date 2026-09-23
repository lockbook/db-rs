use std::{fs::File, io, ops::Deref};

use crate::{View, errors::Result};

pub struct Lock(pub(crate) Option<File>);

impl Lock {
    pub fn unlock(&self) -> io::Result<()> {
        if let Some(file) = &self.0 {
            file.unlock()?;
        }
        Ok(())
    }
}

pub struct ReadTx<'a, V: ?Sized> {
    pub(crate) view: &'a V,
    pub(crate) lock: Lock,
}

impl<V: ?Sized> Deref for ReadTx<'_, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.view
    }
}

impl<V: ?Sized> Drop for ReadTx<'_, V> {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}

/// Holds the write lock until explicitly ended or dropped.
/// Dropping this guard does not flush or roll back pending changes.
#[must_use = "call end_tx(&mut view) to flush pending changes before releasing the lock"]
pub struct WriteTx {
    pub seq_no: u64,
    pub(crate) lock: Lock,
}

impl WriteTx {
    /// Flush the view that started this transaction, then release the lock.
    pub fn end_tx(self, view: &mut impl View) -> Result<u64> {
        let flush_result = view.flush_pending(self.seq_no);
        let unlock_result = self.lock.unlock().map_err(Into::into);
        flush_result.and(unlock_result)?;
        Ok(view.log().seq_no)
    }
}
