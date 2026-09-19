use std::{
    fs::File,
    ops::{Deref, DerefMut},
};

use crate::{View, errors::Result};

pub struct ReadTx<'a, V: ?Sized> {
    pub(crate) view: &'a V,
    pub(crate) lock: File,
    pub stale: bool,
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

pub struct WriteTx<'a, V: View> {
    pub(crate) view: &'a mut V,
    pub(crate) lock: File,
    pub(crate) finalized: bool,
}

impl<V: View> WriteTx<'_, V> {
    pub fn end_tx(mut self) -> Result<()> {
        let flush_result = self.view.flush_pending();
        let unlock_result = self.lock.unlock().map_err(Into::into);
        self.finalized = true;
        flush_result.and(unlock_result)
    }
}

impl<V: View> Deref for WriteTx<'_, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.view
    }
}

impl<V: View> DerefMut for WriteTx<'_, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.view
    }
}

impl<V: View> Drop for WriteTx<'_, V> {
    fn drop(&mut self) {
        if !self.finalized {
            let _ = self.view.flush_pending();
            let _ = self.lock.unlock();
        }
    }
}
