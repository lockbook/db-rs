use std::error::Error;
use std::fmt::Display;
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

impl Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Unexpected(u) => write!(f, "unexpected error: {u}"),
            DbError::Io(i) => write!(f, "io error: {i}"),
            DbError::Bincode(b) => write!(f, "bincode error: {b}"),
            DbError::MutexPoisoned => write!(f, "mutex poisoned"),
        }
    }
}

impl Error for DbError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DbError::Io(e) => Some(e),
            DbError::Bincode(e) => Some(e),
            DbError::MutexPoisoned => None,
            DbError::Unexpected(_) => None,
        }
    }
}
