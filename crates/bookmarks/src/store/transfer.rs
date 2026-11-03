//! Export and import: the database side of [`crate::transfer`].

use serde::Serialize;
use turso::Value;

use super::{Store, row, tags};
use crate::{
  Bookmark, NewBookmark, Result,
  transfer::{self, Format, ImportItem},
};

/// How an import went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
  pub format: Format,
  /// New bookmarks.
  pub added: u32,
  /// Already saved: tags added, title and note filled where they were empty.
  pub merged: u32,
  /// Not a link the app can clean and save.
  pub skipped: u32,
}

/// Bookmarks read per query when exporting (keeps the tag lookup's `IN (…)` list small).
const CHUNK: usize = 500;

impl Store {
  /// Imports an export file (see [`crate::transfer`] for the formats). Every link is cleaned;
  /// a link that's already saved is merged into its bookmark; original dates are kept.
  pub async fn import(&mut self, file: &str) -> std::result::Result<ImportReport, ImportError> {
    let (format, items) = transfer::parse(file)?;
    let mut report = ImportReport { format, added: 0, merged: 0, skipped: 0 };
    for item in items {
      match self.import_one(item).await? {
        Some(true) => report.added += 1,
        Some(false) => report.merged += 1,
        None => report.skipped += 1,
      }
    }
    if report.added + report.merged > 0 {
      self.optimize().await?;
    }
    Ok(report)
  }

  /// `Some(created)`, or `None` if it has no link to save.
  async fn import_one(&mut self, item: ImportItem) -> Result<Option<bool>> {
    if self.cleaner.links(&item.url).is_empty() {
      return Ok(None);
    }
    let saved = self
      .save(NewBookmark {
        shared_text: item.url,
        title: item.title,
        note: item.note,
        tags: item.tags,
        created_at: item.created_at,
      })
      .await?;
    Ok(Some(saved.created))
  }

  /// Every bookmark as this app's JSON backup.
  pub async fn export_json(&self) -> Result<String> {
    Ok(transfer::json::export(&self.all().await?, self.clock.now_ms()))
  }

  /// Every bookmark as Netscape bookmark HTML, for browsers and other apps.
  pub async fn export_html(&self) -> Result<String> {
    Ok(transfer::netscape::export(&self.all().await?))
  }

  /// Every bookmark with its tags, newest first.
  async fn all(&self) -> Result<Vec<Bookmark>> {
    let sql = format!(
      "SELECT {} FROM bookmark ORDER BY created_at DESC, id DESC",
      row::BOOKMARK_COLUMNS
    );
    let mut rows = self.conn.query(&sql, Vec::<Value>::new()).await?;
    let mut all = Vec::new();
    while let Some(r) = rows.next().await? {
      all.push(row::bookmark(&r)?);
    }
    drop(rows);
    for chunk in all.chunks_mut(CHUNK) {
      let ids: Vec<String> = chunk.iter().map(|b| b.id.clone()).collect();
      let mut found = tags::of(&self.conn, &ids).await?;
      for b in chunk {
        b.tags = found.remove(&b.id).unwrap_or_default();
      }
    }
    Ok(all)
  }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
  #[error(transparent)]
  Parse(#[from] transfer::ParseError),
  #[error(transparent)]
  Store(#[from] crate::Error),
}
