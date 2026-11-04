//! Re-cleaning saved links when the rules change: every bookmark keeps the link as it was shared
//! (`original_url`), so newer rules can clean it again. A link that becomes another bookmark's
//! link is merged into it.

use serde::Serialize;
use turso::{
  Value,
  transaction::{Transaction, TransactionBehavior},
};

use super::{Store, row, tags};
use crate::Result;

/// What a re-clean did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecleanReport {
  /// Bookmarks looked at.
  pub checked: u32,
  /// Whose link changed.
  pub changed: u32,
  /// Of those, how many turned out to be a link already saved, and were merged into it.
  pub merged: u32,
  /// Bookmarks removed by merging (their thumbnails can go).
  pub removed: Vec<String>,
}

struct Saved {
  id: String,
  url: String,
  original_url: String,
}

impl Store {
  /// Cleans saved links again with the current rules: those saved with other rules, or `all`.
  /// A link that changes keeps its bookmark (and ID); if it becomes the link of another bookmark,
  /// the two are merged: the older one stays, with the tags of both, and the title and note of
  /// either where it had none.
  pub async fn reclean(&mut self, all: bool) -> Result<RecleanReport> {
    let revision = self.cleaner.revision().to_string();
    let filter = if all { "" } else { "WHERE rules_revision != ?1" };
    let sql = format!("SELECT id, url, original_url FROM bookmark {filter} ORDER BY created_at, id");
    let params: Vec<Value> = if all { Vec::new() } else { vec![Value::from(revision.as_str())] };
    let mut rows = self.conn.query(&sql, params).await?;
    let mut saved = Vec::new();
    while let Some(r) = rows.next().await? {
      saved.push(Saved {
        id: row::text(&r, 0, "id")?,
        url: row::text(&r, 1, "url")?,
        original_url: row::text(&r, 2, "original_url")?,
      });
    }
    drop(rows);

    let mut report = RecleanReport { checked: saved.len() as u32, ..Default::default() };
    for bookmark in saved {
      // Already merged away earlier in this run.
      if report.removed.contains(&bookmark.id) {
        continue;
      }
      let link = self.cleaner.links(&bookmark.original_url).into_iter().next();
      let Some(link) = link.filter(|l| l.cleaned != bookmark.url) else {
        self.set_revision(&bookmark.id, &revision).await?;
        continue;
      };
      report.changed += 1;
      match self.find_by_url(&link.cleaned).await? {
        Some(other) => {
          // The older bookmark stays.
          let (keep, drop) = if other.created_at <= self.created_at(&bookmark.id).await? {
            (other.id, bookmark.id.clone())
          } else {
            (bookmark.id.clone(), other.id)
          };
          self.merge(&keep, &drop, &link.cleaned, link.provider.as_deref(), &revision).await?;
          report.merged += 1;
          report.removed.push(drop);
        }
        None => {
          self
            .conn
            .execute(
              "UPDATE bookmark SET url = ?2, provider = ?3, rules_revision = ?4 WHERE id = ?1",
              turso::params![bookmark.id.as_str(), link.cleaned.as_str(), link.provider, revision.as_str()],
            )
            .await?;
        }
      }
    }
    Ok(report)
  }

  async fn set_revision(&mut self, id: &str, revision: &str) -> Result<()> {
    self
      .conn
      .execute("UPDATE bookmark SET rules_revision = ?2 WHERE id = ?1", turso::params![id, revision])
      .await?;
    Ok(())
  }

  async fn created_at(&self, id: &str) -> Result<i64> {
    let mut rows = self.conn.query("SELECT created_at FROM bookmark WHERE id = ?1", [id]).await?;
    match rows.next().await? {
      Some(r) => row::int(&r, 0, "created_at"),
      None => Err(crate::Error::NotFound),
    }
  }

  /// Merges `drop` into `keep`, which gets `url`: tags of both, empty title and note filled from
  /// `drop`, the earlier saved date.
  async fn merge(&mut self, keep: &str, drop: &str, url: &str, provider: Option<&str>, revision: &str) -> Result<()> {
    let now = self.clock.now_ms();
    let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).await?;
    tx.execute(
      "INSERT OR IGNORE INTO bookmark_tag (bookmark_id, tag_id, source, created_at)
       SELECT ?1, tag_id, source, created_at FROM bookmark_tag WHERE bookmark_id = ?2",
      turso::params![keep, drop],
    )
    .await?;
    tx.execute(
      "UPDATE bookmark SET
         title = coalesce(nullif(title, ''), (SELECT title FROM bookmark WHERE id = ?2)),
         note = coalesce(nullif(note, ''), (SELECT note FROM bookmark WHERE id = ?2)),
         created_at = min(created_at, (SELECT created_at FROM bookmark WHERE id = ?2))
       WHERE id = ?1",
      turso::params![keep, drop],
    )
    .await?;
    // Remove the other one first: `url` is unique.
    tx.execute("DELETE FROM bookmark WHERE id = ?1", [drop]).await?;
    tx.execute(
      "UPDATE bookmark SET url = ?2, provider = ?3, rules_revision = ?4, updated_at = ?5 WHERE id = ?1",
      turso::params![keep, url, provider, revision, now],
    )
    .await?;
    tags::add(&tx, keep, &[], now).await?;
    tx.commit().await?;
    Ok(())
  }
}
