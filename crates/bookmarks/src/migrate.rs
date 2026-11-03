//! Schema migrations, tracked in `PRAGMA user_version`.
//!
//! Each migration is a SQL file in `migrations/`, compiled into the app, so phones need no files
//! at runtime. Each phone migrates its own database when it opens it, so there's never a server
//! job that has to migrate every user's database.
//!
//! To change the schema, add the next numbered file and list it in [`MIGRATIONS`]. Never edit a
//! released migration: phones that already ran it won't run it again.

use turso::{
  Connection,
  transaction::{Transaction, TransactionBehavior},
};

use crate::{Error, Result};

/// In order: migration `i` takes the schema from version `i` to `i + 1`.
const MIGRATIONS: &[(&str, &str)] = &[
  ("0001_init", include_str!("../migrations/0001_init.sql")),
  ("0002_previews", include_str!("../migrations/0002_previews.sql")),
];

/// The schema version this build creates and understands.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

pub async fn migrate(conn: &Connection) -> Result<()> {
  let mut version = user_version(conn).await?;
  if version > SCHEMA_VERSION {
    return Err(Error::NewerSchema { found: version, supported: SCHEMA_VERSION });
  }
  while version < SCHEMA_VERSION {
    let (_, sql) = MIGRATIONS[version as usize];
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate).await?;
    tx.execute_batch(sql).await?;
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

#[cfg(test)]
mod tests {
  use super::*;

  /// Every file in migrations/ is listed, in order, and numbered 0001, 0002, … without gaps.
  /// Catches a new file that was never added to MIGRATIONS (it would silently never run).
  #[test]
  fn migrations_match_the_files_on_disk() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut files: Vec<String> = std::fs::read_dir(dir)
      .unwrap()
      .map(|entry| entry.unwrap().file_name().into_string().unwrap())
      .filter(|name| name.ends_with(".sql"))
      .map(|name| name.trim_end_matches(".sql").to_string())
      .collect();
    files.sort();

    let listed: Vec<&str> = MIGRATIONS.iter().map(|(name, _)| *name).collect();
    assert_eq!(files, listed, "migrations/ and MIGRATIONS differ");
    for (i, name) in listed.iter().enumerate() {
      assert!(
        name.starts_with(&format!("{:04}_", i + 1)),
        "{name} should be numbered {:04}",
        i + 1
      );
    }
  }
}
