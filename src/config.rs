use std::{
    fs,
    io::ErrorKind,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
pub struct Config {
    pub log_location: PathBuf,
    /// Keeps data only in the view, without filesystem access or IPC; ignores `log_location`.
    pub in_memory: bool,
}

impl Config {
    /// Creates an isolated, non-persistent database, including on WebAssembly.
    /// Dropping the view discards its data; initializing again starts empty.
    pub fn in_memory() -> Self {
        Self {
            in_memory: true,
            ..Self::default()
        }
    }

    pub fn log_location(mut self, location: impl Into<PathBuf>) -> Self {
        self.log_location = location.into();
        self
    }

    pub fn test() -> Self {
        let mut id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        loop {
            let log_location = std::env::temp_dir().join(format!("db-rs-{id}"));
            match fs::create_dir(&log_location) {
                Ok(()) => {
                    return Self {
                        log_location,
                        ..Self::default()
                    };
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => id += 1,
                Err(error) => panic!("failed to create test directory: {error}"),
            }
        }
    }
}
