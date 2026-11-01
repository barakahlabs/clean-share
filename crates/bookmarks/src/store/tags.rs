//! Tags. A tag's ID is derived from its lowercased name, so every device agrees on it; renaming
//! a tag therefore moves its bookmarks to the tag with the new name (merging if it exists).

use std::collections::HashMap;

use turso::{
  Connection, Value,
  transaction::{Transaction, TransactionBehavior},
};

use super::{Store, row};
use crate::{BookmarkTag, Error, Result, TagCount, TagInput, ids};

impl Store {
  /// Replaces the bookmark's tags.
  pub async fn set_tags(
    &mut self,
    bookmark_id: &str,
    tags: &[TagInput],
  ) -> Result<Vec<BookmarkTag>> {
    let now = self.clock.now_ms();
    let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).await?;
    let changed = tx
      .execute(
        "UPDATE bookmark SET updated_at = ?2 WHERE id = ?1",
        turso::params![bookmark_id, now],
      )
      .await?;
    if changed == 0 {
      return Err(Error::NotFound);
    }
    tx.execute("DELETE FROM bookmark_tag WHERE bookmark_id = ?1", [bookmark_id]).await?;
    add(&tx, bookmark_id, tags, now).await?;
    remove_unused(&tx).await?;
    tx.commit().await?;
    Ok(of(&self.conn, &[bookmark_id.to_string()]).await?.remove(bookmark_id).unwrap_or_default())
  }

  /// Every tag with how many bookmarks use it, most used first.
  pub async fn tags(&self) -> Result<Vec<TagCount>> {
    let mut rows = self
      .conn
      .query(
        "SELECT t.name, count(bt.bookmark_id) AS uses
         FROM tag t LEFT JOIN bookmark_tag bt ON bt.tag_id = t.id
         GROUP BY t.id, t.name
         ORDER BY uses DESC, t.name COLLATE NOCASE",
        (),
      )
      .await?;
    let mut tags = Vec::new();
    while let Some(r) = rows.next().await? {
      tags
        .push(TagCount { name: row::text(&r, 0, "name")?, count: row::int(&r, 1, "uses")? as u64 });
    }
    Ok(tags)
  }

  /// Renames a tag. If a tag named `to` exists, the two are merged.
  pub async fn rename_tag(&mut self, from: &str, to: &str) -> Result<()> {
    let to = normalize(to).ok_or(Error::EmptyTag)?;
    let from_id = ids::tag_id(&normalize(from).ok_or(Error::EmptyTag)?);
    let to_id = ids::tag_id(&to);
    let now = self.clock.now_ms();

    let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).await?;
    if !exists(&tx, &from_id).await? {
      return Err(Error::TagNotFound(from.to_string()));
    }
    let affected = bookmarks_with(&tx, &from_id).await?;
    if from_id == to_id {
      // Only the capitalisation changes.
      tx.execute(
        "UPDATE tag SET name = ?2 WHERE id = ?1",
        turso::params![from_id.as_str(), to.as_str()],
      )
      .await?;
    } else {
      tx.execute(
        "INSERT OR IGNORE INTO tag (id, name, created_at) VALUES (?1, ?2, ?3)",
        turso::params![to_id.as_str(), to.as_str(), now],
      )
      .await?;
      tx.execute(
        "INSERT OR IGNORE INTO bookmark_tag (bookmark_id, tag_id, source, created_at)
         SELECT bookmark_id, ?2, source, created_at FROM bookmark_tag WHERE tag_id = ?1",
        turso::params![from_id.as_str(), to_id.as_str()],
      )
      .await?;
      tx.execute("DELETE FROM tag WHERE id = ?1", [from_id.as_str()]).await?;
    }
    for id in &affected {
      refresh_text(&tx, id, now).await?;
    }
    tx.commit().await?;
    Ok(())
  }

  /// Removes a tag from every bookmark.
  pub async fn delete_tag(&mut self, name: &str) -> Result<()> {
    let id = ids::tag_id(&normalize(name).ok_or(Error::EmptyTag)?);
    let now = self.clock.now_ms();
    let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).await?;
    if !exists(&tx, &id).await? {
      return Err(Error::TagNotFound(name.to_string()));
    }
    let affected = bookmarks_with(&tx, &id).await?;
    tx.execute("DELETE FROM tag WHERE id = ?1", [id.as_str()]).await?;
    for bookmark_id in &affected {
      refresh_text(&tx, bookmark_id, now).await?;
    }
    tx.commit().await?;
    Ok(())
  }
}

/// `"  #Rust   lang "` → `"Rust lang"`. `None` if nothing is left.
pub fn normalize(name: &str) -> Option<String> {
  let name = name.trim().trim_start_matches('#');
  let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
  (!name.is_empty()).then_some(name)
}

/// Adds tags to a bookmark, keeping the ones it has. Run inside a transaction.
pub(super) async fn add(
  conn: &Connection,
  bookmark_id: &str,
  tags: &[TagInput],
  now: i64,
) -> Result<()> {
  for tag in tags {
    let Some(name) = normalize(&tag.name) else { continue };
    let tag_id = ids::tag_id(&name);
    conn
      .execute(
        "INSERT OR IGNORE INTO tag (id, name, created_at) VALUES (?1, ?2, ?3)",
        turso::params![tag_id.as_str(), name.as_str(), now],
      )
      .await?;
    conn
      .execute(
        "INSERT OR IGNORE INTO bookmark_tag (bookmark_id, tag_id, source, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        turso::params![bookmark_id, tag_id.as_str(), row::tag_source_str(tag.source), now],
      )
      .await?;
  }
  // Updates `tags_text` without touching `updated_at` (the caller sets it).
  conn
    .execute(
      "UPDATE bookmark SET tags_text = coalesce(
         (SELECT group_concat(name, ' ') FROM
           (SELECT t.name FROM bookmark_tag bt JOIN tag t ON t.id = bt.tag_id
            WHERE bt.bookmark_id = ?1 ORDER BY bt.created_at, t.name)), '')
       WHERE id = ?1",
      [bookmark_id],
    )
    .await?;
  Ok(())
}

/// The tags of each given bookmark, in the order they were added.
pub(super) async fn of(
  conn: &Connection,
  bookmark_ids: &[String],
) -> Result<HashMap<String, Vec<BookmarkTag>>> {
  let mut out: HashMap<String, Vec<BookmarkTag>> = HashMap::new();
  if bookmark_ids.is_empty() {
    return Ok(out);
  }
  let placeholders = vec!["?"; bookmark_ids.len()].join(", ");
  let sql = format!(
    "SELECT bt.bookmark_id, t.name, bt.source FROM bookmark_tag bt JOIN tag t ON t.id = bt.tag_id
     WHERE bt.bookmark_id IN ({placeholders}) ORDER BY bt.created_at, t.name"
  );
  let params: Vec<Value> = bookmark_ids.iter().map(|id| Value::from(id.as_str())).collect();
  let mut rows = conn.query(&sql, params).await?;
  while let Some(r) = rows.next().await? {
    out.entry(row::text(&r, 0, "bookmark_id")?).or_default().push(BookmarkTag {
      name: row::text(&r, 1, "name")?,
      source: row::tag_source(&row::text(&r, 2, "source")?),
    });
  }
  Ok(out)
}

async fn refresh_text(conn: &Connection, bookmark_id: &str, now: i64) -> Result<()> {
  add(conn, bookmark_id, &[], now).await?;
  conn
    .execute("UPDATE bookmark SET updated_at = ?2 WHERE id = ?1", turso::params![bookmark_id, now])
    .await?;
  Ok(())
}

async fn exists(conn: &Connection, tag_id: &str) -> Result<bool> {
  Ok(conn.query("SELECT 1 FROM tag WHERE id = ?1", [tag_id]).await?.next().await?.is_some())
}

async fn bookmarks_with(conn: &Connection, tag_id: &str) -> Result<Vec<String>> {
  let mut rows =
    conn.query("SELECT bookmark_id FROM bookmark_tag WHERE tag_id = ?1", [tag_id]).await?;
  let mut ids = Vec::new();
  while let Some(r) = rows.next().await? {
    ids.push(row::text(&r, 0, "bookmark_id")?);
  }
  Ok(ids)
}

/// Drops tags no bookmark uses any more, so autocomplete doesn't offer them.
pub(super) async fn remove_unused(conn: &Connection) -> Result<()> {
  conn.execute("DELETE FROM tag WHERE id NOT IN (SELECT tag_id FROM bookmark_tag)", ()).await?;
  Ok(())
}
