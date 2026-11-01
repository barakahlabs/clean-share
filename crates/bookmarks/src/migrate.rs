//! Schema migrations, tracked in `PRAGMA user_version`.
//!
//! Each phone migrates its own database when it opens it, so there's never a server job that
//! has to migrate every user's database. Never edit a released migration: add a new one.

use turso::{
  Connection,
  transaction::{Transaction, TransactionBehavior},
};

use crate::{Error, Result};

/// Migration `i` takes the schema from version `i` to `i + 1`.
const MIGRATIONS: &[&str] = &[V1];

/// The schema version this build creates and understands.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// Bookmarks, tags, and full-text search.
///
/// - IDs are text UUIDs derived from content (see `ids.rs`); sync replays rows by primary key.
/// - Thumbnails are not stored here: they're a per-device cache named after the bookmark ID.
/// - Times are milliseconds since the Unix epoch.
const V1: &str = "
CREATE TABLE bookmark (
  id                 TEXT PRIMARY KEY NOT NULL,
  url                TEXT NOT NULL UNIQUE,
  original_url       TEXT NOT NULL,
  title              TEXT,
  note               TEXT,
  shared_text        TEXT NOT NULL,
  provider           TEXT,
  site_name          TEXT,
  description        TEXT,
  image_url          TEXT,
  embed_kind         TEXT,
  embed_url          TEXT,
  preview_status     TEXT NOT NULL DEFAULT 'none',
  preview_fetched_at INTEGER,
  rules_revision     TEXT NOT NULL,
  tags_text          TEXT NOT NULL DEFAULT '',
  created_at         INTEGER NOT NULL,
  updated_at         INTEGER NOT NULL
) STRICT;

CREATE INDEX bookmark_newest ON bookmark (created_at DESC, id DESC);

CREATE TABLE tag (
  id         TEXT PRIMARY KEY NOT NULL,
  name       TEXT NOT NULL UNIQUE COLLATE NOCASE,
  created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE bookmark_tag (
  bookmark_id TEXT NOT NULL REFERENCES bookmark (id) ON DELETE CASCADE,
  tag_id      TEXT NOT NULL REFERENCES tag (id) ON DELETE CASCADE,
  source      TEXT NOT NULL,
  created_at  INTEGER NOT NULL,
  PRIMARY KEY (bookmark_id, tag_id)
) STRICT;

CREATE INDEX bookmark_tag_by_tag ON bookmark_tag (tag_id);

-- ngram tokens give search-as-you-type ('ru' finds 'Rust'); Turso's prefix queries ('ru*')
-- don't match yet. tags_text is the bookmark's tag names joined by spaces.
CREATE INDEX bookmark_search ON bookmark
  USING fts (title, note, shared_text, description, tags_text)
  WITH (tokenizer = 'ngram');
";

pub async fn migrate(conn: &Connection) -> Result<()> {
  let mut version = user_version(conn).await?;
  if version > SCHEMA_VERSION {
    return Err(Error::NewerSchema { found: version, supported: SCHEMA_VERSION });
  }
  while version < SCHEMA_VERSION {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate).await?;
    tx.execute_batch(MIGRATIONS[version as usize]).await?;
    version += 1;
    tx.execute_batch(&format!("PRAGMA user_version = {version}")).await?;
    tx.commit().await?;
  }
  Ok(())
}

pub async fn user_version(conn: &Connection) -> Result<i64> {
  let row = conn.query("PRAGMA user_version", ()).await?.next().await?;
  let value = row.map(|r| r.get_value(0)).transpose()?;
  Ok(value.and_then(|v| v.as_integer().copied()).unwrap_or(0))
}
