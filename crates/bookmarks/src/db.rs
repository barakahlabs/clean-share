//! Opening a database. This is the only place that knows *where* the data lives.
//!
//! Today: in memory (tests) or a local file. Later a `sync` module opens the user's Turso Cloud
//! database the same way and returns the same [`Connection`] type, so [`crate::Store`] works on
//! either without changes.

use std::path::PathBuf;

use turso::{Builder, Connection};

use crate::Result;

#[derive(Debug, Clone)]
pub enum Location {
  Memory,
  File(PathBuf),
}

/// Opens a local database and returns a connection to it.
pub async fn open(location: &Location) -> Result<Connection> {
  let path = match location {
    Location::Memory => ":memory:".to_string(),
    Location::File(path) => path.to_string_lossy().into_owned(),
  };
  let db = Builder::new_local(&path)
    // Full-text search (`CREATE INDEX … USING fts`) is still behind this flag in Turso.
    // A synced database must enable it too.
    .experimental_index_method(true)
    .build()
    .await?;
  Ok(db.connect()?)
}
