use std::{
    fs,
    io::ErrorKind,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
pub struct Config {
    pub log_location: PathBuf,
}

impl Config {
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
                Ok(()) => return Self { log_location },
                Err(error) if error.kind() == ErrorKind::AlreadyExists => id += 1,
                Err(error) => panic!("failed to create test directory: {error}"),
            }
        }
    }
}
