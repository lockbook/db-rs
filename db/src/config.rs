use crate::errors::{DbError, DbResult};
use std::path::{Path, PathBuf};

/// db-rs's config that describes where the log file should be and how the database should behave.
/// use [Config::in_folder] as a starting point.
#[derive(Clone, Debug)]
pub struct Config {
    /// folder where db-rs can write it's log
    pub path: PathBuf,

    /// should db-rs create parent folders that don't exist? Default: true
    pub create_path: bool,

    /// should db-rs create a log if one doesn't exist? Default: true
    pub create_db: bool,

    /// should db-rs only read and not write? (good for analysis tooling) Default: false
    pub read_only: bool,

    /// should db-rs avoid all IO? (good for tests) Default: false
    pub no_io: bool,

    #[doc(hidden)]
    pub schema_name: Option<String>,
}

impl Config {
    fn base() -> Self {
        Self {
            path: Default::default(),
            schema_name: None,
            create_path: true,
            create_db: true,
            read_only: false,
            no_io: false,
        }
    }

    pub fn no_io() -> Self {
        Self {
            path: Default::default(),
            schema_name: None,
            create_path: false,
            create_db: false,
            read_only: true,
            no_io: true,
        }
    }

    pub fn in_folder<P>(p: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self { path: PathBuf::from(p.as_ref()), ..Self::base() }
    }

    pub fn db_location(&self) -> DbResult<PathBuf> {
        let name = self.schema_name.as_ref().ok_or(DbError::Unexpected(
            "Schema name not populated! db-rs-derive should have done this",
        ))?;
        let mut pathbuf = self.path.clone();
        pathbuf.push(name);
        Ok(pathbuf)
    }

    pub fn compaction_location(&self) -> DbResult<PathBuf> {
        let name = self.schema_name.as_ref().ok_or(DbError::Unexpected(
            "Schema name not populated! db-rs-derive should have done this",
        ))?;
        let mut pathbuf = self.path.clone();
        pathbuf.push(format!("{name}.tmp"));
        Ok(pathbuf)
    }
}
