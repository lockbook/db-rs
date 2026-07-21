use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Config {
    pub(crate) io: Option<IoConfig>,
}

pub struct IoConfig {
    pub(crate) data_dir: PathBuf,
    pub(crate) instance_name: Option<String>,
    pub(crate) read_only: bool,
    pub(crate) ipc_profile: IpcProfile,
}

impl IoConfig {
    pub(crate) fn log_path(&self) -> PathBuf {
        self.data_dir.join(format!(
            "{}.db",
            self.instance_name.clone().unwrap_or_else(|| "db-rs".into())
        ))
    }

    pub(crate) fn lock_path(&self) -> PathBuf {
        self.data_dir.join(format!(
            "{}.lock",
            self.instance_name.clone().unwrap_or_else(|| "db-rs".into())
        ))
    }
}

#[derive(PartialEq, Eq)]
pub enum IpcProfile {
    /// Single Process Optimizations on, supports multiple shards
    Server,

    /// Multiple, co-operative-processes, requires flocks, precludes multiple shards
    Client,
}

impl Default for Config {
    fn default() -> Self {
        Self::base().test_location()
    }
}

impl Config {
    /// Specifies the default config, one that's suitable for test-clients
    pub(crate) fn base() -> Self {
        Config { io: None }
    }

    /// Points the data dir at the OS temp dir: %TEMP% on Windows, $TMPDIR or /tmp elsewhere
    pub fn test_location(mut self) -> Self {
        let data_dir = env::temp_dir().join("db-rs");
        self.io = Some(match self.io {
            Some(io) => IoConfig { data_dir, ..io },
            None => IoConfig {
                data_dir,
                instance_name: None,
                read_only: false,
                ipc_profile: IpcProfile::Server,
            },
        });
        self
    }

    pub fn instance_name(mut self, name: String) -> Self {
        self.io
            .as_mut()
            .expect("Must specify location before name")
            .instance_name = Some(name);
        self
    }

    pub fn test_name(self) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX epoch")
            .as_nanos();
        self.instance_name(format!("{nanos}"))
    }
}
