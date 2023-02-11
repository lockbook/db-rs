use std::io;
use std::sync::PoisonError;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug)]
pub enum DbError {
    Unexpected(&'static str),
    Io(io::Error),
    Bincode(bincode::Error),
    MutexPoisoned,
}

impl From<bincode::Error> for DbError {
    fn from(err: bincode::Error) -> Self {
        Self::Bincode(err)
    }
}

impl From<io::Error> for DbError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl<G> From<PoisonError<G>> for DbError {
    fn from(_: PoisonError<G>) -> Self {
        Self::MutexPoisoned
    }
}
