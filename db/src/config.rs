use crate::errors::{DbError, DbResult};
use std::path::{Path, PathBuf};

pub struct Config {
    pub path: PathBuf,
    pub schema_name: Option<String>,
    pub create_path: bool,
    pub create_db: bool,
    pub read_only: bool,
    pub no_io: bool,
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
