//! All reads and writes. Works on one connection to one user's database.

mod list;
mod row;
mod suggest;
mod tags;

use std::sync::Arc;

use turso::{
  Connection, Value,
  transaction::{Transaction, TransactionBehavior},
};

use crate::{
  Bookmark, BookmarkPatch, BundledRules, Cleaner, Clock, Error, NewBookmark, Prepared, Result,
  Saved, SystemClock,
  db::{self, Location},
  ids, migrate,
};

pub struct Store {
  conn: Connection,
  cleaner: Arc<dyn Cleaner>,
  clock: Arc<dyn Clock>,
}

impl Store {
  /// Wraps an open connection (from [`db::open`], or later a synced database) and migrates it.
  pub async fn new(
    conn: Connection,
    cleaner: Arc<dyn Cleaner>,
    clock: Arc<dyn Clock>,
  ) -> Result<Self> {
    conn.execute_batch("PRAGMA foreign_keys = ON").await?;
    migrate::migrate(&conn).await?;
    Ok(Self { conn, cleaner, clock })
  }

  /// Opens a local database with the bundled rules and the system clock.
  pub async fn open(location: &Location) -> Result<Self> {
    Self::new(db::open(location).await?, Arc::new(BundledRules), Arc::new(SystemClock)).await
  }

  pub async fn open_in_memory() -> Result<Self> {
    Self::open(&Location::Memory).await
  }

  /// Rules now in use; e.g. after a newer rules download.
  pub fn set_cleaner(&mut self, cleaner: Arc<dyn Cleaner>) {
    self.cleaner = cleaner;
  }

  /// Cleans the links in `text`, finds whether the first one is already saved, and suggests a
  /// title from the text around it.
  pub async fn prepare(&self, text: &str) -> Result<Prepared> {
    let links = self.cleaner.links(text);
    let (existing, title) = match links.first() {
      Some(link) => (self.find_by_url(&link.cleaned).await?, title_from(text, &links)),
      None => (None, None),
    };
    Ok(Prepared { links, existing, title })
  }

  /// Saves the first link in `new.shared_text`. If that link (after cleaning) is already saved,
  /// merges into the existing bookmark instead: adds the new tags, and fills title and note only
  /// where they were empty.
  pub async fn save(&mut self, new: NewBookmark) -> Result<Saved> {
    let links = self.cleaner.links(&new.shared_text);
    let link = links.first().cloned().ok_or(Error::NoLink)?;
    let now = self.clock.now_ms();
    let title = clean_opt(new.title).or_else(|| title_from(&new.shared_text, &links));
    let note = clean_opt(new.note);
    let existing = self.find_by_url(&link.cleaned).await?;

    let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).await?;
    let id = match &existing {
      Some(old) => {
        tx.execute(
          "UPDATE bookmark SET
             title = coalesce(nullif(title, ''), ?2),
             note = coalesce(nullif(note, ''), ?3),
             updated_at = ?4
           WHERE id = ?1",
          turso::params![old.id.as_str(), title, note, now],
        )
        .await?;
        old.id.clone()
      }
      None => {
        let id = ids::bookmark_id(&link.cleaned);
        tx.execute(
          "INSERT INTO bookmark
             (id, url, original_url, title, note, shared_text, provider, rules_revision,
              created_at, updated_at)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
          turso::params![
            id.as_str(),
            link.cleaned.as_str(),
            link.original.as_str(),
            title,
            note,
            new.shared_text.as_str(),
            link.provider,
            self.cleaner.revision(),
            now
          ],
        )
        .await?;
        id
      }
    };
    tags::add(&tx, &id, &new.tags, now).await?;
    tx.commit().await?;

    let bookmark = self.get(&id).await?.ok_or(Error::NotFound)?;
    Ok(Saved { bookmark, created: existing.is_none() })
  }

  pub async fn get(&self, id: &str) -> Result<Option<Bookmark>> {
    self.one("WHERE id = ?1", Value::from(id)).await
  }

  /// `url` must already be cleaned.
  pub async fn find_by_url(&self, url: &str) -> Result<Option<Bookmark>> {
    self.one("WHERE url = ?1", Value::from(url)).await
  }

  pub async fn update(&mut self, id: &str, patch: BookmarkPatch) -> Result<Bookmark> {
    let set_title = patch.title.is_some();
    let set_note = patch.note.is_some();
    let changed = self
      .conn
      .execute(
        "UPDATE bookmark SET
           title = CASE WHEN ?2 THEN ?3 ELSE title END,
           note = CASE WHEN ?4 THEN ?5 ELSE note END,
           updated_at = ?6
         WHERE id = ?1",
        turso::params![
          id,
          set_title,
          clean_opt(patch.title),
          set_note,
          clean_opt(patch.note),
          self.clock.now_ms()
        ],
      )
      .await?;
    if changed == 0 {
      return Err(Error::NotFound);
    }
    self.get(id).await?.ok_or(Error::NotFound)
  }

  /// Deletes a bookmark. Its tags stay, even if no other bookmark uses them.
  pub async fn delete(&mut self, id: &str) -> Result<()> {
    if self.conn.execute("DELETE FROM bookmark WHERE id = ?1", [id]).await? == 0 {
      return Err(Error::NotFound);
    }
    Ok(())
  }

  /// Compacts the search index. Turso never does this on its own: edits and deletes leave old
  /// entries behind, which slowly makes searches slower and the file bigger. The app runs this
  /// now and then (e.g. at most weekly on launch) and after bulk imports.
  pub async fn optimize(&mut self) -> Result<()> {
    self.conn.execute_batch("OPTIMIZE INDEX bookmark_search").await?;
    Ok(())
  }

  async fn one(&self, filter: &str, param: Value) -> Result<Option<Bookmark>> {
    let sql = format!("SELECT {} FROM bookmark {filter}", row::BOOKMARK_COLUMNS);
    let mut rows = self.conn.query(&sql, [param]).await?;
    let Some(found) = rows.next().await? else { return Ok(None) };
    let mut bookmark = row::bookmark(&found)?;
    drop(rows);
    bookmark.tags =
      tags::of(&self.conn, &[bookmark.id.clone()]).await?.remove(&bookmark.id).unwrap_or_default();
    Ok(Some(bookmark))
  }
}

/// Trimmed, with empty meaning none.
fn clean_opt(text: Option<String>) -> Option<String> {
  text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

/// The shared text without its links, if anything is left ("Great talk https://…" → "Great talk").
fn title_from(shared_text: &str, links: &[link_cleaner::Link]) -> Option<String> {
  let rest =
    links.iter().fold(shared_text.to_string(), |text, link| text.replacen(&link.original, " ", 1));
  let title = rest.split_whitespace().collect::<Vec<_>>().join(" ");
  let title = title.trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '–' | '|' | ':'));
  (!title.is_empty()).then(|| title.to_string())
}

#[cfg(test)]
mod tests;
