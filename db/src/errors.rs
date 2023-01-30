use std::io;

pub enum DbError {
    // Unexpected Errors
    MisConfig(&'static str),
    Io(io::Error),
}

impl From<io::Error> for DbError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
