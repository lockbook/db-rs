use std::path::PathBuf;

#[derive(Default)]
pub struct Config {
    pub log_location: PathBuf,
}

impl Config {
    pub fn log_location(mut self, location: impl Into<PathBuf>) -> Self {
        self.log_location = location.into();
        self
    }
}
