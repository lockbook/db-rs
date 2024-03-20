use std::sync::{Arc, Mutex};

use crate::Db;

pub trait CooperativeDb {
    fn start(&self);
}

impl<D> CooperativeDb for Arc<Mutex<D>>
where
    D: Db + Send + Sync + 'static,
{
    fn start(&self) {}
}
