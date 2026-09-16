use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Encode(bincode::error::EncodeError),
    Decode(bincode::error::DecodeError),
    IncompleteLog { remaining_bytes: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "log I/O error: {error}"),
            Self::Encode(error) => write!(formatter, "event encode error: {error}"),
            Self::Decode(error) => write!(formatter, "event decode error: {error}"),
            Self::IncompleteLog { remaining_bytes } => {
                write!(formatter, "incomplete log entry with {remaining_bytes} trailing bytes")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            Self::IncompleteLog { .. } => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<bincode::error::DecodeError> for Error {
    fn from(error: bincode::error::DecodeError) -> Self {
        Self::Decode(error)
    }
}

impl From<bincode::error::EncodeError> for Error {
    fn from(error: bincode::error::EncodeError) -> Self {
        Self::Encode(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
