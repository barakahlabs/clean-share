//! Link previews on bookmarks. The store doesn't fetch anything: the app's background fetcher
//! takes bookmarks from [`Store::preview_queue`], fetches, and hands the result to
//! [`Store::set_preview`] (or marks it failed).

use super::{Store, clean_opt, row};
use crate::{Bookmark, Error, PreviewData, PreviewStatus, Result};

impl Store {
  /// Stores what a preview found and marks it done. The title is only filled in if the bookmark
  /// has none: a title the user typed (or one from the shared text) wins.
  pub async fn set_preview(&mut self, id: &str, data: &PreviewData) -> Result<Bookmark> {
    let now = self.clock.now_ms();
    let keywords: Vec<&str> =
      data.keywords.iter().map(|k| k.trim()).filter(|k| !k.is_empty() && !k.contains('\n')).collect();
    let changed = self
      .conn
      .execute(
        "UPDATE bookmark SET
           title = coalesce(nullif(title, ''), ?2),
           description = ?3, site_name = ?4, image_url = ?5, icon_url = ?6, author = ?7,
           page_kind = ?8, keywords = ?9, published = ?10, excerpt = ?11,
           embed_kind = ?12, embed_url = ?13,
           preview_status = 'done', preview_fetched_at = ?14, updated_at = ?14
         WHERE id = ?1",
        turso::params![
          id,
          clean_opt(data.title.clone()),
          clean_opt(data.description.clone()),
          clean_opt(data.site_name.clone()),
          clean_opt(data.image_url.clone()),
          clean_opt(data.icon_url.clone()),
          clean_opt(data.author.clone()),
          clean_opt(data.page_kind.clone()),
          keywords.join("\n"),
          clean_opt(data.published.clone()),
          clean_opt(data.excerpt.clone()),
          clean_opt(data.embed_kind.clone()),
          clean_opt(data.embed_url.clone()),
          now
        ],
      )
      .await?;
    if changed == 0 {
      return Err(Error::NotFound);
    }
    self.get(id).await?.ok_or(Error::NotFound)
  }

  /// Sets the preview status only (`pending` to queue a fetch, `failed` after one failed).
  /// Doesn't count as an edit, so `updated_at` stays.
  pub async fn set_preview_status(&mut self, id: &str, status: PreviewStatus) -> Result<()> {
    let changed = self
      .conn
      .execute(
        "UPDATE bookmark SET preview_status = ?2 WHERE id = ?1",
        turso::params![id, row::preview_status_str(status)],
      )
      .await?;
    if changed == 0 {
      return Err(Error::NotFound);
    }
    Ok(())
  }

  /// Records a failed fetch. A bookmark that already has a preview keeps it (status back to
  /// `done`): a retry that times out shouldn't take away a good preview. Otherwise `failed`.
  pub async fn preview_failed(&mut self, id: &str) -> Result<()> {
    let changed = self
      .conn
      .execute(
        "UPDATE bookmark SET preview_status =
           CASE WHEN preview_fetched_at IS NULL THEN 'failed' ELSE 'done' END
         WHERE id = ?1",
        [id],
      )
      .await?;
    if changed == 0 {
      return Err(Error::NotFound);
    }
    Ok(())
  }

  /// Bookmarks waiting for a preview, newest first: `(id, url)`.
  pub async fn preview_queue(&self, limit: u32) -> Result<Vec<(String, String)>> {
    let mut rows = self
      .conn
      .query(
        "SELECT id, url FROM bookmark WHERE preview_status = 'pending'
         ORDER BY created_at DESC LIMIT ?1",
        [limit as i64],
      )
      .await?;
    let mut queue = Vec::new();
    while let Some(r) = rows.next().await? {
      queue.push((row::text(&r, 0, "id")?, row::text(&r, 1, "url")?));
    }
    Ok(queue)
  }

  /// Queues every bookmark that never had a preview (e.g. when previews are switched on).
  /// Failed ones stay failed: they're retried one by one. Returns how many were queued.
  pub async fn queue_missing_previews(&mut self) -> Result<u64> {
    Ok(
      self
        .conn
        .execute("UPDATE bookmark SET preview_status = 'pending' WHERE preview_status = 'none'", ())
        .await?,
    )
  }
}
