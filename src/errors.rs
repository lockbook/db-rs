use std::error::Error;
use std::fmt::Display;
use std::io;
use std::sync::PoisonError;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug)]
pub enum DbError {
    Unexpected(&'static str),
    Io(io::Error),
    MutexPoisoned,

    /// The database started up successfully, but contains an incomplete transaction
    ///
    /// For software deployed to end users, likely this is safe to ignore. You can see
    /// what portion of the entire log was found to be unreadable if you want to handle
    /// this error with nuance.
    ///
    /// For software deployed to servers under your control (where you can author graceful
    /// shutdowns), you should likely treat this as an error.
    IncompleteLog {
        total_log: usize,
        incomplete_size: usize,
    },
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
            DbError::MutexPoisoned => write!(f, "mutex poisoned"),
            DbError::IncompleteLog { total_log, incomplete_size } => todo!(),
        }
    }
}

impl Error for DbError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            DbError::Io(e) => Some(e),
            DbError::MutexPoisoned => None,
            DbError::Unexpected(_) => None,
            DbError::IncompleteLog { total_log, incomplete_size } => None,
        }
    }
}
