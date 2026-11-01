pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("the shared text has no link")]
  NoLink,
  #[error("bookmark not found")]
  NotFound,
  #[error("tag name is empty")]
  EmptyTag,
  #[error("tag {0:?} not found")]
  TagNotFound(String),
  #[error("invalid cursor")]
  InvalidCursor,
  /// The database was written by a newer app version (e.g. synced from another device).
  #[error("database schema {found} is newer than this app supports ({supported}); update the app")]
  NewerSchema { found: i64, supported: i64 },
  #[error("unexpected value in column {column}")]
  BadColumn { column: &'static str },
  #[error("database: {0}")]
  Db(#[from] turso::Error),
}
