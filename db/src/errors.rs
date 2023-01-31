use std::io;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug)]
pub enum DbError {
    // Unexpected Errors
    MisConfig(&'static str),
    Io(io::Error),
    Bincode(bincode::Error),
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
